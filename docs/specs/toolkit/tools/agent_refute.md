# `agent_refute`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-agent` | [Sub-agents](../agents.md) | deferred | read | P5 |

**Given → returns:** a statement → a counterexample, or a repaired statement.

Given a statement, looks for a counterexample before anyone tries to prove
it.

- **Child tools:** `calc_search_counterexample`, Jacobian over MCP,
  `lean_check`, `lit_oeis`, `ledger_record`, `ledger_bound`.
- The child states the search frame first (what is searched, over what
  range) and records it, so "nothing found" says how far it looked.
- **Returns:** a witness, re-checked by the tool with `lean_check` on a
  `decide`-able instance where possible; or "none found in frame" with the
  frame, as `unknown`. When it finds a counterexample it proposes the
  nearest repaired statement as a `lead`
  ([`strategies/03-failure.md`](../../../research/strategies/03-failure.md)).
