# Python: solvers and computation

- **Status:** Draft
- **Crate:** `supermath-python`
- **Parent:** [`../README.md`](../README.md)
- **Decision:** [`../../../adr/0007-run-python-solvers-as-a-pinned-service.md`](../../../adr/0007-run-python-solvers-as-a-pinned-service.md)

Lean decides what is proved. Most of the work before a proof is
computation: search small cases, fit a sequence, solve a SAT or integer
program, compute a Gröbner basis, optimize a construction, test a
conjecture numerically to 10,000 digits. Python is where the solvers for
that live. These tools run Python files in the project's
[workspace](../files/README.md) against a pinned solver environment, in a
sandbox, and report what came back with enough provenance to re-run it.

## The Python service

A `python` service in `compose.yaml`, built from `docker/python/`:

- **Pinned.** One Python version and one solver environment, locked with
  `uv` in `docker/python/pyproject.toml` and `uv.lock`. The image installs
  the lock at build time; nothing is installed at run time.
- **Behind HTTP.** A small server in the image takes a run request (a file,
  an entry point, JSON arguments), starts a fresh interpreter for it, and
  returns its result, output, and resource use. One process per run; no
  state carries between runs.
- **Per-project package.** It mounts the `projects` volume read-only and
  runs each project's `python/` package from there
  ([`files/`](../files/README.md#layout)).

`supermath-python` reaches the service at `PYTHON_URL` (default
`http://localhost:3160`). `GET /health` reports the Python version and the
lock's digest; both `check` commands report them.

### The solver environment

The lock is the contract: a solver is available exactly when it is in
`uv.lock`, and [`py_solvers`](tools/py_solvers.md) reads the list from the
lock, not from a hand-kept table. The first lock holds:

| Area | Packages |
| --- | --- |
| Symbolic | `sympy`, `python-flint` (FLINT: exact integers, polynomials, matrices), `cypari2` (PARI/GP) |
| Numeric | `numpy`, `scipy`, `mpmath` (arbitrary precision) |
| SMT and SAT | `z3-solver`, `python-sat` (PySAT: CaDiCaL, Glucose, MiniSat) |
| Optimization | `ortools` (CP-SAT, MIP), `highspy` (HiGHS LP/MIP), `cvxpy` |
| Graphs and combinatorics | `networkx`, `pynauty` (canonical labelling, automorphisms) |
| Sequences | `oeis` lookups stay in [`lit_oeis`](../literature/tools/lit_oeis.md); local fitting uses `sympy` and `flint` |
| Jacobian | [`jacobian`](https://github.com/morluto/jacobian), importable as a library as well as reachable over MCP |

SageMath is not in the first lock: it is several gigabytes and pins its own
Python. If it is needed, it gets its own image and service, and its own
ADR.

Adding a package is a change to `docker/python/pyproject.toml` and the lock,
in its own commit, with the reason in the commit message.

### Isolation

Python run by a model is untrusted. The service:

- has no network at run time (an `internal: true` compose network) and runs
  as an unprivileged user with a read-only root filesystem;
- runs each request in a fresh process with limits on wall time, CPU time,
  address space, open files, and output size, and kills its process group
  when any is exceeded;
- gives the run a private, empty temporary directory as its only writable
  path, deleted afterward; the project directory is read-only to runs
  (writes go through [`file_write`](../files/tools/file_write.md), from the
  agent's side);
- sets `PYTHONHASHSEED=0` and passes an explicit `seed` to every run, so a
  run is repeatable unless the code reads the clock.

The container is the boundary. The import check at write time
([`files/`](../files/README.md#path-rules)) is an early refusal, not the
sandbox.

## Running a file

Every Python tool takes a path under `python/work/` and an **entry point**:
a function in that file. The service imports the module, calls the
function with the JSON arguments as keyword arguments, and serializes what
it returns.

```python
# python/work/sunflower_search.py
def largest_free_family(n: int, k: int, seed: int) -> dict:
    ...
    return {"size": best, "family": family}
```

- The return value must be JSON-serializable: `None`, booleans, numbers,
  strings, lists, and dicts with string keys. Integers of any size are
  returned as strings when they exceed 2^53, so nothing is rounded.
- `print` output is captured separately as `stdout`, trimmed to 8,000
  characters; it is for the agent to read, and nothing parses it.
- An exception is a failed run, with its type, message, and the last ten
  frames of the traceback.

### Outcomes

| Outcome | When |
| --- | --- |
| `returned` | the function returned a JSON-serializable value |
| `raised` | the function raised; the exception is reported |
| `unknown` | the run exceeded a limit (time, memory, output) or the service refused it by budget |

A run that times out is `unknown`, never a negative answer about the
mathematics.

### Provenance

Every run carries:

```json
{
  "path": "python/work/sunflower_search.py",
  "file_sha256": "<hex>",
  "imports_sha256": "<hex over every work.* module it imports>",
  "entry": "largest_free_family",
  "args_sha256": "<hex>",
  "seed": 0,
  "python": "3.12.x",
  "lock_sha256": "<hex of uv.lock>",
  "elapsed_ms": 4210,
  "peak_rss_mb": 312
}
```

### Status rules

A Python result is computation, not proof:

- A run that this process executed (`evidence_origin: collected`) can
  record a claim as `checked`, with the frame it covered stated in the
  claim: "holds for all n ≤ 10^6", never "holds".
- It never makes a claim `formalised`. A witness found in Python is
  re-checked in Lean before it refutes anything: `next` suggests
  [`lean_search_counterexample`](../lean/tools/lean_search_counterexample.md)
  or a `decide` proof on the witness.
- A floating-point result is reported with its precision, and cannot record
  an exact claim as `checked` unless the run used exact or interval
  arithmetic (`flint`, `mpmath` intervals, `sympy` rationals), which the
  function declares in its result as `"exact": true`.

## Crate

| | |
| --- | --- |
| Name | `supermath-python` |
| Depends on | `supermath-tool`, `supermath-files`, `reqwest`, `serde_json`, `tokio` |
| Public API | `PythonClient`, `RunRequest`, `RunOutcome`, `Provenance`, `PythonObserver` (a `FileObserver`), and the tools below |
| Configuration | `PYTHON_URL`, `SUPERMATH_PY_DEFAULT_TIMEOUT` (default 60 s), `SUPERMATH_PY_MAX_TIMEOUT` (default 900 s) |
| Tests | every outcome and tool against a loopback stub replaying recorded service answers; `live_py_*` against the real service, gated by `PYTHON_URL` |

## Tools

Each tool is specified in its own file in this crate's `tools/` folder, in
the shape [`../tools.md`](../tools.md) sets out.

| Tool | Given → returns |
| --- | --- |
| [`py_run`](tools/py_run.md) | a Python file, an entry point, and arguments → its returned value, output, and provenance |
| [`py_search_counterexample`](tools/py_search_counterexample.md) | a predicate function in a project file and a frame → a witness, or none in frame |
| [`py_solvers`](tools/py_solvers.md) | nothing, or an area → the solvers in the pinned environment, with versions |

## Acceptance criteria

- `py_run` maps each outcome (returned, raised, timed out, out of memory,
  output over the cap, a non-JSON return value) correctly, against the
  stub.
- A run's provenance is complete and its `args_sha256` matches the
  arguments sent.
- `live_py_run`, gated by `PYTHON_URL`, writes a file computing
  `sympy.factorint(2**64 + 1)` and checks the returned factors.
- `live_py_isolation`, gated the same way, asserts that a run cannot open a
  socket, write to the project directory, or outlive its timeout.
- `py_solvers` lists exactly the packages in the lock.

## Open questions

1. Should a run be able to read a data file written earlier (for example a
   table produced by another run), and through which tool is it written?
2. Do we need GPU solvers, and if so on which host?
3. Should SageMath be a second image from the start?
