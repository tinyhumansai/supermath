# ProofAtlas' known-theorem bench, read as a calibration set

Source: `proofatlas.ai/formalizations/`, read 17 August 2026. Twenty-four
published formalizations of theorems that were already true. Four read in full
with every content sub-link (statement page, proof route, evidence record,
`.lean` source, source ZIP): Sylvester–Gallai, Kirchhoff matrix-tree,
hook-length, Brooks. Three small ones read for contrast: no-retraction (233
lines), Heron (86), Napoleon (74). Plus the sibling index of 121 Mathlib
landmarks, and one collaboration page.

[`01-sendov-bundle-anatomy.md`](01-sendov-bundle-anatomy.md) took apart their
one open result. This is the other half of the same harness: what they do when
the answer is already known and the only question is whether the *machinery*
works. That is exactly what `docs/calibration.md` is for here, so the bench is
readable as a competing answer to a question this repository has already asked.

## The bench, measured

Every entry states `N recorded declarations · M first-party Lean files · L
lines`. The full distribution, sorted:

| Lines | Files | Decls | Theorem |
| ---: | ---: | ---: | --- |
| 124,019 | 397 | 2 | Tao, almost-bounded Collatz orbits |
| 92,816 | 1,160 | 1 | Sendov |
| 10,237 | 58 | 3 | Collatz predecessor bounds at 0.90 |
| 8,926 | 40 | 1 | Rhin phase gap for `log₂ 3` |
| 3,692 | 1 | 2 | hook-length formula |
| 3,373 | 1 | 1 | Brooks |
| 3,197 | 1 | 2 | butterfly |
| 2,756 | 1 | 2 | Cayley |
| 2,511 | 20 | 1 | Terras power-saving |
| 2,043 | 1 | 2 | matrix-tree |
| 1,662 | 6 | 1 | 12-vertex Hamilton counterexample |
| 1,625 | 1 | 3 | Sylvester–Gallai |
| 1,386 | 1 | 2 | Dilworth |
| 1,203 | 1 | 3 | Pick, algebraic lemma |
| 1,107 | 2 | 3 | Brianchon |
| 1,070 | 1 | 2 | König edge-colouring |
| 1,032 | 1 | 3 | friendship |
| 698 | 1 | 3 | Euler pentagonal recurrence |
| 357 | 1 | 2 | Wolstenholme |
| 337 | 1 | 3 | Fourier L¹/L² bridge |
| 251 | 1 | 1 | Erdős–Szekeres |
| 233 | 1 | 2 | no-retraction, disk to circle |
| 86 | 1 | 4 | Heron, coordinate identity |
| 74 | 1 | 4 | Napoleon, algebraic core |

Seventeen of twenty-four are a single file. The median is about 1,300 lines. The
line rule is stated rather than assumed — "physical lines are split on LF, CRLF,
or CR", "counted lines contain at least one non-whitespace character",
"comments and documentation count", "external dependencies such as Mathlib are
excluded" — and the definition ships in the JSON beside the number.

Recorded build times, from the seven evidence records read: 2,183 ms (Brooks,
3,373 lines) to 6,709 ms (no-retraction, 233 lines). **Line count is not cost.**
No-retraction is fifteen times smaller than Brooks and takes three times as long
to check, because its import closure reaches `Topology.Homotopy.Lifting`. Every
one of the seven records `leanprover/lean4:v4.29.1`, axiom profile
`classical_mathlib_standard`, axioms `[Classical.choice, Quot.sound, propext]`,
`hasSorryAx: false`, `hasUnexpectedAxioms: false`. Zero `sorry`, zero `axiom`,
zero `native_decide`, and — notably against the Sendov bundle's 1,010 —
**zero `decide` of any kind across all seven files.** There is no certificate
arm on this bench at all. It is entirely argument.

## Recorded declarations: what they are, and what they are not

The count is not proof depth. It is the list in `declarations[]` in the evidence
JSON, and it holds the names the site is willing to point at. Four shapes appear:

1. **the wrapper theorem** — `theorem sylvesterGallai : SylvesterGallaiStatement`;
2. **the `Prop` definition** it names — `def SylvesterGallaiStatement : Prop := …`;
3. **a route lemma** with independent value — `heron_factor_identity`,
   `napoleon_centroid_relation`, `coordinate_heron_factor_identity`;
4. **a tautology** — and this one is worth quoting in full, because both
   Sylvester–Gallai and Napoleon record it as one of their declarations:

