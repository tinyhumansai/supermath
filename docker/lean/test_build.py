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


class Stage(unittest.TestCase):
    def write(self, root, relative, text):
        path = os.path.join(root, relative)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        return path

    def test_mirrors_lean_sources_and_reuses_its_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = os.path.join(tmp, "candidate", "lean")
            builds = os.path.join(tmp, "builds")
            self.write(source, "Supermath/Statement.lean", "def Conjecture : Prop := True\n")
            self.write(source, "Supermath/Prove.lean", "import Supermath.Statement\n")
            self.write(source, ".lake/build/junk.olean", "x")
            self.write(source, "notes.txt", "not lean")
            os.symlink("/etc/passwd", os.path.join(source, "Supermath", "Link.lean"))
            dest = B.stage(source, builds)
            self.assertTrue(dest.startswith(builds + os.sep))
            self.assertTrue(os.path.isfile(os.path.join(dest, "Supermath", "Prove.lean")))
            for absent in [".lake", "notes.txt", os.path.join("Supermath", "Link.lean")]:
                self.assertFalse(os.path.exists(os.path.join(dest, absent)), absent)
            self.assertFalse(os.path.exists(os.path.join(source, "lakefile.toml")), "the source is never written")
            self.assertEqual(B.stage(source, builds), dest)

    def test_keeps_modification_times_and_drops_deleted_sources(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = os.path.join(tmp, "candidate", "lean")
            builds = os.path.join(tmp, "builds")
            kept = self.write(source, "Supermath/A.lean", "theorem a : True := trivial\n")
            gone = self.write(source, "Supermath/B.lean", "theorem b : True := trivial\n")
            os.utime(kept, (1_000_000, 1_000_000))
            dest = B.stage(source, builds)
            self.assertEqual(int(os.stat(os.path.join(dest, "Supermath", "A.lean")).st_mtime), 1_000_000)
            built = self.write(dest, ".lake/build/lib/lean/Supermath/A.olean", "built")
            os.remove(gone)
            with open(kept, "a", encoding="utf-8") as f:
                f.write("-- edited\n")
            B.stage(source, builds)
            self.assertFalse(os.path.exists(os.path.join(dest, "Supermath", "B.lean")))
            with open(os.path.join(dest, "Supermath", "A.lean"), encoding="utf-8") as f:
                self.assertIn("edited", f.read())
            self.assertTrue(os.path.exists(built), "build output survives a restage")

    def test_two_projects_build_apart(self):
        with tempfile.TemporaryDirectory() as tmp:
            builds = os.path.join(tmp, "builds")
            one = os.path.join(tmp, "one", "lean")
            two = os.path.join(tmp, "two", "lean")
            for source in [one, two]:
                self.write(source, "Supermath/A.lean", "theorem a : True := trivial\n")
            self.assertNotEqual(B.stage(one, builds), B.stage(two, builds))


if __name__ == "__main__":
    unittest.main()
