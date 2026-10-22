# `lean_goals`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | deferred | read | P4 |

**Given → returns:** source and a position → the goal state there.

Given source and a position (line and column), returns the goal state there:
hypotheses and targets, pretty-printed. Used to work a proof without
re-sending it for every step.
