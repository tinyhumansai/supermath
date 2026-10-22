# `agent_digest_paper`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-agent` | [Sub-agents](../agents.md) | deferred | read | P4 |

**Given → returns:** an arXiv id → facts recorded from it.

Given an arXiv id, reads the paper and records what a later turn will need.

- **Child tools:** `lit_get_paper`, `lit_extract_statements`, `lit_citations`,
  `ledger_record`, `ledger_link`, `recall_memory`.
- **Records:** each definition and main result as a `fact` citing its label;
  the techniques the proofs use; open problems and conjectures the paper
  states, as `lead`s; the paper's own dependency edges as `uses`.
- **Returns:** the ids recorded, grouped by kind, and a one-paragraph
  account of what the paper proves, at most 120 words.

Every fact it records is `cited`, never higher: reading is not checking.
