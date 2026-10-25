"""The supermath Python run service: a small stdlib HTTP server.

Each request starts a fresh interpreter (`runner.py`) with resource limits,
a private temporary directory, and a wall-clock kill, and returns what the
run produced. Nothing carries between runs. The API is specified in
docs/specs/toolkit/python/README.md ("HTTP API"):

    GET  /health   the Python version and the lock's digest
    GET  /solvers the solver set, read from solvers.toml and uv.lock
    POST /run      call one function of a project file
    POST /search   search a frame for a predicate's counterexample

Configuration (environment)
    PY_SERVICE_HOST / PY_SERVICE_PORT   bind address (0.0.0.0:3160)
    SUPERMATH_PROJECTS                  projects root, read-only (/projects)
    PY_MAX_TIMEOUT_S                    longest run a request may ask (900)
    PY_MEMORY_MB                        address-space limit per run (4096)
    PY_OUTPUT_MB                        largest file a run may write (16)
    PY_OPEN_FILES                       open-file limit per run (256)
    PY_MAX_CONCURRENT                   simultaneous runs (2)
    PY_LOCK_PATH / PY_SOLVERS_PATH      uv.lock and solvers.toml locations
"""

import hashlib
import json
import os
import platform
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import tomllib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

HERE = os.path.dirname(os.path.abspath(__file__))
RUNNER = os.path.join(HERE, "runner.py")

MAX_BODY_BYTES = 256 * 1024
MAX_ARGS_BYTES = 64 * 1024
OUTPUT_CHARS = 8000
SLUG = re.compile(r"^[a-z0-9-]+$")
PATH = re.compile(r"^python/work/[a-z][a-z0-9_]*(/[a-z][a-z0-9_]*)*\.py$")
IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


class HttpError(Exception):
    """An answer other than 200, in the service's error shape."""

    def __init__(self, status, code, message):
        super().__init__(message)
        self.status = status
        self.code = code
        self.message = message


def env_int(environ, name, default):
    try:
        return int(environ.get(name, default))
    except ValueError:
        return int(default)


class Config:
    """Settings, read once from the environment."""

    def __init__(self, environ=None):
        environ = os.environ if environ is None else environ
        self.host = environ.get("PY_SERVICE_HOST", "0.0.0.0")
        self.port = env_int(environ, "PY_SERVICE_PORT", 3160)
        self.projects = environ.get("SUPERMATH_PROJECTS", "/projects")
        # Directories a request's `root` may name, colon-separated: where
        # projects off the volume live, such as rsi-ralph candidate checkouts.
        self.roots = [
            os.path.realpath(p) for p in environ.get("SUPERMATH_ROOTS", "").split(":") if os.path.isabs(p)
        ]
        self.max_timeout_s = env_int(environ, "PY_MAX_TIMEOUT_S", 900)
        self.memory_mb = env_int(environ, "PY_MEMORY_MB", 4096)
        self.output_mb = env_int(environ, "PY_OUTPUT_MB", 16)
        self.open_files = env_int(environ, "PY_OPEN_FILES", 256)
        self.max_concurrent = env_int(environ, "PY_MAX_CONCURRENT", 2)
        self.lock_path = environ.get("PY_LOCK_PATH", os.path.join(HERE, "uv.lock"))
        self.solvers_path = environ.get("PY_SOLVERS_PATH", os.path.join(HERE, "solvers.toml"))
        self.interpreter = environ.get("PY_INTERPRETER", sys.executable)


def sha256_file(path):
    try:
        with open(path, "rb") as handle:
            return hashlib.sha256(handle.read()).hexdigest()
    except OSError:
        return None


def normalize(name):
    return re.sub(r"[-_.]+", "-", name).lower()


def read_lock_versions(path):
    """Package name to version for every package in uv.lock."""
    with open(path, "rb") as handle:
        lock = tomllib.load(handle)
    return {normalize(p["name"]): p.get("version", "") for p in lock.get("package", [])}


