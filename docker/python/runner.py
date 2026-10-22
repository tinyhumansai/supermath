"""One sandboxed run: imports a project module and calls one function.

`server.py` starts this file in a fresh interpreter for every request, as
`python -s -B -P runner.py <request.json>`, with the run's private temporary
directory as its working directory. The request file holds what to run and
the limits to apply; the child writes `result.json` there and exits. Its
stdout and stderr are files the server opened, so output survives a kill.

Request fields
    mode        "run" or "search"
    project_dir the project's `python/` directory (read-only to the run)
    module      dotted module name, e.g. "work.sunflower_search"
    entry       function to call (mode "run")
    args        keyword arguments, a JSON object (mode "run")
    seed        integer seed for `random`, `numpy`, and a `seed` parameter
    predicate   function of one argument returning bool (mode "search")
    frame       the search frame (mode "search")
    max_frame   admission cap on the number of values (mode "search")
    limits      {cpu_s, memory_mb, output_bytes, open_files}

Result `status` values: "returned", "raised", "no_entry", "bad_args",
"bad_request", "limit_memory", "limit_output", and, in search mode, "found",
"exhausted", "predicate_raised", "capped".
"""

import inspect
import itertools
import json
import os
import random
import resource
import signal
import sys
import time
import traceback

MAX_SAFE_INTEGER = 2**53
MAX_VALUE_BYTES = 4 * 1024 * 1024
TRACEBACK_FRAMES = 10
PROGRESS_INTERVAL_S = 0.25
HERE = os.path.abspath(__file__)


class OutputLimit(Exception):
    """The run produced more than the service allows."""


class NotJson(Exception):
    """The returned value cannot be represented as JSON."""


class BadRequest(Exception):
    """The request names something that does not exist or does not fit."""

    def __init__(self, status, message):
        super().__init__(message)
        self.status = status


def apply_limits(limits):
    """Applies resource limits; a limit the platform refuses is skipped."""
    # Python ignores SIGXFSZ, turning an over-size write into an OSError the
    # program could swallow; restoring the default kills the run instead.
    signal.signal(signal.SIGXFSZ, signal.SIG_DFL)
    wanted = [
        (resource.RLIMIT_CPU, limits.get("cpu_s"), 5),
        (resource.RLIMIT_AS, mb_to_bytes(limits.get("memory_mb")), 0),
        (resource.RLIMIT_FSIZE, limits.get("output_bytes"), 0),
        (resource.RLIMIT_NOFILE, limits.get("open_files"), 0),
        (resource.RLIMIT_CORE, 0, 0),
    ]
    for which, soft, slack in wanted:
        if soft is None:
            continue
        try:
            resource.setrlimit(which, (soft, soft + slack))
        except (ValueError, OSError):
            pass


def mb_to_bytes(megabytes):
    return None if not megabytes else int(megabytes) * 1024 * 1024


def block_network():
    """Refuses IP sockets from inside the interpreter.

    The compose network is the real boundary (`internal: true`, no egress);
    this makes the refusal immediate and legible, and holds when the service
    runs somewhere that is not isolated.
    """
    import socket

    def refuse(*_args, **_kwargs):
        raise PermissionError("network access is disabled in this sandbox")

    def guarded(original):
        def wrapper(self, *args, **kwargs):
            if self.family in (socket.AF_INET, socket.AF_INET6):
                refuse()
            return original(self, *args, **kwargs)

        return wrapper

    for name in ("connect", "connect_ex", "sendto", "bind"):
        setattr(socket.socket, name, guarded(getattr(socket.socket, name)))
    for name in ("getaddrinfo", "gethostbyname", "gethostbyname_ex", "create_connection"):
        setattr(socket, name, refuse)


def seed_everything(seed):
    random.seed(seed)
    numpy = sys.modules.get("numpy")
    if numpy is not None:
        numpy.random.seed(seed % 2**32)


def to_json(value, where="$"):
    """The JSON form of a returned value; integers past 2**53 become strings."""
    if value is None or isinstance(value, (bool, str)):
        return value
    if isinstance(value, int):
        return str(value) if abs(value) > MAX_SAFE_INTEGER else value
    if isinstance(value, float):
        if value != value or value in (float("inf"), float("-inf")):
            raise NotJson(f"{where} is a non-finite float, which JSON cannot hold")
        return value
    if isinstance(value, (list, tuple)):
        return [to_json(item, f"{where}[{i}]") for i, item in enumerate(value)]
    if isinstance(value, dict):
        out = {}
        for key, item in value.items():
            if not isinstance(key, str):
                raise NotJson(f"{where} has a {type(key).__name__} key; keys must be strings")
            out[key] = to_json(item, f"{where}.{key}")
        return out
    numpy = sys.modules.get("numpy")
    if numpy is not None and isinstance(value, numpy.generic):
        return to_json(value.item(), where)
    raise NotJson(
        f"{where} is a {type(value).__name__}, which is not JSON; "
        "return None, bool, int, float, str, list, or dict (convert with int(), float(), or str())"
    )


def exact_json(value):
    """Lossless JSON text (big integers stay numbers) for a witness file."""
    return json.dumps(value, allow_nan=False, default=_tuple_to_list, separators=(",", ":"))


def _tuple_to_list(value):
    numpy = sys.modules.get("numpy")
    if numpy is not None and isinstance(value, numpy.generic):
        return value.item()
    raise TypeError(f"{type(value).__name__} is not JSON serializable")


