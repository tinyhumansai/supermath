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
is `unknown`. A Lean file the workspace lists but cannot read (over the size
cap, not UTF-8) cannot be checked either: it is listed after the checked
files with verdict `unknown` and the read error as its `reason`, counted in
`summary.unknown` (so the outcome is `unknown`), and named in one `unknowns`
note: `<n> Lean file(s) could not be read and were not checked: <paths>`.

Paged (`limit`, `cursor`) with `total` and a `summary` of counts over the
whole build. `limit` defaults to 10 (maximum 50, and never more than the
call's `rows` budget). The `cursor` is the decimal offset of the next row;
a numeric cursor at or past `total` returns an `ok` page with no files and
no cursor.

## Errors

`INVALID_INPUT` for a bad `cursor`; `BACKEND_UNAVAILABLE` when the Lean
service is down.
