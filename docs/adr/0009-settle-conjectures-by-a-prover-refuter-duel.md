# 9. Settle conjectures by a prover–refuter duel

- **Status:** Proposed
- **Date:** 2026-10-22
- **Spec:** [`agent_settle`](../specs/toolkit/subagents/tools/agent_settle.md)

## Context

To settle a conjecture we have to either prove it or find a counterexample,
and before we start we can't tell which one is possible. The toolkit has a
tool for each direction, `agent_formalize` and `agent_refute`, but they are
separate one-shot calls. The top-level agent has to choose one to run first,
and whichever it chooses gets most of the effort. The strategy notes count
that ordering as a mistake. `strategies/01-intake.md` §5 asks for the
refutation attempt to run "before or alongside the proof turn", on the same
budget, so that whichever side finishes first ends the other. The same note
also argues that two turns with separate contexts "cannot share a bias the
way one turn arguing both sides does".

OpenHuman can now steer a running turn (`TurnSteering`, which adds a user
message at the next safe checkpoint) and cancel one (`TurnCancellation`).
That means one host can run two agents at once, carry messages between them,
and stop either of them.

## Decision

- `agent_settle` runs two OpenHuman agents at the same time on one
  aligned Lean conjecture: a **prover** and a **refuter**. They have the same
  model and the same budget, and the budget is never moved from one side to
  the other.
- Both sides attack **one pinned proposition**, `Conjecture`, which the host
  writes from the claim's reviewed declaration. A win is a Lean theorem of
  `Conjecture` or of `¬ Conjecture` that the host re-checks itself in a file
  it writes. A side's reply is never a verdict.
- Both sides share one project workspace, but each **writes only to its own
  area**. Reads are open to both, so each side can import the other's
  checked lemmas.
- The sides talk only through **typed posts that the host checks**. A
  `lemma` must pass `lean_check` and a `bound` must match an exhausted search
  before the partner sees it as verified. Everything else is relayed
  labelled as an unverified lead. Posts are delivered by steering, batched
  and rate-limited.
- The host keeps each side **on task** using facts from the side's own
  journal: it nudges a side that makes no progress and cancels a turn that
  keeps drifting. The side then continues in a fresh turn whose prompt the
  host writes.
- The first verified win cancels the other side. If both sides verify, the
  result is `inconsistent` and nothing is recorded.

## Consequences

- One call does both reconnaissance and attack, and its result says which
  side won. Exhausted searches are always recorded as bounds, so an
  `unknown` result still says how far the search went.
- A duel costs two agents' budgets for one answer. The split is equal on
  purpose. Moving the budget to the side that looks closer would need a
  measure of closeness that we do not trust yet, so we do not do it. This is
  recorded in configuration, as `strategies/03-failure.md` §5 asks.
- A statement has to be formalised and reviewed before it can be settled.
  `agent_settle` does neither step itself.
- Two concurrent agents put twice the load on the Lean and Python services.
  Their write areas cannot overlap, so `if_sha256` conflicts between the two
  sides cannot happen. Each side can still conflict with its own stale reads.
