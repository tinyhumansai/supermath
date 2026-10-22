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

Each row is `{name, type, module}`, with types trimmed to 300 characters.
`coverage` names the backend and the Mathlib revision it indexes; a hosted
backend on a different revision than ours is noted in `unknowns`.

`name` mode needs the Lean service; `pattern` and `natural` call the hosted
indexes from the agent (the service has no network).

## Errors

`INVALID_INPUT` for an empty or over-long query, an unknown mode, or a query
Loogle rejects; `BACKEND_UNAVAILABLE` when the service (name mode) or the
hosted index is down; `RATE_LIMITED` when a hosted index asks us to slow down
(`retry_after_ms`); `TIMEOUT` when it is too slow.
