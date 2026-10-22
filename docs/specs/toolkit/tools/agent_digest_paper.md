# `agent_digest_paper`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-agent` | [Sub-agents](../agents.md) | agentic (OpenHuman) | deferred | read | P4 |

**Given → returns:** an arXiv id → facts recorded from it.

Runs an OpenHuman agent turn that reads a paper and records what a later
turn will need. The caller gets the ledger ids, not the paper: the reading
stays in the child's context.

```json
{ "arxiv_id": "2012.04125", "focus": "the bound on the critical radius" }
```

- `focus` optionally narrows what the child records; without it, the
  paper's main results and definitions.

## Turn

| | |
| --- | --- |
| Model | `SUPERMATH_READER_MODEL` (default `flash`) |
| Tools in the turn | `lit_get_paper`, `lit_extract_statements`, `lit_citations`, `ledger_record`, `ledger_link`, `recall_memory` |
| Budget | 24 iterations, 200,000 tokens, 600 s by default ([`agents.md`](../agents.md#budgets)) |
| Prompt | read the statement list first, then only the sections the main results need; record one entry per finding; cite every entry by label |

The child records:

- each definition and main result as a `fact` citing its label;
- the techniques the proofs use;
- the open problems and conjectures the paper states, as `lead`s;
- the paper's own dependency edges, as `uses`.

## Returns

- **Verified after the turn:** the tool reads back every id the child
  recorded with [`ledger_claims`](ledger_claims.md) and returns those that
  exist, grouped by kind. An id the child claims but the ledger lacks is
  dropped and counted in `unknowns`.
- **Passed through, as untrusted prose:** a one-paragraph account of what
  the paper proves, at most 120 words.
- **Discarded:** everything else the child read or said. Its session id is
  returned for inspection.
- `usage` with the turn's model, tool calls, tokens, and time.

Every fact it records is `cited`, never higher: reading is not checking.

## Errors

`INVALID_INPUT` for a malformed id; `NOT_FOUND` when arXiv has no such
paper (checked before the turn starts); `BUDGET_EXCEEDED` when the
project's budget cannot admit a turn. A turn that runs out of budget is
`outcome: "unknown"` with whatever was recorded.
