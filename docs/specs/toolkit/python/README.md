# Python: solvers and computation

- **Status:** Implemented
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

### HTTP API

The service speaks JSON over HTTP on port 3160. `supermath-python`'s
`PythonClient` and `docker/python/server.py` implement exactly this; both
are tested against the fixtures recorded in
`crates/supermath-python/tests/fixtures/`.

| Route | Does |
| --- | --- |
| `GET /health` | `{status: "ok", python, lock_sha256}` |
| `GET /solvers` | `{python, lock_sha256, solvers: [{package, version, import_name, area, summary}]}`: the rows of `solvers.toml` whose package is in `uv.lock`, with the locked version |
| `POST /run` | calls one function of a project file |
| `POST /search` | searches a frame for a predicate's counterexample |

`POST /run` takes `{project, path, entry, args, seed, timeout_s}`: `project`
is the slug, `path` is `python/work/<module>.py` under it, `args` a JSON
object (at most 64 KB), `seed` a non-negative integer (default 0), and
`timeout_s` a positive number up to the service's cap
(`SUPERMATH_PY_MAX_TIMEOUT`, default 900; default 60). `POST /search` takes
`{project, path, predicate, frame, seed, timeout_s, max_frame}`; `frame` is as
in [`py_search_counterexample`](tools/py_search_counterexample.md) and
`max_frame` (default 10^8) is the most values the service will check.

A run that happened answers `200`, whatever came of it:

```json
{
  "result": "returned",
  "value": {"size": 18},
  "exception": null,
  "limit": null,
  "search": null,
  "stdout": "searched 6 3\n", "stdout_truncated": false,
  "stderr": "", "stderr_truncated": false,
  "python": "3.12.15", "lock_sha256": "<hex>", "file_sha256": "<hex>",
  "elapsed_ms": 121, "peak_rss_mb": 26
}
```

- `/run` `result` is `returned` (`value`, integers past 2^53 as strings),
  `raised` (`exception: {type, message, traceback}`; this includes an import
  error in the file, a value that is not JSON (`NotJsonSerializable`), and
  `SystemExit`), or `limit` (`limit` is `time`, `cpu`, `memory`, `output`, or
  `crashed`; `stdout` holds the output so far).
- `/search` `result` is `found`, `exhausted`, `capped` (a generator reached
  `max_frame`), `predicate_raised`, `raised`, or `limit`; `search` holds
  `checked` (values that passed before the end or the witness), plus
  `witness` and `witness_json` (the witness as lossless JSON text, for the
  generated file) for `found`, and `input` and `exception` for
  `predicate_raised`. After a `limit`, `search.checked` is the last progress
  the run recorded.
- `file_sha256` is the digest of the file as the service ran it; the client
  compares it with the workspace's and reports a mismatch (a service that
  mounts a different projects directory) as `BACKEND_UNAVAILABLE`.