def read_solvers(config):
    """The solver rows: solvers.toml entries with versions taken from the lock."""
    with open(config.solvers_path, "rb") as handle:
        entries = tomllib.load(handle).get("solver", [])
    versions = read_lock_versions(config.lock_path)
    rows = []
    for entry in entries:
        version = versions.get(normalize(entry["package"]))
        if version is None:
            continue
        rows.append(
            {
                "package": entry["package"],
                "version": version,
                "import_name": entry["import_name"],
                "area": entry["area"],
                "summary": entry["summary"],
            }
        )
    return rows


def read_text(path, limit_chars):
    """Up to `limit_chars` of a file's text, and whether it was cut."""
    try:
        with open(path, "rb") as handle:
            raw = handle.read(limit_chars * 4 + 4)
            more = bool(handle.read(1))
    except OSError:
        return "", False
    text = raw.decode("utf-8", errors="replace")
    return text[:limit_chars], more or len(text) > limit_chars


def require_object(body, what):
    if not isinstance(body, dict):
        raise HttpError(400, "invalid_input", f"{what} must be a JSON object")
    return body


def common_request(config, body):
    """Validates the fields `/run` and `/search` share."""
    require_object(body, "the request")
    project = body.get("project")
    if not isinstance(project, str) or not SLUG.match(project):
        raise HttpError(400, "invalid_input", "project must be a slug of [a-z0-9-]")
    path = body.get("path")
    if not isinstance(path, str) or not PATH.match(path) or len(path) > 256:
        raise HttpError(400, "invalid_input", "path must be python/work/<module>.py")
    seed = body.get("seed", 0)
    if isinstance(seed, bool) or not isinstance(seed, int) or not 0 <= seed < 2**63:
        raise HttpError(400, "invalid_input", "seed must be a non-negative integer")
    timeout = body.get("timeout_s", min(60, config.max_timeout_s))
    if isinstance(timeout, bool) or not isinstance(timeout, (int, float)) or timeout <= 0:
        raise HttpError(400, "invalid_input", "timeout_s must be a positive number")
    if timeout > config.max_timeout_s:
        raise HttpError(400, "invalid_input", f"timeout_s is over the service cap of {config.max_timeout_s}")
    return project, path, seed, float(timeout)


def project_root(config, project, root):
    """The project's real directory: `root` when the request names one inside
    an allowed directory, else <projects>/<project>."""
    if root is None:
        return os.path.realpath(os.path.join(os.path.realpath(config.projects), project))
    if not isinstance(root, str) or not os.path.isabs(root):
        raise HttpError(400, "invalid_input", "root must be an absolute directory")
    real = os.path.realpath(root)
    if not any(real == allowed or real.startswith(allowed + os.sep) for allowed in config.roots):
        raise HttpError(400, "invalid_input", "root is not inside an allowed directory")
    return real


def resolve_file(config, project, path, root=None):
    """The file's real path, refusing anything that leaves the project."""
    base = project_root(config, project, root)
    target = os.path.realpath(os.path.join(base, path))
    if not target.startswith(base + os.sep) or not target.endswith(".py"):
        raise HttpError(400, "invalid_input", "path leaves the project")
    if not os.path.isfile(target):
        raise HttpError(404, "not_found", f"{path} does not exist in project {project}")
    return os.path.join(base, "python"), target


def module_name(path):
    return path[len("python/") : -len(".py")].replace("/", ".")


