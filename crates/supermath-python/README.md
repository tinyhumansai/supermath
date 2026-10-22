# supermath-python

The Python solver tools for the supermath agent: `py_run`,
`py_search_counterexample`, and `py_solvers`, and the HTTP client that reaches
the pinned, sandboxed Python service (`docker/python/`). The behavior is
specified in [`docs/specs/toolkit/python/`](../../docs/specs/toolkit/python/README.md);
the decision in [ADR 0007](../../docs/adr/0007-run-python-solvers-as-a-pinned-service.md).

A tool never takes source. It runs a *file* already written to the project
workspace (`python/work/*.py`, via `file_write`) and a function in it, and
returns the value with provenance: the file's digest, its `work.*` imports'
digest, the arguments' digest, the seed, the Python version, and the digest of
the service's `uv.lock`. A Python result is computation, not proof: it can
back a claim as `checked` on a stated frame, never `formalised`.

## Layout

| Module | Holds |
| --- | --- |
| `config/` | `PythonConfig`: `PYTHON_URL`, `SUPERMATH_PY_DEFAULT_TIMEOUT`, `SUPERMATH_PY_MAX_TIMEOUT`, `SUPERMATH_PY_MAX_FRAME` |
| `client/` | `PythonClient` and the wire types (`RunRequest`, `RunOutcome`, `SearchOutcome`, `Limit`, ...) |
| `frame/` | `Frame`: the range, product, or generator a search covers |
| `provenance/` | `Provenance`, and the canonical JSON `args_sha256` is taken over |
| `observer/` | `PythonObserver`, the run log `file_list` reads (`FileObserver`) |
| `witness/` | the generated `python/work/witnesses/` file a found counterexample is written to |
| `tools/` | `Python` (builds everything) and the three tools |

## Wiring

```rust
let python = Python::new(&PythonConfig::from_env(), workspace.clone())?;
for tool in python.tools() { registry.register(tool)?; }
// the Python FileObserver, handed to `FileList::new(workspace, observers)`
let observers: Vec<Arc<dyn FileObserver>> = vec![python.observer()];
let health = python.client().health().await; // for `check`
```

Without `PYTHON_URL` the tools answer `BACKEND_UNAVAILABLE`; they never fail to
build.

## Testing

`cargo test -p supermath-python` runs everything against a loopback stub that
replays answers recorded from the real service (`tests/fixtures/`). The live
tests (`live_python_run`, `live_python_search_counterexample`,
`live_python_isolation`) run only when `PYTHON_URL` is set:

```sh
docker compose --profile host up -d --build python python-gateway
PYTHON_URL=http://localhost:3160 SUPERMATH_PROJECTS=<the directory mounted at /projects> \
    cargo test -p supermath-python --test live
```

To re-record a fixture, start the service over a scratch projects directory and
`curl` the route (see the HTTP API in the spec).
