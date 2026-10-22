# `lean_try_tactics`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | deferred | read | P4 |

**Given → returns:** source, a `sorry`, tactics → per-tactic outcome.

Given source, the index of one `sorry`, and up to 12 tactics, returns per
tactic one of `closes`, `progress` (with the remaining goals), or `fails`
(with the error). The default list, when none is given, is `rfl`,
`norm_num`, `simp`, `omega`, `decide`, `linarith`, `nlinarith`, `positivity`,
`aesop`, and `exact?`. A tactic that closes the goal is returned with the
term `exact?` found, when it found one.

Tactics run in the REPL's tactic mode against the `sorry`'s proof state, in
parallel, each under its own timeout (10 s by default).