def build_request(config, kind, body):
    project, path, seed, timeout = common_request(config, body)
    request = {
        "mode": kind,
        "module": module_name(path),
        "seed": seed,
        "limits": {
            "cpu_s": int(timeout) + 5,
            "memory_mb": config.memory_mb,
            "output_bytes": config.output_mb * 1024 * 1024,
            "open_files": config.open_files,
        },
    }
    if kind == "run":
        entry = body.get("entry")
        if not isinstance(entry, str) or not IDENT.match(entry):
            raise HttpError(400, "invalid_input", "entry must be a function name")
        args = body.get("args", {})
        if not isinstance(args, dict):
            raise HttpError(400, "invalid_input", "args must be a JSON object")
        if len(json.dumps(args)) > MAX_ARGS_BYTES:
            raise HttpError(400, "invalid_input", "args are over 64 KB")
        request.update(entry=entry, args=args)
    else:
        predicate = body.get("predicate")
        if not isinstance(predicate, str) or not IDENT.match(predicate):
            raise HttpError(400, "invalid_input", "predicate must be a function name")
        frame = body.get("frame")
        if not isinstance(frame, dict):
            raise HttpError(400, "invalid_input", "frame must be a JSON object")
        max_frame = body.get("max_frame", 10**8)
        if isinstance(max_frame, bool) or not isinstance(max_frame, int) or max_frame < 1:
            raise HttpError(400, "invalid_input", "max_frame must be a positive integer")
        request.update(predicate=predicate, frame=frame, max_frame=max_frame)
    project_dir, target = resolve_file(config, project, path, body.get("root"))
    request["project_dir"] = project_dir
    return request, target, timeout


def child_environment(workdir):
    environment = {
        "PATH": os.path.dirname(sys.executable) + os.pathsep + os.defpath,
        "PYTHONHASHSEED": "0",
        "PYTHONDONTWRITEBYTECODE": "1",
        "PYTHONUNBUFFERED": "1",
        "PYTHONIOENCODING": "utf-8",
        "HOME": workdir,
        "TMPDIR": workdir,
        "XDG_CACHE_HOME": workdir,
        "MPLCONFIGDIR": workdir,
        "LANG": "C.UTF-8",
    }
    for name in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "NUMEXPR_NUM_THREADS"):
        environment[name] = "1"
    return environment


def kill_group(process):
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass


def signal_limit(returncode):
    """The limit a signal-killed run hit, from its negative return code."""
    names = {
        signal.SIGXCPU: "cpu",
        signal.SIGXFSZ: "output",
        signal.SIGKILL: "memory",
    }
    return names.get(-returncode, "crashed")


def load_result(workdir):
    try:
        with open(os.path.join(workdir, "result.json"), encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, ValueError):
        return None


def load_progress(workdir):
    try:
        with open(os.path.join(workdir, "progress.json"), encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, ValueError):
        return None


def run_in_child(config, request, timeout):
    """Runs the request in a fresh interpreter; returns (answer, elapsed_ms)."""
    workdir = tempfile.mkdtemp(prefix="run-")
    try:
        with open(os.path.join(workdir, "request.json"), "w", encoding="utf-8") as handle:
            json.dump(request, handle)
        started = time.monotonic()
        timed_out = False
        with open(os.path.join(workdir, "out.txt"), "wb") as out, open(
            os.path.join(workdir, "err.txt"), "wb"
        ) as err:
            process = subprocess.Popen(
                [config.interpreter, "-s", "-B", "-P", RUNNER, os.path.join(workdir, "request.json")],
                cwd=workdir,
                env=child_environment(workdir),
                stdin=subprocess.DEVNULL,
                stdout=out,
                stderr=err,
                start_new_session=True,
            )
            try:
                process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
            finally:
                kill_group(process)
            process.wait()
        elapsed_ms = int((time.monotonic() - started) * 1000)
        stdout, stdout_cut = read_text(os.path.join(workdir, "out.txt"), OUTPUT_CHARS)
        stderr, stderr_cut = read_text(os.path.join(workdir, "err.txt"), OUTPUT_CHARS)
        answer = {
            "stdout": stdout,
            "stdout_truncated": stdout_cut,
            "stderr": stderr,
            "stderr_truncated": stderr_cut,
            "elapsed_ms": elapsed_ms,
            "peak_rss_mb": None,
        }
        result = None if timed_out else load_result(workdir)
        progress = load_progress(workdir)
        classify(answer, result, process.returncode, timed_out, progress)
        return answer
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def classify(answer, result, returncode, timed_out, progress):
    """Fills in `result` (and `limit`, `value`, `exception`, `search`)."""
    base = {"limit": None, "value": None, "exception": None, "search": None}
    answer.update(base)
    if timed_out:
        limit_answer(answer, "time", progress)
    elif result is None:
        limit = signal_limit(returncode) if returncode < 0 else "crashed"
        limit_answer(answer, limit, progress)
    else:
        answer["peak_rss_mb"] = result.pop("peak_rss_mb", None)
        status = result.pop("status")
        if status in ("no_entry", "bad_args", "bad_request"):
            raise HttpError(400, "invalid_input", result.get("message", status))
        if status == "limit_memory":
            limit_answer(answer, "memory", progress)
        elif status == "limit_output":
            limit_answer(answer, "output", progress)
        elif status == "returned":
            answer.update(result="returned", value=result["value"])
        elif status == "raised":
            answer.update(result="raised", exception=result["exception"])
        else:
            answer.update(result=status, search=result)