Anything the service refuses answers `{"error": {"code", "message"}}`: `400`
`invalid_input` (a bad field, a path outside `python/work/`, a missing entry
point or predicate, arguments that do not fit the function's signature),
`404` `not_found` (no such file or route), `500` otherwise. The client maps
`400` to `INVALID_INPUT`, `404` to `NOT_FOUND`, and everything else, an
unreachable service, or an unparsable answer to `BACKEND_UNAVAILABLE`.

The server's own settings are environment variables (`PY_MAX_TIMEOUT_S`,
`PY_MEMORY_MB`, `PY_OUTPUT_MB`, `PY_OPEN_FILES`, `PY_MAX_CONCURRENT`, the
projects root `SUPERMATH_PROJECTS`); `docker/python/server.py` lists them.
The server's tests run with plain Python 3.12, no solver packages needed:
`cd docker/python && python3 -m unittest discover -s tests -t .`.

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
| Jacobian | [`jacobian`](https://github.com/morluto/jacobian) 0.9.0 from PyPI, importable as a library as well as reachable over MCP; 0.9.0 is what the lock resolves, since newer releases pull `cypari`, `pplpy`, and `gmpy2` (source builds) and downgrade `scipy` and `z3-solver` |

SageMath is not in the first lock: it is several gigabytes and pins its own
Python. If it is needed, it gets its own image and service, and its own
ADR.

Adding a package is a change to `docker/python/pyproject.toml` and the lock,
in its own commit, with the reason in the commit message.

### Isolation

Python run by a model is untrusted. The service:

- has no network at run time (an `internal: true` compose network, which
  has no egress; the interpreter also refuses IP sockets itself, so a
  refusal is immediate and holds where the network is not isolated) and runs
  as an unprivileged user with a read-only root filesystem, no capabilities,
  and a process limit;
- runs each request in a fresh process with limits on wall time, CPU time,
  address space, open files, and output size, and kills its process group
  when any is exceeded;
- reports a run killed by a signal as the limit it hit (`SIGXCPU` is `cpu`,
  `SIGXFSZ` is `output`, an out-of-memory kill is `memory`); the limits use
  `setrlimit`, and the address-space limit is enforced on Linux only;
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
| `unknown` | the run exceeded a limit (time, CPU, memory, output) or crashed |

A budget refusal is not an outcome: the host refuses a spent compute budget
or a timeout over its cap with `BUDGET_EXCEEDED` before the service is
called, and the service has no budget reply of its own (a `timeout_s` over
its cap is a `400`, so `INVALID_INPUT`).

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
  "args_sha256": "<hex of the arguments' canonical JSON: sorted keys, no whitespace>",
  "seed": 0,
  "python": "3.12.x",
  "lock_sha256": "<hex of uv.lock>",
  "elapsed_ms": 4210,
  "peak_rss_mb": 312
}
```

### Status rules

Without `PYTHON_URL`, every Python tool answers `BACKEND_UNAVAILABLE`, and
`supermath-python` records each run in a `PythonObserver` (the Python
`FileObserver`: the last outcome per file, with the digest it was for) so
`file_list` can show it.

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
| Public API | `Python` (builds the tools, the run log, and the client), `PythonClient`, `RunRequest`, `RunOutcome`, `Provenance`, `PythonObserver` (a `FileObserver`), and the tools below |
| Configuration | `PYTHON_URL`, `SUPERMATH_PY_DEFAULT_TIMEOUT` (default 60 s), `SUPERMATH_PY_MAX_TIMEOUT` (default 900 s), `SUPERMATH_PY_MAX_FRAME` (default 10^8 values) |
| Tests | every outcome and tool against a loopback stub replaying recorded service answers; `live_python_*` against the real service, gated by `PYTHON_URL` (and `SUPERMATH_PROJECTS`, the directory the service mounts) |

## Tools

Each tool is specified in its own file in this crate's `tools/` folder, in
the shape [`../tools.md`](../tools.md) sets out.

| Tool | Given → returns |
| --- | --- |
| [`py_run`](tools/py_run.md) | a Python file, an entry point, and arguments → its returned value, output, and provenance |
| [`py_search_counterexample`](tools/py_search_counterexample.md) | a predicate function in a project file and a frame → a witness, or `exhausted` over the whole frame |
| [`py_solvers`](tools/py_solvers.md) | nothing, or an area → the solvers in the pinned environment, with versions |

## Acceptance criteria

- `py_run` maps each outcome (returned, raised, timed out, out of memory,
  output over the cap, a non-JSON return value) correctly, against the
  stub.
- A run's provenance is complete and its `args_sha256` matches the
  arguments sent.
- `live_python_run`, gated by `PYTHON_URL`, writes a file computing
  `sympy.factorint(2**64 + 1)` and checks the returned factors.
- `live_python_isolation`, gated the same way, asserts that a run cannot open a
  socket, write to the project directory, or outlive its timeout.
- `py_solvers` lists exactly the packages in the lock.

## Open questions

1. Should a run be able to read a data file written earlier (for example a
   table produced by another run), and through which tool is it written?
2. Do we need GPU solvers, and if so on which host?
3. Should SageMath be a second image from the start?