```lean
/-- Checked statement wrapper for the minimum theorem. -/
theorem sylvesterGallai_statement :
    SylvesterGallaiStatement ↔ SylvesterGallaiStatement := by
  rfl
```

It states nothing. The same declaration, renamed, sits at line 258 of the
no-retraction file — where it is *not* recorded. So one of Sylvester–Gallai's
three declarations is empty, one of Napoleon's four is, and the rule for which
tautology gets counted is not stated anywhere on the site.

The inverse correlation makes the point sharply: the two smallest files on the
bench (74 and 86 lines) carry **four** recorded declarations each; the two
largest (124,019 and 92,816 lines) carry **two and one**. Brooks records exactly
one — `brooksTheorem` — even though its file defines **twenty** named `…Statement`
`Prop`s (`BrooksNonAcyclicStatement`, `BrooksDegreeTwoStatement`,
`BrooksHighDegreeRegularTwoConnectedSelectionStatement`,
`NonseparatingPairDeletionTwoComponentNonadjacentStatement`, …) that are
genuinely the decomposition of the proof. The counter measures how much
of a small file somebody bothered to name. **The site never says what the number
means, and read as a depth signal it is backwards.**

The useful part is the *layering*, not the count. Every entry separates:

- the **informal statement** (one sentence, on the card and in `summary`);
- the **scope line**, a tightened restatement (`"Scope: For every finite Young
  diagram μ, hookProduct μ · standardTableauCount μ = (μ.card)!"`);
- the **`Prop` definition** — the mathematics, quantifiers and all;
- the **wrapper theorem**, which the page explicitly demotes: *"This short
  declaration is useful for source identity, but the expanded proposition is the
  mathematical statement to read first."*

That last sentence is a control disguised as a caption. It concedes that
`theorem brooksTheorem : BrooksTheoremStatement` communicates nothing, and
directs the reader to the unfolded `Prop`. It is also where the presentation
fails: **the Brooks page prints the wrapper and stops.** There is no
"Definitions used in this proposition" block and no expansion of
`BrooksTheoremStatement`, so the bench's second-largest single-file entry
displays its headline claim as an opaque name. Sylvester–Gallai and hook-length
both expand, with each supporting `def` shown inline. The pattern exists; it is
not enforced.

## The fidelity argument: three mechanisms, ranked

A Lean file can check perfectly and state the wrong thing. The bench answers
this three ways, and the ranking is visible in the source.

**1. State it in Mathlib's own vocabulary.** Brooks concludes
`G.Colorable G.maxDegree` for `G : SimpleGraph V`; hook-length quantifies over
`μ : YoungDiagram` and concludes `= Nat.factorial μ.card`. There is nothing to
bridge, because nothing was re-defined. This is the cheapest fidelity there is
and it is available exactly when Mathlib already has the objects.

**2. Prove the bespoke definition equal to Mathlib's.** Where a coordinate
definition was needed for the proof, the file carries an explicit bridge:

```lean
/-- The local integer Laplacian is mathlib's graph Laplacian over `ℤ`. -/
theorem laplacian_eq_lapMatrix (G : SimpleGraph V) [DecidableRel G.Adj] :
    laplacian G = G.lapMatrix ℤ := by
```

and, in Sylvester–Gallai at line 358, the same move for the determinant
collinearity predicate:

```lean
theorem collinear3_iff_affine_collinear {p q r : Point} :
    Collinear3 p q r ↔ Collinear ℝ ({p, q, r} : Set Point)
```

No-retraction does it for its unit-circle subtype against Mathlib's bundled
`Circle`. **This is the transferable control**: a run that invents a predicate
to make a proof go through owes a theorem relating it to the library's. It is
one lemma, it is machine-checkable, and it converts "does this Lean mean the
mathematics" from a judgement into a proof obligation.

**3. Where neither is possible, write the gap down.** Napoleon and Heron have no
bridge, and the continuation JSON says so in a required `nonClaims[]` array:

> "This is the complex-algebra core of the theorem; it does not choose a
> particular sixth root such as exp(πi/3) or determine the external-versus-
> internal orientation of a Euclidean diagram."
> "It does not by itself give the full classical geometric presentation of
> Napoleon's theorem."

and, for hook-length, the sharpest one on the bench:

> "The Lean endpoint is the multiplication identity; the familiar
> factorial-divided-by-hooks expression is an interpretation rather than the
> literal statement."

