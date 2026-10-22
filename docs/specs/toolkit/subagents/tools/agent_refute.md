# `agent_refute`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-subagents` | [Sub-agents](../README.md) | agentic (OpenHuman) | deferred | read | P6 |

**Given → returns:** a statement → a counterexample, or a repaired statement.

Runs an OpenHuman agent turn that looks for a counterexample before anyone
tries to prove the statement.

```json
{ "claim_id": "clm_7f3a…", "budget": { "wall_secs": 900 } }
```

## Turn

| | |
| --- | --- |
| Model | `SUPERMATH_FORMALIZER_MODEL` |
| Tools in the turn | `file_write`, `file_edit`, `file_read`, `py_solvers`, `py_run`, `py_search_counterexample`, `lean_check`, `lean_search_counterexample`, `lit_oeis` |
| Budget | 24 iterations, 200,000 tokens, 600 s by default |
| Prompt | state the search frame first; search broadly in Python (exhaustive small cases, then SAT, CP-SAT, or numeric search), then confirm any witness in Lean as a decidable predicate |

## Returns

- **Verified after the turn:** a witness found in Python is only a lead.
  It counts only if `lean_search_counterexample` produced it and its
  witness file
  [`lean_check`](../../lean/tools/lean_check.md)s as `checked`, re-run by the tool. It is
  recorded as a `refutation` with `refutes` set and status `checked`.
- **Not found:** `outcome: "unknown"` with every frame searched, by
  `py_search_counterexample` or in Lean, recorded as a `bound` with its
  call as the source, so "none found" always says how far the search went.
- **Passed through, as a `lead`:** when a counterexample is found, the
  child's proposed nearest repaired statement
  ([`strategies/03-failure.md`](../../../../research/strategies/03-failure.md)).
- `usage` with the turn's model, tool calls, tokens, and time.

## Errors

`NOT_FOUND` for an unknown claim; `BUDGET_EXCEEDED` when the project's
budget cannot admit a turn.
