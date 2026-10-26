# docker/lean

The Lean service: a pinned toolchain and Mathlib, the REPL, and
`server.py`, a stdlib-only HTTP wrapper. The API it serves is in
[`docs/specs/toolkit/lean/README.md`](../../docs/specs/toolkit/lean/README.md#the-http-api);
the decision is [ADR 0003](../../docs/adr/0003-run-lean-as-a-pinned-service.md).

| File | Role |
| --- | --- |
| `lean-toolchain`, `lakefile.toml` | the pin: one Lean, one Mathlib tag |
| `lake-manifest.json` | written by `lake update`; commit it after the first build to pin the full graph |
| `Dockerfile` | toolchain, `lake exe cache get`, the REPL, the wrapper |
| `server.py` | routes and request handling |
| `repl.py` | warm REPL pool, deadlines, output cap |
| `build.py` | compiles a project's own modules with `lean -o`; per-project locks, atomic oleans |
| `normalize.py` | pure: REPL answers to replies, and the Lean commands sent |
| `test_normalize.py` | `python3 -B -m unittest test_normalize` (no Lean needed) |
| `test_build.py`, `test_server_concurrency.py` | build order, staging, and concurrent checks (no Lean needed) |

## Concurrent checks

The server is threaded, so a caller may send several `/check` requests for one
project at once. Staging a project and compiling its `Supermath.*` modules run
under a lock per project (keyed by the real path of its `lean/` directory), so
concurrent requests for the same project build one at a time: a module two
files import is compiled once, no `.olean` is written twice, and a restage
never rewrites sources under a compile. Requests for different projects hold
different locks and build in parallel. The lock covers staging and the build
only; elaborating the file afterwards just reads the finished oleans and runs
concurrently, bounded by `LEAN_POOL_SIZE`. Oleans are also written atomically
(to a `.tmp` file renamed into place), so a failed or timed-out compile leaves
no partial olean. The locks are per process; run one service per
`LEAN_BUILDS_DIR` and projects volume.

Bumping Mathlib: change `rev` in `lakefile.toml` and the toolchain in
`lean-toolchain`, rebuild, commit the new `lake-manifest.json`, and re-check
every `formalised` claim, all in one commit.
