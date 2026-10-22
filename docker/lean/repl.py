"""Processes: warm REPLs with `import Mathlib` already elaborated, and
one-shot REPLs for files that import project modules.

The container is the sandbox (no network, unprivileged user, read-only root,
a memory limit). This module adds the per-request wall-time and output caps.
"""

import json
import os
import select
import subprocess
import threading
import time

MAX_OUTPUT = 4 << 20  # bytes of REPL output per answer
RECYCLE_AFTER = 40  # commands before a warm REPL is replaced (envs accumulate)
LEAN_ENV = os.environ.get("LEAN_ENV", "/opt/lean-env")
REPL_BIN = os.environ.get("REPL_BIN", "/opt/repl/.lake/build/bin/repl")


class ReplTimeout(Exception):
    """The REPL did not answer before the deadline."""


class ReplOutputLimit(Exception):
    """The REPL's answer passed MAX_OUTPUT bytes."""


class ReplDied(Exception):
    """The REPL exited; `memory` is true when it was killed by SIGKILL, which
    is how the container's memory limit shows up."""

    def __init__(self, memory):
        super().__init__("the REPL died")
        self.memory = memory


class Repl:
    def __init__(self, cwd=None, lake_env=False):
        argv = [REPL_BIN]
        if lake_env:
            argv = ["lake", "env", REPL_BIN]
        self.proc = subprocess.Popen(
            argv,
            cwd=cwd or LEAN_ENV,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        self.commands = 0

    def request(self, obj, timeout):
        """Sends one command and reads one answer, within `timeout` seconds."""
        deadline = time.monotonic() + timeout
        self.commands += 1
        try:
            self.proc.stdin.write(json.dumps(obj).encode() + b"\n\n")
            self.proc.stdin.flush()
        except (BrokenPipeError, OSError):
            raise ReplDied(self._memory_death())
        buf = b""
        fd = self.proc.stdout.fileno()
        while not buf.endswith(b"\n\n"):
            left = deadline - time.monotonic()
            if left <= 0:
                self.kill()
                raise ReplTimeout()
            ready, _, _ = select.select([fd], [], [], left)
            if not ready:
                continue
            chunk = os.read(fd, 65536)
            if not chunk:
                raise ReplDied(self._memory_death())
            buf += chunk
            if len(buf) > MAX_OUTPUT:
                self.kill()
                raise ReplOutputLimit()
        return json.loads(buf.decode("utf-8", "replace"))

    def _memory_death(self):
        try:
            return self.proc.wait(timeout=1) == -9
        except subprocess.TimeoutExpired:
            return False

    def alive(self):
        return self.proc.poll() is None

    def kill(self):
        try:
            os.killpg(self.proc.pid, 9)
        except (ProcessLookupError, PermissionError):
            pass
        self.proc.wait()


class Pool:
    """A fixed number of warm REPLs. `lease()` hands one out exclusively."""

    def __init__(self, size):
        self.free = []
        self.sema = threading.Semaphore(size)
        self.lock = threading.Lock()

    def _fresh(self):
        repl = Repl()
        answer = repl.request({"cmd": "import Mathlib"}, timeout=600)
        repl.base_env = answer["env"]
        return repl

    def lease(self):
        pool = self

        class Lease:
            def __enter__(self):
                pool.sema.acquire()
                with pool.lock:
                    self.repl = pool.free.pop() if pool.free else None
                if self.repl is None or not self.repl.alive() or self.repl.commands > RECYCLE_AFTER:
                    if self.repl is not None:
                        self.repl.kill()
                    try:
                        self.repl = pool._fresh()
                    except BaseException:
                        pool.sema.release()
                        raise
                return self.repl

            def __exit__(self, kind, value, trace):
                with pool.lock:
                    if self.repl.alive():
                        pool.free.append(self.repl)
                pool.sema.release()
                return False

        return Lease()