def limit_answer(answer, limit, progress):
    answer.update(result="limit", limit=limit)
    if progress is not None:
        answer["search"] = {"checked": progress.get("checked", 0)}


class Service:
    """The running service: config, digests, and the concurrency gate."""

    def __init__(self, config):
        self.config = config
        self.python = platform.python_version()
        self.lock_sha256 = sha256_file(config.lock_path)
        self.gate = threading.BoundedSemaphore(max(1, config.max_concurrent))

    def health(self):
        return {"status": "ok", "python": self.python, "lock_sha256": self.lock_sha256}

    def solvers(self):
        try:
            rows = read_solvers(self.config)
        except (OSError, ValueError, KeyError) as error:
            raise HttpError(500, "internal", f"cannot read the solver list: {error}") from None
        return {"python": self.python, "lock_sha256": self.lock_sha256, "solvers": rows}

    def execute(self, kind, body):
        request, target, timeout = build_request(self.config, kind, body)
        file_sha256 = sha256_file(target)
        with self.gate:
            answer = run_in_child(self.config, request, timeout)
        answer.update(python=self.python, lock_sha256=self.lock_sha256, file_sha256=file_sha256)
        return answer


def make_handler(service):
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"
        server_version = "supermath-python"

        def log_message(self, format, *args):  # noqa: A002 (stdlib signature)
            sys.stderr.write("%s %s\n" % (self.command, format % args))

        def send_json(self, status, payload):
            data = json.dumps(payload).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def send_error_body(self, error):
            self.send_json(error.status, {"error": {"code": error.code, "message": error.message}})

        def do_GET(self):  # noqa: N802 (stdlib name)
            try:
                if self.path == "/health":
                    self.send_json(200, service.health())
                elif self.path == "/solvers":
                    self.send_json(200, service.solvers())
                else:
                    raise HttpError(404, "not_found", f"no route {self.path}")
            except HttpError as error:
                self.send_error_body(error)

        def do_POST(self):  # noqa: N802 (stdlib name)
            try:
                kinds = {"/run": "run", "/search": "search"}
                if self.path not in kinds:
                    raise HttpError(404, "not_found", f"no route {self.path}")
                self.send_json(200, service.execute(kinds[self.path], self.read_body()))
            except HttpError as error:
                self.send_error_body(error)

        def read_body(self):
            try:
                length = int(self.headers.get("Content-Length", ""))
            except ValueError:
                raise HttpError(400, "invalid_input", "Content-Length is required") from None
            if length > MAX_BODY_BYTES:
                raise HttpError(400, "invalid_input", "the request body is over 256 KB")
            try:
                return json.loads(self.rfile.read(length))
            except ValueError:
                raise HttpError(400, "invalid_input", "the body is not JSON") from None

    return Handler


def make_server(config):
    """A bound, not yet serving, HTTP server for `config`."""
    service = Service(config)
    server = ThreadingHTTPServer((config.host, config.port), make_handler(service))
    server.daemon_threads = True
    return server


def main():
    config = Config()
    server = make_server(config)
    sys.stderr.write(f"supermath-python listening on {config.host}:{server.server_port}\n")
    server.serve_forever()


if __name__ == "__main__":
    main()
