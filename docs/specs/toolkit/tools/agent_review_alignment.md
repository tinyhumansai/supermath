# `agent_review_alignment`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-agent` | [Sub-agents](../agents.md) | deferred | read | P4 |

**Given → returns:** a declaration and its informal statement → alignment.

Given a Lean declaration and the informal statement it is meant to state,
judges whether they say the same thing.

- **Child tools:** `lean_decl`, `lean_search`, `lit_get_paper`,
  `recall_memory`. No write tools and no `lean_goals`.
- The child sees the statement and the definitions it uses, **never the
  proof**, and starts in a fresh context with no access to the formalizer's
  session. It checks quantifiers, hypotheses, degenerate cases, and whether
  every hypothesis of the informal statement appears
  ([`strategies/04-trust.md`](../../../research/strategies/04-trust.md)).
- **Returns:** `accepted` or `disputed`, and for `disputed` the specific
  mismatch. The harness sets the claim's alignment from that.

A model's review is not evidence the way a kernel check is; the ledger
records it as the reviewer's judgment, with the reviewer's model and
session.
