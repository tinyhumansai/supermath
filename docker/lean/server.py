"""The supermath Lean service: a small HTTP wrapper around pooled
leanprover-community/repl processes.

Routes (JSON in, JSON out; the contract is docs/specs/toolkit/lean/README.md):

    GET  /health          toolchain and Mathlib revision
    POST /check           elaborate a project file; messages, sorries, axioms
    POST /goals           the goals at a position
    POST /tactics         tactics tried against one sorry
    POST /decl            a declaration's signature, docstring, module
    POST /search          Mathlib declarations by name fragment
    POST /counterexample  first value of a range satisfying a predicate

The service reads project files itself from the `projects` volume
(PROJECTS_DIR); a request names a project slug and a path, never source. A
request may also name `root`, the project's absolute directory, when the
project is not on that volume (an rsi-ralph candidate checkout); the root must
lie inside one of the directories SUPERMATH_ROOTS lists, or it is refused.
Such a project is only ever read: its package is built in a copy under
LEAN_BUILDS_DIR (`build.stage`), so no build output lands in the checkout.
"""

import json
import os
import re
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import build
import normalize as N
from repl import PACKAGES, Pool, Repl, ReplDied, ReplOutputLimit, ReplTimeout

PROJECTS = os.environ.get("PROJECTS_DIR", "/projects")
ROOTS = N.allowed_roots(os.environ.get("SUPERMATH_ROOTS"))
BUILDS = os.environ.get("LEAN_BUILDS_DIR", "/tmp/lean-builds")
STAGING = threading.Lock()
POOL = Pool(int(os.environ.get("LEAN_POOL_SIZE", "2")))
CHUNK = 1000
LAKEFILE = """name = "supermath"
defaultTargets = ["Supermath"]

[[lean_lib]]
name = "Supermath"
globs = ["Supermath", "Supermath.+"]

[[require]]
name = "mathlib"
path = "/opt/lean-env/.lake/packages/mathlib"
"""


def read_text(path, default=""):
    try:
        with open(path, encoding="utf-8") as f:
            return f.read().strip()
    except OSError:
        return default


TOOLCHAIN = read_text("/opt/toolchain", "unknown")
MATHLIB = read_text("/opt/mathlib-rev", "unknown")


class Refused(Exception):
    def __init__(self, status, message):
        super().__init__(message)
        self.status, self.message = status, message


def project_file(req):
    """Resolves {project, root?, path} to (project directory, source),
    refusing anything odd."""
    try:
        project = N.project_dir(req, PROJECTS, ROOTS)
    except ValueError as refused:
        raise Refused(400, str(refused))
    path = req.get("path", "")
    if not isinstance(path, str) or not N.PATH_RE.match(path):
        raise Refused(400, "bad project or path")
    real = os.path.realpath(os.path.join(project, path))
    root = os.path.realpath(project)
    if not real.startswith(root + os.sep) or os.path.islink(os.path.join(project, path)):
        raise Refused(400, "path leaves the project")
    try:
        with open(real, encoding="utf-8") as f:
            return project, f.read()
    except OSError:
        raise Refused(404, "no such file on the projects volume")


def ensure_package(project):
    """The directory to build `project`'s package in: its own `lean/` on the
    projects volume, a staged copy for a project elsewhere."""
    lean_dir = os.path.join(project, "lean")
    volume = os.path.realpath(PROJECTS)
    if not os.path.realpath(project).startswith(volume + os.sep):
        with STAGING:
            lean_dir = build.stage(lean_dir, BUILDS)
    for name, text in (("lakefile.toml", LAKEFILE), ("lean-toolchain", TOOLCHAIN + "\n")):
        target = os.path.join(lean_dir, name)
        if not os.path.exists(target):
            with open(target, "w", encoding="utf-8") as f:
                f.write(text)
    return lean_dir


def build_package(project, mods, timeout, run=subprocess.run):
    """Stages and builds `project`'s package; returns (lean_dir, LEAN_PATH)."""
    lean_dir = ensure_package(project)
    return lean_dir, build.build_modules(lean_dir, mods, PACKAGES, timeout, run)


