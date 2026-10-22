"""Tests for build.py: which project modules compile, in what order, and when.

    python3 -I -m unittest discover -s docker/lean
"""

import os
import subprocess
import tempfile
import unittest

import build as B


def files(mapping):
    """A `read` over `{module: source}` laid out under /p."""
    by_path = {B.module_source("/p", m): src for m, src in mapping.items()}
    return lambda path: by_path.get(path)


class Order(unittest.TestCase):
    def test_dependencies_come_first_and_each_module_once(self):
        read = files({
            "Supermath.A": "import Mathlib\nimport Supermath.B\nimport Supermath.C\n",
            "Supermath.B": "import Supermath.C\n",
            "Supermath.C": "import Mathlib\n",
        })
        self.assertEqual(B.build_order("/p", ["Supermath.A"], read),
                         ["Supermath.C", "Supermath.B", "Supermath.A"])

    def test_a_missing_module_and_a_cycle_are_build_errors(self):
        with self.assertRaises(B.BuildError) as missing:
            B.build_order("/p", ["Supermath.X"], files({}))
        self.assertIn("unknown module Supermath.X", missing.exception.text)
        cyclic = files({"Supermath.A": "import Supermath.B\n", "Supermath.B": "import Supermath.A\n"})
        with self.assertRaises(B.BuildError) as cycle:
            B.build_order("/p", ["Supermath.A"], cyclic)
        self.assertIn("Supermath.A -> Supermath.B -> Supermath.A", cycle.exception.text)


class Staleness(unittest.TestCase):
    def test_a_module_rebuilds_when_its_olean_is_missing_or_older_than_an_input(self):
        times = {}
        mtime = times.get
        src, olean = B.module_source("/p", "Supermath.A"), B.module_olean("/p", "Supermath.A")
        dep = B.module_olean("/p", "Supermath.B")
        times.update({src: 1})
        self.assertTrue(B.stale("/p", "Supermath.A", [], mtime))
        times.update({olean: 2, dep: 1})
        self.assertFalse(B.stale("/p", "Supermath.A", ["Supermath.B"], mtime))
        times[src] = 3
        self.assertTrue(B.stale("/p", "Supermath.A", [], mtime))
        times.update({src: 1, dep: 5})
        self.assertTrue(B.stale("/p", "Supermath.A", ["Supermath.B"], mtime))


class Compile(unittest.TestCase):
    def test_stale_modules_compile_in_order_with_the_packages_on_lean_path(self):
        with tempfile.TemporaryDirectory() as lean_dir:
            for module, text in {"Supermath.A": "import Supermath.B\n", "Supermath.B": "import Mathlib\n"}.items():
                os.makedirs(os.path.dirname(B.module_source(lean_dir, module)), exist_ok=True)
                with open(B.module_source(lean_dir, module), "w", encoding="utf-8") as f:
                    f.write(text)
            calls = []

            def run(argv, **kw):
                calls.append((argv, kw["env"]["LEAN_PATH"]))
                with open(argv[2], "w") as f:
                    f.write("olean")
                return subprocess.CompletedProcess(argv, 0, "", "")

            path = B.build_modules(lean_dir, ["Supermath.A"], ["/pkg/mathlib"], 10, run)
            self.assertEqual([c[0][-1] for c in calls],
                             [B.module_source(lean_dir, "Supermath.B"), B.module_source(lean_dir, "Supermath.A")])
            self.assertEqual(path, os.pathsep.join(["/pkg/mathlib", os.path.join(lean_dir, B.BUILD_LIB)]))
            calls.clear()
            B.build_modules(lean_dir, ["Supermath.A"], ["/pkg/mathlib"], 10, run)
            self.assertEqual(calls, [], "up-to-date modules are not recompiled")

    def test_a_module_that_does_not_compile_reports_leans_output(self):
        with tempfile.TemporaryDirectory() as lean_dir:
            os.makedirs(os.path.join(lean_dir, "Supermath"))
            with open(B.module_source(lean_dir, "Supermath.A"), "w", encoding="utf-8") as f:
                f.write("import Mathlib\n")

            def run(argv, **kw):
                return subprocess.CompletedProcess(argv, 1, "A.lean:3:0: error: unknown identifier", "")

            with self.assertRaises(B.BuildError) as failed:
                B.build_modules(lean_dir, ["Supermath.A"], [], 10, run)
            self.assertIn("Supermath.A does not compile", failed.exception.text)
            self.assertIn("unknown identifier", failed.exception.text)

    def test_package_paths_lists_each_baked_package_library(self):
        with tempfile.TemporaryDirectory() as env:
            for name in ["mathlib", "batteries"]:
                os.makedirs(os.path.join(env, ".lake", "packages", name, B.BUILD_LIB))
            self.assertEqual([p.split(os.sep)[-5] for p in B.package_paths(env)], ["batteries", "mathlib"])


if __name__ == "__main__":
    unittest.main()
