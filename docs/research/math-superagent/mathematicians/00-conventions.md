# How this study was done, and what its tags mean

Ten mathematicians studied the way [`research/tao/`](../tao/) studied Terence
Tao: for the *sequence of moves* that produces a solution, not the mathematics
of it, and always ending in something a runtime could be built to do.

Tao is the eleventh member of this set and the first that was done. Read
[`../tao/01-meta-method.md`](../tao/01-meta-method.md) first — the schema here
descends from it, and every file in this directory cites it.

## Why ten more

A runtime fitted to one mathematician acquires that mathematician's taste along
with their method, and has no way to notice which is which. Tao is a virtuoso of
*tricks*: cheat strategically, turn off nine of the ten difficulties, try the
stupidest thing you can think of because the way it fails is informative. Five
things were built on this branch out of reading him, and all five are trick-side
— `weakener` lowers the target, `refuter` attacks the statement, `searcher`
evolves a program toward a score.

Grothendieck would have built none of them. His stated method is to refuse the
local attack and enlarge the theory until the theorem falls out of it, and his
own account names striking harder at the chisel as the thing he would not do
even when the remaining work was, in his words, finished in principle. Thurston
would say the runtime is optimising the wrong output entirely. Arnold would say
`lean_check` installed the wrong authority. Zeilberger would say `Status::Proved`
and `Status::Formalised` are the wrong two categories and the interesting one is
missing.

The value of the second round is those disagreements. Where all eleven agree,
the runtime has a requirement. Where they split, the runtime has a *setting* it
did not know it had chosen — and this repository's recurring failure is exactly
that: a decision that was made by default, recorded nowhere, and defended by a
prompt. [`12-cross-cutting.md`](12-cross-cutting.md) is organised by those
splits rather than by person, and is the file to read if only one is read.

## The set, and the axis each subject holds

| File | Subject | Axis they hold that Tao does not |
|---|---|---|
| [`01`](01-grothendieck.md) | Grothendieck | Rising sea: enlarge the theory until the theorem is a triviality. The anti-trick pole |
| [`02`](02-erdos.md) | Erdős | Problem *generation* at scale; prizes as a priority signal; probabilistic existence; collaboration as search |
| [`03`](03-gowers.md) | Gowers | Explicit algorithmic meta-mathematics, written by someone who wants it automated |
| [`04`](04-wiles.md) | Wiles | The long solitary siege, and the recovery from a collapse seven years in |
| [`05`](05-thurston.md) | Thurston | Understanding versus a verified string — what a run should actually emit |
| [`06`](06-zeilberger.md) | Zeilberger | Certificates over proofs; semi-rigorous as a legitimate status |
| [`07`](07-ramanujan.md) | Ramanujan | Conjecture generation from numerical experiment, with proof deferred or absent |
| [`08`](08-scholze.md) | Scholze | Find the category in which the problem is easy — the rising sea with a formalisation practice attached |
| [`09`](09-arnold.md) | Arnold | Geometric and physical intuition; cross-domain transfer; open hostility to formalism |
| [`10`](10-perelman.md) | Perelman | Completing a programme someone else stalled, by supplying the one missing estimate |

[`11-harness-inventory.md`](11-harness-inventory.md) is what they are compared
against: this runtime's capabilities at the current branch HEAD, every claim
carrying a `file:line`. It replaces
[`../tao/03-harness-inventory.md`](../tao/03-harness-inventory.md), which was
written at commit `55e14efd` and predates `lean.rs`, `search.rs`, `refute.rs`,
`weakened.rs`, `closure.rs` and `blueprint.rs`.

## The schema every subject file uses

Fixed, so that ten files are comparable and so that
[`docs/methods-gap-analysis.md`](../../docs/methods-gap-analysis.md) can cite
`04`§A3 the way the Tao gap analysis cites `01`§16.

1. **Header** — who, the axis, and short source keys with full URLs.
2. **Accuracy conventions** — what is verbatim, what is summarised, what is
   apocryphal. Never omitted. See below.
3. **§A Stated method** — numbered entries, each tagged `[STATED]` or
   `[INFERRED]`, each in three parts: the idea in prose, the quote with a source
   key, then a bold **Agent:** paragraph naming a requirement on *this* runtime.
4. **§B Anatomy** — two to four results dissected under the six-field schema of
   [`../tao/02-proof-anatomy.md`](../tao/02-proof-anatomy.md): **(a)** as posed,
   **(b)** the reframing that unlocked it, **(c)** imported machinery and its
   source field, **(d)** what was computed or verified, **(e)** programme length
   and the ladder of prior partial and no-go results, **(f)** the transferable
   **MOVE** as a firing rule with a trigger, an action, and a safety check.
5. **§C Against Tao** — the explicit tension, naming which of the 36 heuristics
   in `../tao/01` this subject contradicts and when a run should follow which.
6. **Sources** — URLs. Anything at or past the assistant's knowledge cutoff is
   marked `[UNVERIFIED]`.

`[STATED]` means the subject said it about method. `[INFERRED]` means it is read
off their practice, or is a principle they apply without naming. The distinction
is load-bearing: an `[INFERRED]` entry is this document's opinion, and a
proposal resting only on inferred entries should say so.

## Accuracy conventions

These apply to every file here and are the reason the Tao corpus is worth
trusting. [`../tao/01`](../tao/01-meta-method.md) opens by demolishing the
widely-repeated "5% chance" quote — no such Tao statement exists — and that
section has been more useful than most of the heuristics below it.

- **Quotation marks mean reproduction.** A sentence inside quotation marks was
  reproduced from a fetched source, and the source key is on the same line. If a
  page could only be reached through a summarising fetch, the entry says
  *summarised* and does not present the words as the subject's.
- **A widely repeated line with no traceable origin is named and not used.** It
  goes in the accuracy section as apocryphal, so the next session does not spend
  an hour rediscovering that it is unsourceable.
- **Translation is flagged.** Grothendieck, Arnold and Perelman are read here
  mostly in translation, and the translator is named where the wording carries
  the argument.
- **Second-hand method claims are attributed to the person making them.** Serre
  on Grothendieck's originality and Deligne on the shape of a Grothendieck proof
  are evidence about Grothendieck, and are cited as Serre's and Deligne's words.

Four subjects are surrounded by folklore that reads like quotation — Erdős,
Ramanujan, Grothendieck and Perelman — and their files carry the longest
accuracy sections for that reason.

**Where a subject is thin on stated method**, the file says so and leans on §B.
Ramanujan and Perelman wrote almost nothing about how they worked; manufacturing
§A entries for them would be inventing the finding rather than reporting it. A
short §A with a stated reason is the honest shape.

## What this round does not do

It writes research and proposals. It builds nothing and closes no gap, so
[`docs/methods-gap-analysis.md`](../../docs/methods-gap-analysis.md) has no
`[closed]` rows, unlike its Tao counterpart. The status vocabulary is otherwise
the same and is worth restating because it is the whole point of the exercise:

- **Absent** — nothing in the runtime does this.
- **Unenforced** — a prompt asks for it. A prompt instruction is not a control,
  so an unenforced rule is a rule the code stopped guaranteeing.
- **Unused** — the code produces it and nothing reads it.
- **Partly**, **Present** — as they read.