def run_guarded(operation):
    """Runs `operation()`; maps REPL failures to a service status."""
    started = time.monotonic()
    try:
        reply = operation()
    except (ReplTimeout, subprocess.TimeoutExpired):
        reply = {"status": "timeout"}
    except ReplOutputLimit:
        reply = {"status": "output_limit"}
    except ReplDied as died:
        reply = {"status": "memory" if died.memory else "timeout"}
    reply.setdefault("status", "ok")
    reply["toolchain"] = TOOLCHAIN
    reply["mathlib"] = MATHLIB
    reply["elapsed_ms"] = int((time.monotonic() - started) * 1000)
    return reply


def elaborate(project, source, timeout, with_tactics=False):
    """Returns (repl_context, first_answer). The context is a callable that
    runs more commands in the environment the source produced."""
    options = {"allTactics": True} if with_tactics else {}
    mods = N.project_imports(source)
    if not mods:
        lease = POOL.lease()
        repl = lease.__enter__()

        def release():
            lease.__exit__(None, None, None)

        try:
            answer = repl.request({"cmd": N.blank_header(source), "env": repl.base_env, **options}, timeout)
        except BaseException:
            release()
            raise
        return repl, answer, release
    try:
        lean_dir, path = build_package(project, mods, timeout)
    except build.BuildError as failed:
        fail = {"messages": [{"severity": "error", "pos": {"line": 1, "column": 0},
                              "data": "an imported project module failed to build:\n" + failed.text}]}
        return None, fail, lambda: None
    repl = Repl(cwd=lean_dir, lean_path=path)
    try:
        answer = repl.request({"cmd": source, **options}, timeout)
    except BaseException:
        repl.kill()
        raise
    return repl, answer, repl.kill


def check(req):
    project, source = project_file(req)
    timeout = req["timeout_ms"] / 1000
    targets = [t for t in req.get("targets", []) if N.valid_name(t)]

    def operation():
        try:
            return checked()
        except ReplDied as died:
            # A REPL that died on its own crashed on this file: report that
            # as the file's error. The memory limit stays `memory`.
            if died.memory:
                raise
            return N.crash_reply(died.code)

    def checked():
        repl, answer, release = elaborate(project, source, timeout)
        try:
            reply = {
                "messages": N.flatten_messages(answer.get("messages")),
                "sorries": N.flatten_sorries(answer.get("sorries")),
                "axioms": {},
                "uses": {},
            }
            if repl is not None and "env" in answer and targets:
                commands = "\n".join(f"#print axioms {t}" for t in targets)
                axioms = repl.request({"cmd": commands, "env": answer["env"]}, timeout)
                reply["axioms"] = N.axioms_by_target(axioms.get("messages"), targets)
                if req.get("uses"):
                    used = repl.request({"cmd": N.uses_command(targets), "env": answer["env"]}, timeout)
                    reply["uses"] = N.parse_uses(used.get("messages"))
            return reply
        finally:
            release()

    return run_guarded(operation)


def goals(req):
    project, source = project_file(req)
    line, column = req["line"], req["column"]
    lines = source.split("\n")
    if not 1 <= line <= len(lines) or column > len(lines[line - 1]):
        return {"status": "bad_position"}

    def operation():
        repl, answer, release = elaborate(project, source, req["timeout_ms"] / 1000, with_tactics=True)
        release()
        return {"goals": N.goals_at(answer, line, column)}

    return run_guarded(operation)


def tactics(req):
    project, source = project_file(req)
    per_tactic = req["timeout_ms"] / 1000

    def operation():
        repl, answer, release = elaborate(project, source, per_tactic)
        try:
            state = N.find_sorry(answer, req["line"], req["column"])
            if state is None or repl is None:
                return {"status": "no_sorry"}
            results = []
            for tactic in req["tactics"]:
                reply = repl.request({"tactic": tactic, "proofState": state}, per_tactic)
                results.append(N.tactic_result(tactic, reply))
            return {"results": results}
        finally:
            release()

    return run_guarded(operation)


