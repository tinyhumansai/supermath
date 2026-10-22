# `agent_review_alignment`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-subagents` | [Sub-agents](../README.md) | agentic (OpenHuman) | deferred | read | P5 |

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
| Model | `SUPERMATH_REVIEWER_MODEL` (default `glm-5.3`), a different model from the formalizer's: `Models` refuses an equal pair with `Error::ReviewerIsFormalizer` |
| Tools in the turn | `lean_decl`, `lean_search`, `lit_get_paper`, `recall_memory` |
| Budget | 12 iterations, 100,000 tokens, 300 s by default |
| Prompt | compare quantifiers, hypotheses, degenerate cases, and conventions; list every hypothesis of the informal statement and where it appears in Lean |

The child **never sees the proof**: it has `lean_decl`, which returns
signatures without proofs, and no `file_read`, `lean_goals`, or file
access. It starts in a fresh context with no access to the formalizer's
session ([`strategies/04-trust.md`](../../../../research/strategies/04-trust.md)).

## Returns

- `accepted` or `disputed`, and for `disputed` the specific mismatch, as
  untrusted prose (at most 100 words).
- The tool, not the child, records the review. `Ledger::set_alignment`
  exists in `supermath-ledger` but no tool exposes it, `ledger_record`
  refuses an `alignment` argument, and this crate may not depend on the
  ledger crate, so for a `claim_id` the tool records a `fact` that
  `uses` the claim ("Alignment review of …: accepted"), with a `supplied`
  source naming the reviewer's model and session, and returns its id as
  `review_id`. The claim's own `alignment` therefore stays `not_reviewed` in
  the ledger; the review is in the fact and in this answer's `alignment`.
  When a tool exposes the setter, this tool calls it.
- **Verified after the turn:** when the declaration lives in a project file,
  the tool re-runs [`lean_check`](../../lean/tools/lean_check.md) on it
  (`path` is the ledger claim's `lean.path`, or `lean_decl`'s `data.path`,
  present only for a declaration in the project package). A
  file that does not compile is never aligned, whatever the reviewer said:
  the answer is `unknown`, alignment stays `not_reviewed`, nothing is recorded,
  and `lean_verdict` reports Lean's word.
- `usage` with the turn's model, tool calls, tokens, and time.

A model's review is not evidence the way a kernel check is; the ledger
records it as the reviewer's judgment. A reply that is neither `accepted`
nor `disputed` leaves alignment `not_reviewed` and returns `unknown`.

## Errors

`NOT_FOUND` for an unknown claim or declaration (`lean_decl` is asked with
`{name}` before the turn); `INVALID_INPUT` for a claim with no `lean` source,
for arguments that are neither `{claim_id}` nor `{declaration, statement}`, or
a `budget` out of range; `BUDGET_EXCEEDED` when the project's budget cannot
admit a turn; `FORBIDDEN` for a call from another project than the one the
tool serves; `BACKEND_UNAVAILABLE` when the ledger or Lean cannot be read or
the child's model backend fails.
