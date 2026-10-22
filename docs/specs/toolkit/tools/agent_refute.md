# `agent_refute`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-agent` | [Sub-agents](../agents.md) | agentic (OpenHuman) | deferred | read | P5 |

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
| Tools in the turn | `lean_write`, `lean_edit`, `lean_check`, `calc_search_counterexample`, `lit_oeis`, Jacobian's `math.find` and `math.run` over MCP when connected |
| Budget | 24 iterations, 200,000 tokens, 600 s by default |
| Prompt | state the search frame first; write the negation as a decidable predicate in a project file; search small cases exhaustively before clever ones |

## Returns

- **Verified after the turn:** a witness counts only if
  `calc_search_counterexample` produced it and its witness file
  [`lean_check`](lean_check.md)s as `checked`, re-run by the tool. It is
  recorded as a `refutation` with `refutes` set and status `checked`.
- **Not found:** `outcome: "unknown"` with every frame searched, recorded
  as a `bound`, so "none found" always says how far the search went.
- **Passed through, as a `lead`:** when a counterexample is found, the
  child's proposed nearest repaired statement
  ([`strategies/03-failure.md`](../../../research/strategies/03-failure.md)).
- `usage` with the turn's model, tool calls, tokens, and time.

## Errors

`NOT_FOUND` for an unknown claim; `BUDGET_EXCEEDED` when the project's
budget cannot admit a turn.
