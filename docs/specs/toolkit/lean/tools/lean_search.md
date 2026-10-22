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
