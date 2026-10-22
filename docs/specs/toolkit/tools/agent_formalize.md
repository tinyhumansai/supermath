# `agent_formalize`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-agent` | [Sub-agents](../agents.md) | deferred | read | P4 |

**Given → returns:** a statement or paper locator → Lean declarations and their verdict.

Given a statement or a locator, writes it in Lean.

```json
{
  "source": { "arxiv_id": "2012.04125", "label": "thm:main" },
  "depth": "statement",
  "module": "Supermath/Sendov/Main.lean"
}
```

- `source` is `{arxiv_id, label}` or `{statement}` (informal text) or
  `{claim_id}`.
- `depth: "statement"` produces the theorem statement with a `sorry` proof,
  plus any definitions it needs. `depth: "proof"` also attempts the proof.
- **Child tools:** `lit_get_paper`, `lean_search`, `lean_decl`, `lean_goals`,
  `lean_try_tactics`, `lean_write`, `lean_check`, `ledger_record`,
  `recall_memory`.
- **Child method**, in its prompt: find the Mathlib names first
  (`lean_search`), reuse Mathlib definitions rather than redefining them,
  state the theorem, check that it elaborates, then work each `sorry` with
  `lean_goals` and `lean_try_tactics`. If the statement cannot be written
  from Mathlib primitives, say which definition is missing and stop: that
  is a `formalisation_depth` finding, not a failure
  ([`proofatlas/02-refutation-path.md`](../../../research/math-superagent/proofatlas/02-refutation-path.md)).
- **Returns:** the module, its declarations, and for each the verdict from
  **the tool's own `lean_check`**, run after the child finished; the child's
  claims about its work are ignored. Each declaration is recorded in the
  ledger as a `result` with that verdict, alignment `not_reviewed`, and a
  `uses` edge to the fact it formalizes.