def decl(req):
    name = req.get("name", "")
    if not N.valid_name(name):
        raise Refused(400, "bad declaration name")
    timeout = req["timeout_ms"] / 1000

    def operation():
        if req.get("path"):
            project, source = project_file(req)
            repl, answer, release = elaborate(project, source, timeout)
            env = answer.get("env")
        else:
            lease = POOL.lease()
            repl = lease.__enter__()
            release, env = (lambda: lease.__exit__(None, None, None)), repl.base_env
        try:
            if repl is None or env is None:
                return {"status": "not_found"}
            reply = repl.request({"cmd": N.decl_command(name), "env": env}, timeout)
            found = N.parse_decl(reply.get("messages"), name)
            return found or {"status": "not_found"}
        finally:
            release()

    return run_guarded(operation)


def search(req):
    query = req.get("query", "")
    if not query or len(query) > 300 or any(c in query for c in '"\\\n'):
        raise Refused(400, "bad query")

    def operation():
        with POOL.lease() as repl:
            reply = repl.request(
                {"cmd": N.search_command(query, req.get("limit", 10)), "env": repl.base_env},
                req["timeout_ms"] / 1000,
            )
        return {"results": N.parse_search(reply.get("messages"))}

    return run_guarded(operation)


def counterexample(req):
    project, source = project_file(req)
    predicate = req.get("predicate", "")
    if not N.valid_name(predicate):
        raise Refused(400, "bad predicate name")
    lo, hi = int(req["from"]), int(req["to"])
    deadline = time.monotonic() + req["timeout_ms"] / 1000

    def left():
        return max(deadline - time.monotonic(), 0.001)

    def operation():
        repl, answer, release = elaborate(project, source, left())
        try:
            if repl is None or "env" not in answer:
                return {"status": "bad_predicate", "message": "the file does not elaborate"}
            env = answer["env"]
            domain, message = None, ""
            for candidate in ("nat", "int"):
                probe = repl.request({"cmd": N.counter_probe(predicate, candidate), "env": env}, left())
                errors = N.errors_of(probe)
                if not errors:
                    domain = candidate
                    break
                message = message or errors[0]["text"]
            if domain is None:
                return {"status": "bad_predicate", "message": message[:400]}
            start = max(lo, 0) if domain == "nat" else lo
            covered = None
            try:
                while start <= hi:
                    stop = min(start + CHUNK - 1, hi)
                    chunk = repl.request(
                        {"cmd": N.counter_chunk(predicate, domain, start, stop), "env": env}, left()
                    )
                    found, value = N.parse_option(chunk.get("messages"))
                    if found:
                        return {"domain": domain, "witness": value, "covered_to": value}
                    covered, start = stop, stop + 1
            except ReplTimeout:
                return {"status": "timeout", "domain": domain, "witness": None, "covered_to": covered}
            return {"domain": domain, "witness": None, "covered_to": covered if covered is not None else hi}
        finally:
            release()

    return run_guarded(operation)


ROUTES = {
    "/check": check,
    "/goals": goals,
    "/tactics": tactics,
    "/decl": decl,
    "/search": search,
    "/counterexample": counterexample,
}


class Handler(BaseHTTPRequestHandler):
    server_version = "supermath-lean"

    def _send(self, status, body):
        data = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == "/health":
            self._send(200, {"status": "ok", "toolchain": TOOLCHAIN, "mathlib": MATHLIB})
        else:
            self._send(404, {"status": "not_found", "message": "no such route"})

    def do_POST(self):
        route = ROUTES.get(self.path)
        if route is None:
            return self._send(404, {"status": "not_found", "message": "no such route"})
        try:
            length = int(self.headers.get("Content-Length", "0"))
            if length > (1 << 20):
                raise Refused(400, "request too large")
            req = json.loads(self.rfile.read(length) or b"{}")
            self._send(200, route(req))
        except Refused as refused:
            self._send(refused.status, {"status": "refused", "message": refused.message})
        except (ValueError, KeyError, TypeError):
            self._send(400, {"status": "refused", "message": "malformed request"})
        except Exception as error:  # noqa: BLE001 - the service must keep serving
            self._send(500, {"status": "error", "message": type(error).__name__})

    def log_message(self, fmt, *args):
        pass


if __name__ == "__main__":
    port = int(os.environ.get("PORT", "3150"))
    ThreadingHTTPServer(("0.0.0.0", port), Handler).serve_forever()