`f^μ · ∏h(c) = |μ|!` is not `f^μ = |μ|!/∏h(c)`; over ℕ the division form needs
`∏h(c) ∣ |μ|!`, which is not proved. The page prints the familiar form on the
poster under the heading "THE FAMILIAR FORM" and then, in the boundary block,
says it is not what was checked. Every entry has such an array — Brooks: "does
not cover disconnected or infinite graphs"; matrix-tree: "does not cover
weighted graphs, multigraphs, or directed arborescences"; Sylvester–Gallai:
"This is not a projective or line-object quotient theorem." Two to three lines
each, always phrased as what a reader would *wrongly* take away.

The fourth layer is process: four named review gates recorded as IDs in
`publicationReview.reviewIds` — `formal_evidence`, `statement_alignment`,
`result_boundary`, `public_wording` — with `statementAlignmentStatus:
"accepted"` as its own field. The statement-alignment review is a human deciding
that the Lean means the theorem, and it is *recorded as a separate artifact from
the build*, so "Lean checked" and "the Lean says what we claim" never collapse
into one status. What none of the pages give is the reviewer's reasoning, or
what a rejection would have looked like. The gate is visible; its content is not.

## The visuals: asserted, and labelled asserted

Every entry carries a poster ("at a glance"), a schematic, and one image per
proof stage — 2.6 MB PNGs, `modelRequested: "gpt-image-2"`, each with a
`sha256`, a `generatedAt` timestamp, WebP derivatives, an `alt`, a
`description`, and a full plaintext `transcript` of the poster's words. They are
generated, and the site says so twice:

> "These AI-generated visuals explain the theorem and proof route; they are not
> proof evidence. Their publication review was completed separately from review
> of the formal result. The exact Lean proposition and checked source remain
> authoritative."

> "These stages follow the checked source and explain the mathematical route.
> They summarize the argument; they are not a visualization of Lean's internal
> proof term."

By this repository's vocabulary they are **asserted, never established** — the
`teams/BOARD.md` rule, applied to exposition. The second disclaimer is the more
interesting one, because it pre-empts the reading a diagram invites: that the
five stages are the proof's structure. They are not; they are prose about it.

And the seam shows. Sylvester–Gallai's five stages each name a *different*,
correct set of source lemmas — stage 2 lists `normalizeToXAxis`,
`normalizeToXAxis_left`, `normalizeToXAxis_right`,
`normalizeToXAxis_snd_ne_zero_of_not_collinear3`,
`linePointScore_normalizeToXAxis_lt_iff`; stage 4 lists six adjacency lemmas.
Somebody did the binding. **Brooks and hook-length repeat one identical
eight-name and seven-name list under all seven and all six stages.** Hook-length
prints `Cell, hookCells, hookLength, IsStandardTableau, standardTableauCount,
hookProduct, hookLengthFormula` six times, including under "Finite interpolation
resolves the content sum", which none of those names touches. The field degraded
to a constant and nothing caught it, because nothing derives it: the stage-to-
lemma map is hand-written beside the proof, not read out of it. A `derived/`
folder that refuses hand-written files is precisely the control that would have
made this impossible — and precisely the control they do not have here.

## What predicts a small proof

The small end is not "easy theorems". It is two specific shapes.

**Napoleon, 74 lines: the statement was chosen so the endgame is `ring`.** The
whole mathematical content is

```lean
theorem napoleon_centroid_relation (ω a b c : ℂ)
    (hω : IsSixtyDegreeRotationParameter ω) : …
```

proved by `simp; ring_nf`, one `have hω_sq : ω ^ 2 = ω - 1` derived from the
hypothesis by `calc`, then `rw; ring`. Geometry never appears. The rotation is
an abstract `ω` with `ω² − ω + 1 = 0`, not `exp(πi/3)`, so no analysis is
needed — and that substitution is exactly the thing the `nonClaims` array then
charges them for. Heron, 86 lines, is the same trade: `sqDist`, `twiceSignedArea`
and `heronRadicand` are polynomial, four identities close by `ring`, and the one
place a square root enters (`Real.sq_sqrt` three times, guarded by
`sqDist_nonneg`) is isolated in a single lemma. Riemann's own clearing
discipline — `lemmas::uncleared_divisions` — is the same instinct, one step
earlier.

**No-retraction, 233 lines: the last step already existed.** The proof is
`Circle.isCoveringMap_exp.liftHomotopy`, `cauchySeq`-style Mathlib assembly, and
`omega` on the winding-number parity. A theorem usually taught as hard is 233
lines because homotopy lifting is in the library. Its 6.7-second build is the
price.

