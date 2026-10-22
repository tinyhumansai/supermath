# `agent_review_alignment`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-agent` | [Sub-agents](../agents.md) | agentic (OpenHuman) | deferred | read | P4 |

**Given → returns:** a declaration and its informal statement → alignment.

Runs an OpenHuman agent turn that judges whether a Lean statement says what
the informal statement says. A kernel cannot check that; a second reader
can.

```json
{ "claim_id": "clm_7f3a…" }
```

- `claim_id` names a ledger `result` with a `lean` source; the informal
  statement is the fact it formalizes. `{declaration, statement}` may be
  given instead.

## Turn

| | |
| --- | --- |
| Model | `SUPERMATH_REVIEWER_MODEL` (default `reasoning`), a different rung from the formalizer's |
| Tools in the turn | `lean_decl`, `lean_search`, `lit_get_paper`, `recall_memory` |
| Budget | 12 iterations, 100,000 tokens, 300 s by default |
| Prompt | compare quantifiers, hypotheses, degenerate cases, and conventions; list every hypothesis of the informal statement and where it appears in Lean |

The child **never sees the proof**: it has `lean_decl`, which returns
signatures without proofs, and no `lean_read`, `lean_goals`, or file
access. It starts in a fresh context with no access to the formalizer's
session ([`strategies/04-trust.md`](../../../research/strategies/04-trust.md)).

## Returns

- `accepted` or `disputed`, and for `disputed` the specific mismatch.
- The tool, not the child, sets the claim's alignment in the ledger, with
  the reviewer's model and session.
- `usage` with the turn's model, tool calls, tokens, and time.

A model's review is not evidence the way a kernel check is; the ledger
records it as the reviewer's judgment. A reply that is neither `accepted`
nor `disputed` leaves alignment `not_reviewed` and returns `unknown`.

## Errors

`NOT_FOUND` for an unknown claim or declaration; `INVALID_INPUT` for a
claim with no `lean` source.
