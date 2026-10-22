# `lean_blueprint`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | deferred | read | P5 |

**Given → returns:** the project → the declaration DAG with status.

Given the project, returns the dependency graph of its declarations: nodes
with their verdict and ledger claim id, edges for each declaration's use of
another. The answer to "what does the main theorem still rest on" is the
set of `checked_with_sorry` nodes reachable from it. Formalization effort
goes first to the node most other nodes rest on (Scholze's rule,
[`strategies/04-trust.md`](../../../research/strategies/04-trust.md)).