So the predictor is **not** the theorem's reputation. It is: *is the conclusion
an identity in a polynomial ring, or is the final step a named Mathlib lemma?*
Everything above ~1,000 lines on this bench is a theorem where the answer is no
and the run had to build the combinatorial scaffolding itself — Brooks's 23
`def`s, 94 `lemma`s and 62 `theorem`s, Sylvester–Gallai's 25 `def`s and 145
`theorem`s (line-anchored counts).

The consequence for choosing a *first* Lean statement is direct, and it is not
"pick something easy". It is: pick the statement whose conclusion is an equation
between two expressions you can already write, in objects Mathlib already
defines. A run that opens by stating its problem in Lean — which this repository
requires of attempt one — will get a checked file on the first try when the
statement is a `ring` identity or a Mathlib-object equality, and will spend the
attempt on scaffolding otherwise. That is a schedulable distinction.

## Mathlib: used hard, and indexed separately

Every one of the seven files imports Mathlib, and every one imports
`Mathlib.Tactic` wholesale. Sylvester–Gallai pulls
`LinearAlgebra.AffineSpace.FiniteDimensional`; Brooks pulls six
`Combinatorics.SimpleGraph.*` modules; hook-length pulls `LinearAlgebra.Lagrange`
for its interpolation step. There is no avoidance, no re-proving of library
results, and no vendored copy — the ZIP ships "the checked first-party Lean
import closure" and states plainly that "Mathlib and other third-party
dependencies are identified but not rebundled". The reported line counts exclude
it, so the numbers above measure *new* mathematics only.

The 121-theorem landmark index is the other half of that policy, and its point
is addressability. Each entry binds an informal name to an exact declaration
with pinned bytes. From `banach-fixed-point-theorem.json`:

```
upstreamOrigin.declarationName  ContractingWith.exists_fixedPoint
upstreamOrigin.sourceCommit     5e932f97dd25535344f80f9dd8da3aab83df0fe6
upstreamOrigin.sourceFile       Mathlib/Topology/MetricSpace/Contracting.lean
upstreamOrigin.sourceLine       94
upstreamOrigin.sourceByteLength 15870
upstreamOrigin.sourceArtifactHash sha256:bb620eaf…
upstreamOrigin.verificationKind git_worktree
relationship                    upstream_direct
```

plus four **independent status axes**, each with its own sentence:

```
upstreamIndexed     Pinned source bytes verified locally
locallyReproduced   Exact upstream declaration replayed
reviewedPage        Current public presentation reviewed
acceptedAtlasResult Not recorded for the preferred artifact
```

and its own `nonClaims`, the first of which is: *"The selected declaration does
not itself state uniqueness of the fixed point."* Followed by: *"ProofAtlas is
indexing an existing Mathlib theorem, not claiming a new proof."*

The point of indexing what is already proved is that **a citation to a
declaration name is not the same object as a citation to a paper.** Riemann's
current answer to "this is known" is an `axiom` under `namespace Cited`, which
earns `conditional` — correct, because a claim read off a PDF is unverified
here. But when the result is *in Mathlib*, an axiom is strictly the wrong
encoding: the proof is already in the import closure, and writing an axiom
throws away a `formalised` verdict the run was entitled to. The bench never
writes such an axiom. It writes the import and uses the declaration.

The status-axis design is the second thing worth taking. "Locally reproduced"
and "reviewed" and "accepted result" are four columns, not four values of one
column, and the Banach entry is openly `Not recorded` on the fourth. A single
status would have had to lie about that row.

## How the proof was found

Almost nothing, and the almost is the interesting part.

There are **no search traces, no failed tactic logs, no retry counts, and no
model attribution** anywhere on the formalization pages — a sharp contrast with
the Sendov bundle, which named GPT-5.6 Pro. `recordedBuildElapsedMs` is the only
timing, and it is labelled "one machine-dependent evidence run, not a
benchmark".

What leaks is the *task shape*:

- Every file's module docstring opens `# <Theorem> Seed` and every headline
  `Prop` is doc-commented **"Minimum … target"**. Napoleon's says: *"The minimum
  target uses an abstract sixth-root rotation parameter; the stretch target
  instantiates it with the usual complex exponential rotation and adds public
  geometric diagrams."* Sylvester–Gallai's: *"The later stretch direction is to
  connect this interface to mathlib's affine geometry `Collinear` API."* So a
  target is authored as a **pair** — a minimum statement chosen to be reachable,
  and a named stretch that closes the fidelity gap the minimum leaves open.
  (Sylvester–Gallai's stretch was in fact reached: `collinear3_iff_affine_collinear`
  is in the file, and the docstring was never updated.)
