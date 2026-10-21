# ProofAtlas, read as a harness

proofatlas.ai publishes AI-produced mathematics: five completed results, 24 Lean
formalizations, 85 open workspaces. It is the closest public thing to what this
repository is, which makes it the only external answer we have to the question
this repository keeps asking — *what does a run have to produce before anyone
should believe it.*

These notes are not about the mathematics. Each takes a class of artifact apart
and asks what control produced it, what failure that control stops, and what it
would cost here. `01` came first and dissects one downloaded bundle; `02`–`06`
were read off the live site on 17 August 2026.

| Note | Artifacts read | The question it answers |
| --- | --- | --- |
| [`01-sendov-bundle-anatomy.md`](01-sendov-bundle-anatomy.md) | the 3.4 MB Sendov release zip, 1,272 files | what a finished proof package contains |
| [`02-refutation-path.md`](02-refutation-path.md) | Jackson 12-vertex counterexample; Berlekamp Domineering | how a **disproof** is stated, witnessed and checked |
| [`03-collatz-decomposition.md`](03-collatz-decomposition.md) | 5 Collatz results in one Lean package | how an unattackable conjecture is cut into publishable parts |
| [`04-known-theorem-bench.md`](04-known-theorem-bench.md) | Sylvester–Gallai, Matrix-Tree, Hook-Length, Brooks, the 74-line tail, the Mathlib landmark index | the calibration bench, and statement fidelity |
| [`05-open-workspace-shape.md`](05-open-workspace-shape.md) | Hilbert 16, Agoh–Giuga, Dürer, Gallai | work in progress: obligations, dead routes, onboarding |
| [`06-trust-model.md`](06-trust-model.md) | the grades, the review lanes, the Sendov blocker | what moves a result between grades |

## The one thing to read if you read nothing else

Sendov is a 1,176-file Lean project with zero `sorry`, zero `axiom`, a printed
axiom list of `[propext, Classical.choice, Quot.sound]`, and 1,010 uses of
`decide +kernel` against zero `native_decide`. The site does not believe it.

Its stated blocker is not mathematics:

> the retained audit is a summary rather than the complete build transcript
> required by the current evidence contract

and in the machine-readable record, `buildTranscriptRecorded: false`,
`collectionProvenanceRecorded: false`, `recordedBuildElapsedMs: 0`,
`collectedAt: null`, `collectorCommit: null` — against `true / true / 2278 /
timestamp / commit` on the accepted Collatz result. Stacked on top,
`statementAlignmentStatus: not_reviewed`: nobody has certified that the 20-line
`Theorem.lean` says Sendov's conjecture.

**A transcript you were given is a claim. A transcript you collected is
evidence.** That distinction is the single most transferable thing on the site,
and we do not currently draw it: our `lean_check` result and a result a role
pasted into a claim are the same shape once filed.

## What converged, ranked

Each of the five notes ends with its own proposals, costed and located. What
follows is only what **three or more notes arrived at independently** — the
convergence is the argument, not any one note.

### 1. A claim must state what it does *not* cover — naming its neighbours

Reached by `02` (a `does-not-cover` list is one of seven bars for believing a
disproof), `03` (boundary is a machine-readable `boundary` string plus a
`nonClaims` array, rendered in three places from one source), and `04` (the
hook-length gap — the Lean endpoint is `∏h · f^μ = |μ|!` and the familiar
`|μ|!/∏h` form "is an interpretation rather than the literal statement").

The sharp detail is that their boundaries name *specific adjacent statements in
the same package*, not generic disclaimers: five of the natural-density result's
six non-claims are about confusion with its own siblings. Boundary length tracks
confusability, not fame.

We have no field for this. A claim that reads as complete is the failure this
repository already writes controls against everywhere else — a cut ledger
section must say what it left out, but a cut *result* need not.

**Where:** a required field on the claims ledger, and on `reductions` where the
two bounds already make the gap visible. **Cost:** ~1 day, mostly the render.

### 2. Statement alignment is a separate gate, judged by someone who did not write the Lean

Reached by `02` (statement-alignment is one of four separately reviewed lanes,
and the load-bearing choice in Jackson was refusing to fix `k = 3` in
`HasHamiltonDecomposition` — 109 lines to derive `k = 3` for arbitrary `k`
rather than take the free weakening), `04` (fidelity bought three ways, ranked:
Mathlib's own vocabulary, a bridge lemma, or a disclosed gap), and `06` (Sendov's
second blocker).

