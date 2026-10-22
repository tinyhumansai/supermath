# `lean_search`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-lean` | [Lean](../README.md) | primitive | deferred | read | P2 |

**Given → returns:** a name, pattern, or phrase → Mathlib declarations.

Given a query and a mode, returns Mathlib declarations.

| Mode | Backend | Example |
| --- | --- | --- |
| `name` | the REPL's environment, by name fragment | `Nat.Prime.two_le` |
| `pattern` | [Loogle](https://loogle.lean-lang.org) | `(?a + ?b) ^ 2` |
| `natural` | a natural-language index such as [LeanSearch](https://leansearch.net) | `sum of two squares` |

Each row is `{name, type, module}`, with types trimmed to 300 characters,
at most `limit` rows (default 10, maximum 50, and never more than the call's
`rows` budget).

`coverage` names the backend. In `name` mode it also names the Mathlib
revision searched, which the service reports. Loogle and LeanSearch do not
report the Mathlib revision they index — neither API returns one — so their
`coverage` names the index alone and every answer from them carries exactly
one note in `unknowns`: `<index> does not report the Mathlib revision it
indexes, which may differ from the pinned one`.

`name` mode needs the Lean service; `pattern` and `natural` call the hosted
indexes from the agent (the service has no network).

## Errors

`INVALID_INPUT` for an empty or over-long query, an unknown mode, or a query
Loogle rejects; `BACKEND_UNAVAILABLE` when the service (name mode) or the
hosted index is down; `RATE_LIMITED` when a hosted index asks us to slow down
(`retry_after_ms`: the index's `Retry-After`, 5 s when it sends none; a
`503` counts as a request to slow down); `TIMEOUT` when the index or the
service is too slow, or when the service's name search hits its memory or
output limit.