- Heron's `heron_factor_identity` is doc-commented *"This is useful route
  support, but it does not prove that side lengths came from coordinates"* — a
  lemma banked mid-chain with an explicit note that it is not the result. That
  is precisely what the `identity` status in `derived/REDUCTIONS.md` was built
  for here.
- Versioning is visible: `v001` vs `v002` artifacts, `formalizationTargetId`
  distinct from `statementId` distinct from `artifactId`, and a
  `releasePlanStatus: "draft"` with the note *"Alternate route metadata; this
  publication uses the reviewed commit-pinned public source route."* Two routes
  to publication existed; one was taken; both are recorded.

The one place the process is on display is the **collaboration** tree — 200-odd
open conjectures, one page each. Conway's 99-graph page carries a route ledger
with statuses this repository does not have: `Active route`, `Eliminated route`,
`Narrowed route`, `Useful but insufficient`, `Not yet justified` — and, on a
*separate axis*, an **evidence posture**: `Reported result`, `Reported
reduction`, `Computation reproduced`, `Reported special case`. It reports "3.2k
retained lines of mathematical investigation" broken into argument development
2,702 / explored or eliminated routes 96 / computational analysis 108 / open
obligations 109 / definitions 151, under this disclaimer:

> "This measures retained mathematical investigation, not proximity to a proof.
> Code, data, logs, repeated text, operational instructions, and generated
> presentation copy are excluded."

and states the acceptance bar for its recommended task as:

> "Every finite elimination has a portable DRAT/LRAT, VeriPB, pivot-minor,
> Farkas, or independently checked orbit-table certificate. No partial log,
> opaque UNSAT line, or timeout is used as theorem evidence."

Riemann's `approaches` ledger already covers most of the route half —
`proposed`/`grounded`/`refuted`/`adopted`/`spent`, with the same argument in its
module docstring about the failure that leaves no trace. What it does not have
is the second axis. A route can be *narrowed* rather than refuted, and a result
can be *reported by a source* rather than established here, and those are
different facts about different objects.

## What this would change in this repository

Five, ordered by ratio of failure stopped to cost. Two are hours; three are days.

### 1. `tautologies` should split on `↔`, not only `=`

*Failure stopped.* `src/orchestrator/lemmas.rs::tautologies` catches
`x = x` and `: True`. It does not catch `P ↔ P`, which is the exact shape
ProofAtlas ships in three of seven files and *records as a declaration* in two.
`./lean-mill` turning a note into `theorem foo_statement : FooStatement ↔
FooStatement := by rfl` is a file that compiles, earns `formalised`, appears in
`derived/LEMMAS.md`, and says nothing. The existing `=` branch already handles
the operator-neighbour guards; `↔` needs the same treatment plus a guard against
`↔` inside binders.

*Where.* `src/orchestrator/lemmas.rs`, the loop at ~line 780; test in
`lemmas_test.rs` with the literal `SylvesterGallaiStatement ↔
SylvesterGallaiStatement` line as the fixture.

*Cost.* Under an hour. It is a `split_once('=')` becoming two passes.

### 2. A `bridge` status on the lemmas index, and a named gap when it is missing

*Failure stopped.* A run defines `myCollinear` to make a proof go through, proves
its theorem about `myCollinear`, and `derived/LEMMAS.md` reports `formalised`.
Nothing anywhere asks whether `myCollinear` is collinearity. This is the
fidelity failure in its live form, and it is the one failure a checked file
cannot detect on its own. ProofAtlas answers it with a theorem —
`laplacian_eq_lapMatrix`, `collinear3_iff_affine_collinear` — not with prose.

