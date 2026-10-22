# supermath-lean

The eight `lean_*` tools, the checking API behind them, and the Lean file
observer. Lean is the toolkit's arbiter: every tool works on a file in the
project's Lean package (`supermath-files`), never on source passed as an
argument, and every answer names a path and a digest anyone can re-check. The
spec is [`docs/specs/toolkit/lean/`](../../docs/specs/toolkit/lean/README.md);
the service it talks to is [`docker/lean/`](../../docker/lean/README.md).

## Public surface

| Module | Exports |
| --- | --- |
| `tools` | `Lean` (builds everything), `LeanCheck`, `LeanSearch`, `LeanDecl`, `LeanGoals`, `LeanTryTactics`, `LeanSearchCounterexample`, `LeanBuild`, `LeanBlueprint`, `ClaimIndex` |
| `checker` | `LeanChecker` (`check`, `verify`, `is_current`), `CheckOutcome` |
| `verdict` | `Verdict`, `UnknownReason`, `CheckReport`, `Provenance`, `Diagnostic`, the axiom allowlist constants |
| `client` | `LeanClient` and the service's request and reply types |
| `hosted` | `HostedSearch`, `Index` (Loogle, `LeanSearch`) |
| `observer` | `LeanObserver` (a `FileObserver` for `file_list`) |
| `declarations` | `declarations`, `Declaration` (qualified names a file declares) |
| `config` | `LeanConfig` (`LEAN_URL`, `LOOGLE_URL`, `LEANSEARCH_URL`) |

## Composing it

```rust,ignore
let lean = Lean::new(LeanConfig::from_env(), workspace.clone())?;
registry.extend(lean.tools());
file_list_observers.push(lean.observer());     // Arc<dyn FileObserver>
let checker = lean.checker();                  // the ledger's LeanEvidence adapter wraps this
```

`LeanChecker::verify(slug, &path, "Decl.name", timeout_ms)` returns a
`CheckReport`: the `Verdict` and the `Provenance` (toolchain, Mathlib
revision, path, file and imports digests, per-target axioms, native_decide
debt). `LeanChecker::is_current(slug, &provenance)` is the staleness test.
`Lean::with_claim_index` supplies `lean_blueprint` with the ledger's claim
ids; the ledger does not depend on this crate, so `supermath-mcp` implements
both ports.

## Design constraints

- Verdict reduction is a pure function of the service's reply
  (`verdict::reduce`), with one unit test per row of the verdict table.
- A verdict is cached in memory by project and path, valid while the file
  digest, the imports digest, and the target list are unchanged; unknown
  verdicts are never cached. The observer reports the last verdict and the
  digest it was for, and a digest no file can have once an import changed.
- `unknown` is never a negative answer: a timeout is a `TIMEOUT` error whose
  `data.verdict` is `unknown`; running out of memory and a budget hit are
  `unknown` outcomes. A counterexample search that covered its whole range
  and found nothing is the exception that proves it: `ok` with
  `data.result: "exhausted"` and the frame.
- Without `LEAN_URL` every tool that needs the service answers
  `BACKEND_UNAVAILABLE`. `lean_search` `pattern` and `natural` modes call the
  hosted indexes from this process; the service has no network.
- Wall-time budgets are the tools' own (`LeanConfig`); the call context's
  budget limits bytes and rows.
- No tool takes Lean source; `tests/schemas.rs` walks every schema.

## Tests

Unit tests are beside the code (`src/*/test.rs`). `tests/` runs the tools
through the public API against a loopback stub (`tests/support`) replaying
service answers recorded in `tests/fixtures` (hand-written to the HTTP API in
the spec, in the shape `docker/lean/normalize.py` produces from the REPL's
JSON; none came from a live Lean here). `tests/live.rs` has `live_lean_check`,
which skips unless `LEAN_URL` and `SUPERMATH_PROJECTS` (the directory the
service mounts at `/projects`) are both set. `tests/compose.rs` asserts the `lean`
service has no network in `compose.yaml`.
