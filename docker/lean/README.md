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
| `normalize.py` | pure: REPL answers to replies, and the Lean commands sent |
| `test_normalize.py` | `python3 -B -m unittest test_normalize` (no Lean needed) |

Bumping Mathlib: change `rev` in `lakefile.toml` and the toolchain in
`lean-toolchain`, rebuild, commit the new `lake-manifest.json`, and re-check
every `formalised` claim, all in one commit.