*Proposal.* `lemmas::collect` already walks every declaration. Add: a declaration
whose signature is `local_def = Mathlib_name` or `local_pred ↔ Mathlib_pred`
(detected by the right-hand side resolving outside the run's own namespace) is
tagged `bridge`. Then, for each `def` that the headline statement's `Prop`
mentions and that is defined in the workspace rather than imported, report
whether a `bridge` exists for it. Not a verdict — a column, the way
`uncleared_divisions` is advisory. The role that must act on it is
`lean_prover`, which already owns "whether the Lean means the mathematics".

*Cost.* A day. The namespace resolution is the fiddly part and can start crude:
a right-hand side whose head identifier does not appear as a local `def` in the
tree is external.

### 3. `does-not-cover` on a claim, required and separate from `holds-here`

*Failure stopped.* `01-sendov-bundle-anatomy.md` proposed a
`witnessed-by`/`does-not-cover` pair and it was not built. The bench makes the
case stronger, because `nonClaims[]` is required on *every* entry including the
74-line one, and the entries where it does the most work are exactly the small
cheap ones — where the temptation to overclaim is highest. Hook-length's
"multiplication identity, not the division form" is a fact no reader recovers
from `hookProduct μ * standardTableauCount μ = Nat.factorial μ.card` unless they
are already suspicious. `holds-here` is about whether the hypotheses hold in this
problem; this is about what the claim is silently *not* about.

*Proposal.* One required field on a `claims` event, rendered as its own column,
phrased as the reader's likely wrong takeaway. Empty is not permitted; "nothing
nearby is confusable" is an acceptable value and is itself informative.

*Where.* `src/orchestrator/claims.rs` and `claims/`, `derived/CLAIMS.md`; the
ceiling test in `ledger/ceiling_test.rs` needs the new column in its truncation
budget.

*Cost.* A day, most of it in the render and the ceiling accounting.

### 4. A `cited` verdict path for a Mathlib declaration, distinct from `Cited` axioms

*Failure stopped.* `namespace Cited` is right for a result read off a paper and
wrong for a result in Mathlib. Today both encode as an `axiom` earning
`conditional`. A run that needs `Nat.Prime` machinery, or the Banach fixed-point
theorem, or `Circle.isCoveringMap_exp`, and axiomatises it, has thrown away a
`formalised` verdict and left a run-wide `#print axioms` output permanently
dirty. The bench never does this; it imports and uses.

*Proposal.* When a `Cited` axiom's statement is available as a Mathlib
declaration, the mill should record the declaration name, the Mathlib commit and
the file:line — the `upstreamOrigin` block, four fields — and the statement
becomes an import plus a `theorem … := Mathlib.thing`, verdict `formalised`.
Where no such declaration is found, the axiom stands and stays `conditional`,
which is today's behaviour and the correct default. The discriminator is
mechanical: does the run's Lean build succeed with the axiom replaced by the
named declaration.

*Where.* `src/orchestrator/lean.rs` for the verdict, `lemmas.rs` for the index
column, the `lean_prover` prompt for when to attempt it. This is the control that
most directly attacks *re-establishing what is known*, which is the standing
failure mode named in `CLAUDE.md`.

*Cost.* Two to three days, and it wants a `mathlib_lookup` of some kind —
search over declaration names and types — which the repository does not have. A
cheaper first version: `lean_prover` proposes the declaration name by hand, the
harness verifies it compiles, and the verdict follows from the build. That is
half a day and captures most of the value.

### 5. Two axes on an approach: route status and evidence posture

*Failure stopped.* `approaches` has one axis with five stances, and `refuted`
currently absorbs three different things: the mathematics closed it, the
literature closed it, and it was pushed as far as it goes and stopped being
useful. ProofAtlas separates `Eliminated` from `Narrowed` from `Useful but
insufficient`, and separately records whether a result is *reported by a source*,
*reproduced here*, or *established here*. The second axis is the one riemann
actually lacks: nothing distinguishes "the paper says the g=32 branch is dead"
from "we killed the g=32 branch", and a later run cannot tell which of its
foundations it owns.

*Proposal.* Add a required `posture` field to an approach event —
`reported` / `reproduced` / `established` — rendered as a second column in
`derived/APPROACHES.md`, and split `refuted` into `refuted` and `narrowed`.
Do *not* copy their five route statuses wholesale; `not yet justified` is a
verdict field, not a route status, and would double-record.

*Where.* `src/orchestrator/approaches.rs`, `ledger/registry.rs` for the enum.

*Cost.* Half a day, plus a migration that defaults existing rows to `reported`,
which is the safe direction.

### What is deliberately not proposed

**A visual/poster pipeline.** Their at-a-glance layer is real work — transcript,
alt text, per-image sha256 — and it is asserted exposition serving a public
audience this repository does not have. The one part worth copying is already
covered by the `derived/` rule: their stage-to-lemma binding silently degraded
to a constant list under six consecutive headings in two of four entries read.
Anything we generate of that shape must be derived from the source or not exist.

**A "recorded declarations" counter.** It is inversely correlated with proof size
on their own bench and the site never defines it. `derived/LEMMAS.md` counting
statements per file already says more and says it honestly.
