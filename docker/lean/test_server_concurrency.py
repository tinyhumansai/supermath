"""Tests for the server's package build under concurrent requests.

    python3 -I -m unittest discover -s docker/lean

No Lean is needed: `build_package` takes the `run` that compiles a module, and
the fakes here record who is compiling what, and when.
"""

import os
import subprocess
import tempfile
import threading
import time
import unittest

import build as B
import server as S


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


class Probe:
    """A fake `lean -o` that notes how many compiles of one olean overlap."""

    def __init__(self, pause=0.15):
        self.pause, self.lock = pause, threading.Lock()
        self.active, self.overlaps, self.compiles = {}, [], []

    def __call__(self, argv, **kw):
        olean = argv[2]
        with self.lock:
            self.active[olean] = self.active.get(olean, 0) + 1
            self.compiles.append(olean)
            if self.active[olean] > 1:
                self.overlaps.append(olean)
        time.sleep(self.pause)
        write(olean, "olean")
        write(argv[4], "ilean")
        with self.lock:
            self.active[olean] -= 1
        return subprocess.CompletedProcess(argv, 0, "", "")


def in_threads(jobs):
    """Runs each job in its own thread; returns the exceptions they raised."""
    errors = []

    def guarded(job):
        try:
            job()
        except BaseException as error:  # noqa: BLE001 - reported to the test
            errors.append(error)

    threads = [threading.Thread(target=guarded, args=(job,)) for job in jobs]
    for t in threads:
        t.start()
    for t in threads:
        t.join(30)
    return errors


class ConcurrentBuilds(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = tmp.name
        self.saved = (S.PROJECTS, S.BUILDS)
        S.PROJECTS, S.BUILDS = os.path.join(self.tmp, "projects"), os.path.join(self.tmp, "builds")
        self.addCleanup(lambda: setattr(S, "PROJECTS", self.saved[0]) or setattr(S, "BUILDS", self.saved[1]))

    def project(self, name, volume):
        base = S.PROJECTS if volume else os.path.join(self.tmp, "checkouts")
        project = os.path.join(base, name)
        write(os.path.join(project, "lean", "Supermath", "Shared.lean"), "import Mathlib\n")
        write(os.path.join(project, "lean", "Supermath", "Top.lean"), "import Supermath.Shared\n")
        return project

    def test_two_checks_of_one_projects_volume_never_compile_a_module_at_once(self):
        self.same_project_serialises(volume=True)

    def test_two_checks_of_one_staged_project_never_compile_a_module_at_once(self):
        self.same_project_serialises(volume=False)

    def same_project_serialises(self, volume):
        project = self.project("p", volume)
        probe = Probe()
        errors = in_threads([lambda: S.build_package(project, ["Supermath.Top"], 10, probe)] * 4)
        self.assertEqual(errors, [])
        self.assertEqual(probe.overlaps, [], "one olean was written by two compiles at once")
        self.assertEqual(len(probe.compiles), 2, "the later requests reuse what the first built")

    def test_a_restage_waits_for_the_build_in_progress(self):
        # Ordered by events, not sleeps: the edit lands and the second request
        # is on its way before the first compile reads its source back, and
        # the compile gives the restage a generous window to run under it.
        project = self.project("p", volume=False)
        source = os.path.join(project, "lean", "Supermath", "Shared.lean")
        compiling, requested, restaged = threading.Event(), threading.Event(), threading.Event()
        seen = []
        stage = B.stage

        def watched_stage(*args, **kw):
            if compiling.is_set():
                restaged.set()
            return stage(*args, **kw)

        def run(argv, **kw):
            before = read(argv[-1])
            if not compiling.is_set():
                compiling.set()
                requested.wait(5)
                restaged.wait(0.5)  # times out unless the restage runs under this compile
            seen.append(before == read(argv[-1]))
            return Probe(0)(argv, **kw)

        def edit_then_build():
            compiling.wait(5)
            write(source, "import Mathlib\n-- edited\n")
            os.utime(source, (2_000_000_000, 2_000_000_000))
            requested.set()
            S.build_package(project, ["Supermath.Top"], 10, run)

        B.stage = watched_stage
        try:
            errors = in_threads([lambda: S.build_package(project, ["Supermath.Top"], 10, run), edit_then_build])
        finally:
            B.stage = stage
        self.assertEqual(errors, [])
        self.assertTrue(seen and all(seen), "a source changed under a compile")

    def test_a_request_waiting_for_the_lock_gives_up_at_its_timeout(self):
        project = self.project("p", volume=True)
        outcome = []

        def request():
            try:
                S.build_package(project, ["Supermath.Top"], 0.3, Probe(0))
                outcome.append("built")
            except subprocess.TimeoutExpired:
                outcome.append("timeout")

        with B.project_lock(os.path.join(project, "lean")):
            waiter = threading.Thread(target=request)
            waiter.start()
            waiter.join(3)
            gave_up = not waiter.is_alive()
        waiter.join(5)
        self.assertTrue(gave_up, "the wait for the lock ignored the request's timeout")
        self.assertEqual(outcome, ["timeout"])

    def test_checks_of_different_projects_build_at_the_same_time(self):
        first, second = self.project("a", True), self.project("b", False)
        both = threading.Barrier(2, timeout=5)

        def run(argv, **kw):
            both.wait()  # passes only while the other project is compiling too
            return Probe(0)(argv, **kw)

        errors = in_threads([
            lambda: S.build_package(first, ["Supermath.Shared"], 10, run),
            lambda: S.build_package(second, ["Supermath.Shared"], 10, run),
        ])
        self.assertEqual(errors, [], "different projects were serialised")


class AtomicOleans(unittest.TestCase):
    def test_a_failed_compile_leaves_no_olean_behind(self):
        with tempfile.TemporaryDirectory() as lean_dir:
            write(B.module_source(lean_dir, "Supermath.A"), "import Mathlib\n")

            def run(argv, **kw):
                write(argv[2], "half an olean")
                return subprocess.CompletedProcess(argv, 1, "error", "")

            with self.assertRaises(B.BuildError):
                B.build_modules(lean_dir, ["Supermath.A"], [], 10, run)
            self.assertFalse(os.path.exists(B.module_olean(lean_dir, "Supermath.A")))
            self.assertEqual(os.listdir(os.path.dirname(B.module_olean(lean_dir, "Supermath.A"))), [])

    def test_a_timed_out_compile_leaves_no_olean_behind(self):
        with tempfile.TemporaryDirectory() as lean_dir:
            write(B.module_source(lean_dir, "Supermath.A"), "import Mathlib\n")

            def run(argv, **kw):
                write(argv[2], "half an olean")
                raise subprocess.TimeoutExpired(argv, 1)

            with self.assertRaises(subprocess.TimeoutExpired):
                B.build_modules(lean_dir, ["Supermath.A"], [], 10, run)
            self.assertEqual(os.listdir(os.path.dirname(B.module_olean(lean_dir, "Supermath.A"))), [])


if __name__ == "__main__":
    unittest.main()