def describe_exception(error, project_dir):
    frames = [f for f in traceback.extract_tb(error.__traceback__) if f.filename != HERE]
    lines = []
    for frame in frames[-TRACEBACK_FRAMES:]:
        name = frame.filename
        if name.startswith(project_dir):
            name = os.path.relpath(name, project_dir)
        code = f": {frame.line}" if frame.line else ""
        lines.append(f"{name}:{frame.lineno} in {frame.name}{code}")
    return {
        "type": type(error).__name__,
        "message": str(error)[:2000],
        "traceback": lines,
    }


def load_function(module, name, what):
    function = getattr(module, name, None)
    if function is None or not callable(function):
        raise BadRequest("no_entry", f"{module.__name__} has no function named {name!r} ({what})")
    return function


def call_arguments(function, args, seed):
    """Keyword arguments, plus `seed` when the function declares it."""
    kwargs = dict(args)
    try:
        signature = inspect.signature(function)
    except (TypeError, ValueError):
        return kwargs
    if "seed" in signature.parameters and "seed" not in kwargs:
        kwargs["seed"] = seed
    try:
        signature.bind(**kwargs)
    except TypeError as error:
        raise BadRequest("bad_args", f"arguments do not fit the function: {error}") from None
    return kwargs


def run_mode(request, module):
    function = load_function(module, request["entry"], "entry")
    kwargs = call_arguments(function, request.get("args") or {}, request["seed"])
    seed_everything(request["seed"])
    value = function(**kwargs)
    shaped = to_json(value)
    if len(json.dumps(shaped)) > MAX_VALUE_BYTES:
        raise OutputLimit("the returned value is over the size cap")
    return {"status": "returned", "value": shaped}


def frame_values(frame, module, seed):
    """An iterator over the frame and its size (None when not known)."""
    kind = frame.get("kind")
    if kind == "range":
        low, high = int(frame["from"]), int(frame["to"])
        return iter(range(low, high + 1)), max(0, high - low + 1)
    if kind == "product":
        ranges = [range(int(r["from"]), int(r["to"]) + 1) for r in frame["ranges"]]
        size = 1
        for r in ranges:
            size *= len(r)
        return itertools.product(*ranges), size
    if kind == "generator":
        generator = load_function(module, frame["entry"], "generator")
        kwargs = call_arguments(generator, {}, seed)
        return iter(generator(**kwargs)), None
    raise BadRequest("bad_request", f"unknown frame kind {kind!r}")


def write_json(path, value):
    temporary = path + ".part"
    with open(temporary, "w", encoding="utf-8") as handle:
        json.dump(value, handle)
    os.replace(temporary, path)


def search_mode(request, module, progress_path):
    predicate = load_function(module, request["predicate"], "predicate")
    seed_everything(request["seed"])
    values, size = frame_values(request["frame"], module, request["seed"])
    cap = request["max_frame"]
    checked = 0
    last_write = time.monotonic()
    last_value = None
    for value in values:
        if checked >= cap:
            return {"status": "capped", "checked": checked, "frame_size": size}
        try:
            verdict = predicate(value)
        except MemoryError:
            raise
        except Exception as error:
            return {
                "status": "predicate_raised",
                "checked": checked,
                "frame_size": size,
                "input": to_json(value),
                "exception": describe_exception(error, request["project_dir"]),
            }
        if verdict is not True and verdict is not False:
            return {
                "status": "predicate_raised",
                "checked": checked,
                "frame_size": size,
                "input": to_json(value),
                "exception": {
                    "type": "NotBool",
                    "message": f"the predicate returned {type(verdict).__name__}, not a bool",
                    "traceback": [],
                },
            }
        if verdict is False:
            return {
                "status": "found",
                "checked": checked,
                "frame_size": size,
                "witness": to_json(value),
                "witness_json": exact_json(value),
            }
        checked += 1
        last_value = value
        if checked % 1024 == 0 and time.monotonic() - last_write >= PROGRESS_INTERVAL_S:
            write_json(progress_path, {"checked": checked})
            last_write = time.monotonic()
    write_json(progress_path, {"checked": checked})
    return {"status": "exhausted", "checked": checked, "frame_size": size, "last": to_json(last_value)}


def peak_rss_mb():
    peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    divisor = 1024 * 1024 if sys.platform == "darwin" else 1024
    return max(1, int(peak / divisor))


def execute(request):
    project_dir = request["project_dir"]
    sys.path.insert(0, project_dir)
    seed_everything(request["seed"])
    try:
        module = __import__(request["module"], fromlist=["_"])
    except MemoryError:
        raise
    except Exception as error:
        return {"status": "raised", "exception": describe_exception(error, project_dir)}
    progress = os.path.join(os.getcwd(), "progress.json")
    try:
        if request["mode"] == "search":
            return search_mode(request, module, progress)
        return run_mode(request, module)
    except BadRequest:
        raise
    except NotJson as error:
        return {
            "status": "raised",
            "exception": {"type": "NotJsonSerializable", "message": str(error), "traceback": []},
        }
    except MemoryError:
        raise
    except Exception as error:
        return {"status": "raised", "exception": describe_exception(error, project_dir)}


def main():
    with open(sys.argv[1], encoding="utf-8") as handle:
        request = json.load(handle)
    apply_limits(request.get("limits", {}))
    block_network()
    try:
        result = execute(request)
    except BadRequest as error:
        result = {"status": error.status, "message": str(error)}
    except MemoryError:
        result = {"status": "limit_memory"}
    except OutputLimit as error:
        result = {"status": "limit_output", "message": str(error)}
    except SystemExit as error:
        result = {
            "status": "raised",
            "exception": {"type": "SystemExit", "message": str(error.code), "traceback": []},
        }
    result["peak_rss_mb"] = peak_rss_mb()
    write_json(os.path.join(os.getcwd(), "result.json"), result)
    sys.stdout.flush()
    sys.stderr.flush()
    os._exit(0)


if __name__ == "__main__":
    main()
