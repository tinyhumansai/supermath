"""Compiling a project's own modules without Lake.

A project file that imports `Supermath.*` modules needs their `.olean`s. Lake
cannot build them here: the root filesystem is read-only, so it cannot take
its lock inside the baked Mathlib package, and the container has no network to
resolve Mathlib's dependencies. The packages are already built, so all a
project needs is `lean -o` on its own modules, in dependency order, against a
`LEAN_PATH` that lists the baked packages and the project's build directory.
"""

import glob
import os
import subprocess

import normalize as N

BUILD_LIB = os.path.join(".lake", "build", "lib", "lean")


class BuildError(Exception):
    """A project module could not be compiled; `text` says why."""

    def __init__(self, text):
        super().__init__(text)
        self.text = text


def package_paths(lean_env):
    """The library directories of every package baked into `lean_env`."""
    return sorted(glob.glob(os.path.join(lean_env, ".lake", "packages", "*", BUILD_LIB)))


def lean_path(paths):
    """`paths` as a LEAN_PATH value."""
    return os.pathsep.join(paths)


def module_source(lean_dir, module):
    """The file that defines `module`: `Supermath.A.B` is `Supermath/A/B.lean`."""
    return os.path.join(lean_dir, *module.split(".")) + ".lean"


def module_olean(lean_dir, module, ext="olean"):
    """Where `module`'s compiled output goes in the project's build directory."""
    return os.path.join(lean_dir, BUILD_LIB, *module.split(".")) + "." + ext


def build_order(lean_dir, modules, read=None):
    """Every project module `modules` needs, dependencies first.

    Raises BuildError for a module with no file or an import cycle.
    """
    read = read or _read
    order, state = [], {}

    def visit(module, chain):
        if state.get(module) == "done":
            return
        if state.get(module) == "open":
            raise BuildError("import cycle: " + " -> ".join(chain + [module]))
        source = read(module_source(lean_dir, module))
        if source is None:
            raise BuildError(f"unknown module {module}: no file {module_source(lean_dir, module)}")
        state[module] = "open"
        for dep in N.project_imports(source):
            visit(dep, chain + [module])
        state[module] = "done"
        order.append(module)

    for module in modules:
        visit(module, [])
    return order


def stale(lean_dir, module, deps, mtime=None):
    """Whether `module` must be compiled: no olean, or one older than its
    source or than any of its project dependencies' oleans."""
    mtime = mtime or _mtime
    built = mtime(module_olean(lean_dir, module))
    if built is None:
        return True
    newer = [mtime(module_source(lean_dir, module))]
    newer += [mtime(module_olean(lean_dir, dep)) for dep in deps]
    return any(t is None or t > built for t in newer)


def build_modules(lean_dir, modules, packages, timeout, run=subprocess.run):
    """Compiles the project modules `modules` need, oldest dependency first,
    and returns the LEAN_PATH a REPL needs to import them.

    Raises BuildError with Lean's output when a module does not compile, and
    subprocess.TimeoutExpired when compiling takes longer than `timeout`.
    """
    paths = packages + [os.path.join(lean_dir, BUILD_LIB)]
    env = dict(os.environ, LEAN_PATH=lean_path(paths))
    for module in build_order(lean_dir, modules):
        with open(module_source(lean_dir, module), encoding="utf-8") as f:
            deps = N.project_imports(f.read())
        if not stale(lean_dir, module, deps):
            continue
        olean = module_olean(lean_dir, module)
        os.makedirs(os.path.dirname(olean), exist_ok=True)
        done = run(
            ["lean", "-o", olean, "-i", module_olean(lean_dir, module, "ilean"),
             module_source(lean_dir, module)],
            cwd=lean_dir, env=env, capture_output=True, text=True, timeout=timeout,
        )
        if done.returncode != 0:
            raise BuildError(f"{module} does not compile:\n{(done.stdout + done.stderr)[-1500:]}")
    return lean_path(paths)


def _read(path):
    try:
        with open(path, encoding="utf-8") as f:
            return f.read()
    except OSError:
        return None


def _mtime(path):
    try:
        return os.stat(path).st_mtime_ns
    except OSError:
        return None
