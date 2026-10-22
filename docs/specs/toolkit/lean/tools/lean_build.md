# `lean_build`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-lean` | [Lean](../README.md) | primitive | deferred | read | P6 |

**Given → returns:** the project package → per-file verdicts.

Checks every Lean file of the package in dependency order and returns a
verdict per file, as [`lean_check`](lean_check.md) would, plus
`depends_on_sorry`: every declaration whose axioms include `sorryAx` although
its own text has no `sorry`, so the `sorry` is in another file. (The service
runs `lake build` on the modules a file imports before checking it.) Files
whose digests and imports have not changed reuse their cached verdicts.
Budget: 15 minutes per build; files not reached are `unknown` and the outcome
is `unknown`. Paged (`limit`, `cursor`) with a `summary` of counts.

## Errors

`INVALID_INPUT` for a bad `cursor`; `BACKEND_UNAVAILABLE` when the Lean
service is down.