We are half-way here already — `lean_prover` decides "whether the Lean means the
mathematics" and `lean_scribe` writes it. What we lack is that the judgement is
**recorded separately from the check**, so `checked` and `aligned` can disagree
and a run can be blocked on the second with the first green.

**Where:** split the verdict; `lean_prover` may not align a statement it filed
in the same attempt. **Cost:** ~1 day, plus a routing threshold.

### 3. Evidence carries its collector, or it is a claim

Reached by `06` (the provenance booleans above), `05` (a three-rung reproduction
ladder — `reported unreproduced` → `reproduced same implementation` →
`independently reimplemented` — where an unreproduced input marks every
downstream derivation `challenged`, and replay is itself a top-priority
obligation), and `03` (five stages carrying one exponent, with the reserve
recomputed rather than restated).

**Where:** `evidence_origin: collected | supplied`, settable only by the
executing tool, plus toolchain, Mathlib commit, elapsed and per-file hash on
every recorded check. The propagation rule from `05` is the valuable half.
**Cost:** ~2 days; the propagation is the hard part.

### 4. A dead route is an entry, not an absence

Reached by `05` (a seven-value route disposition — active / narrowed /
eliminated / refuted / useful-but-insufficient / held-in-reserve /
not-yet-justified — where `narrowed` records the surviving fragment and
`reserved` the revival condition; plus 37 "useful failures", each one imperative
sentence describing a *move*, deliberately repeated under every cluster it
threatens), `04` (route status and evidence posture as two axes on an approach),
and `02` (the search frame — Berlekamp's witness was missed by prior searches
because it lay outside every search frame, 9×8 against `≤7×7`; more compute in
the same frame would never have found it).

This is the largest genuine gap. Our ledgers record what was established; a run
re-walking a dead route is invisible to them.

**Where:** a `disposition` axis on approaches, and a `search_frame` field
recording what was swept and which published exhaustive regime it lies outside.
`02` rates the search frame the cheapest high-value item on the whole list.
**Cost:** ~1 day for the axis; the frame is hours.

### 5. A rendered count must carry its composition

Reached by `06`, `05` and `04`. The site's front page leads with 271,452
investigation lines and 85 workspaces, and every number carries an inline guard
("not a measure of closeness to a proof", "more nodes or edges do not establish
correctness or completion") plus a composition breakdown. The guard it does not
reach: the scale is monotone — nothing shrinks on being refuted.

We render counts in every ledger index. **Cost:** hours.

## Cheapest concrete fix on the list

`tautologies` in `src/orchestrator/lemmas.rs:780` catches `x = x` and, through
`is_vacuous`, `: True` — but **not** `P ↔ P := by rfl`, which is the exact shape
ProofAtlas ships and records, in 2 of the 7 bundles read. Verified against the
code, not inferred: the proposition is taken as everything after the last
top-level `:` and then `split_once('=')`, so an `↔` proposition never reaches the
comparison at all. The same guard that keeps `≠ ≤ ≥ < > ! :` out of the `=`
split is what an `↔` arm would need. Under an hour, and it closes a hole in a
control we already have.

## What was deliberately not proposed

Each note carries its own refusals; two are worth surfacing here.

- **Their review lanes.** Four named lanes look like the right shape, but every
  reviewer is a model reviewing its own workflow's output with
  `independenceStatus: not_asserted` recorded on each, and each review states
  the reviewer did not rerun Lean. Cloning it buys four more agreeing model
  calls, not evidence. Proposal 2 above captures the useful half.
- **Relaxing the `native_decide` refusal.** They allow it and price it — the
  accepted 0.90 predecessor result discloses two native checks, prints both
  axiom names, says "not described as kernel-clean", and files an open route to
  remove them. What we are missing is not permission but a *grade*: something
  below `formalised` for a replayed, non-kernel-clean certificate, with its
  trust boundary filed as a frontier entry.

Also: no rejected artifact exists anywhere on the site. The lowest outcome
observed is "pending". The scheme has not yet shown it can say no.

## Directly usable, outside the harness

`05` ends with intelligence for the **live Hilbert 16 run** in this checkout —
their reduction `A(n) ≤ H(n) ≤ n²A(n)` on same-core stack height, which our
library does not hold; two negative results, including
`H(n) ≤ ν(n)·b(n)·(B(n)+3)` as the quantified form of our own METHOD warning;
and two peer-reviewed sources we do not have (Gasull–Santana 2024,
doi 10.1090/proc/17116; Gasull 2024 survey, doi 10.1007/s40863-024-00471-2).
Their page is *younger* than our workspace and misses the Yeung 2025 Dulac gap
our claims already carry, so this is an exchange, not a catch-up.
