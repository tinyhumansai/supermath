# ProofAtlas' two refutations, read as a harness

Sources, both fetched 17 August 2026 and read in full, with every sub-page:

- `proofatlas.ai/formalizations/jackson-hamilton-decomposition-counterexample/`
  — plus its Lean evidence record, its source listing, all six `.lean` files,
  `…evidence.json` and `…releases/jackson….json`. Published 23 July 2026,
  status **"Accepted in ProofAtlas · Lean-checked counterexample"**.
- `proofatlas.ai/research/berlekamp-domineering-temperature-counterexample/` —
  plus its collaboration page and the full 36 kB LaTeX manuscript. Published
  14 August 2026, status **"Unverified manuscript · adversarial audit
  attached"**.

[`01-sendov-bundle-anatomy.md`](01-sendov-bundle-anatomy.md) read a *proof*
bundle. This one is about the other direction, which our runtime has a tool for
(`refute.rs`, Vampire finite-model-building) and no discipline for. The two
pages are usefully paired: same curator, same year, same Lean stack, same
zero-`native_decide` line — and one is accepted while the other is not. The gap
between them is the finding.

## 1. Stating a counterexample so that it actually refutes something

Both pages spend more care on the *statement* than on the witness. This is the
part our runtime does least.

### Jackson: the statement is a predicate, and the quantifier is not weakened

The endpoint is three lines:

```lean
theorem flippedC4Blowup_counterexample :
    CounterexampleStatement candidateTournament
```

with, in `Basic.lean` (66 non-blank lines, and nothing else in the file):

```lean
noncomputable def CounterexampleStatement (T : Digraph Vertex) : Prop :=
  IsBipartiteTournament T ∧ IsThreeRegular T ∧ ¬ HasHamiltonDecomposition T
```

Every conjunct is spelled out from first principles rather than pulled from
Mathlib: `IsBipartiteTournament` says exactly one arc crosses each opposite-side
pair and none stays inside a side; `IsThreeRegular` says `outdegree = 3 ∧
indegree = 3` at every vertex; `IsDirectedHamiltonCycle` is `σ.IsCycle ∧
σ.support = Finset.univ ∧ ∀ v, T.Adj v (σ v)`.

The load-bearing choice is in `HasHamiltonDecomposition`:

```lean
def HasHamiltonDecomposition (T : Digraph Vertex) : Prop :=
  ∃ k : ℕ, ∃ C : Fin k → Equiv.Perm Vertex, IsHamiltonDecomposition T C
```

`k` is existentially quantified. The file comment on `HamiltonCycle.lean` says
why, plainly: *"No `Fin 3` family is assumed: the equivalence with `Fin 3` is
constructed only after the cardinality theorem."* Fixing `k = 3` would have been
free — a 3-regular digraph obviously decomposes into three cycles if it
decomposes at all — and would have produced a *weaker* refutation that a
sceptic could dismiss as refuting a different statement. Instead
`decomposition_cardinality_eq_outdegree` proves `k = outdegree T u` for
*arbitrary* `k` by building `Fin k ≃ {v // T.Adj u v}` out of the existence and
uniqueness halves of arc coverage, and only then reindexes. That is 50 lines
spent buying nothing mathematically and everything rhetorically.

The page then states the alignment claim explicitly, and this is the sentence
the task asked about:

> The statement matches the unrestricted formulation recorded by Granet and
> Liebenau–Pehova.

The word doing the work is **unrestricted**. Jackson's conjecture has a
restricted form (for orders above some bound) which Granet *proved* — so a
12-vertex counterexample must be pinned against the unrestricted form or it
would be claiming to contradict a theorem. The page carries that as a boundary
line rather than a footnote:

> The result does not conflict with Granet's theorem for all sufficiently large
> orders.

and, separately, refuses the novelty claim the counterexample would otherwise
imply:

> Granet already described the flipped-C₄ construction and its opposite-pair
> structure; no novelty claim is made for the construction.
> Historical novelty of the class-size-three parity obstruction remains under
> specialist review; no first, resolves, or priority claim is made.

Statement alignment is a *reviewed gate*, one of four, with a named reviewer
(`Claude Fable 5`) and a recorded limitation: *"Primary-paper comparison used
retained source quotations rather than a fresh paper fetch."* So the alignment
between the Lean `Prop` and the literature's conjecture is itself an audited,
bounded claim — not something the formalisation is assumed to have got right.

### Berlekamp: the statement was fixed by moving the witness, not the claim

The conjecture is `temp(G(P)) ≤ 2` for every finite Domineering position. The
26-cell core `B` has temperature `33/16` and would already refute it. But:

> The connected 26-cell core B has the same exact value, but it has
> checkerboard imbalance 14 − 12 = 2. A rectangular board has imbalance at most
> one, while every played domino removes one cell of each color. Therefore B
> cannot itself occur as the remainder of play from any rectangle.

Rather than argue that "finite position" includes unreachable regions, the
manuscript *repairs the witness*: translate `B` two cells right, adjoin the
isolated cells `(0,1)` and `(1,4)`, each a zero component, so `G(B') = G(B) + 0
+ 0 = G(B)`, and then exhibit 30 legal alternating moves from an 11×8 rectangle
whose remainder is exactly `B'`. Two extra cells and a 30-move table buy a
refutation that survives the stricter reading of the conjecture. The page then
forbids the conflation it just created:

> The 26-cell board B and the rectangle-reachable 28-cell witness B′ are
> distinct positions and must not be conflated.

This is the same manoeuvre as Jackson's unbounded `k`: when two formulations of
the conjecture exist, refute the one that is harder to refute, and say in the
artifact which one you took.

**Why the prior searches missed it** is the harness lesson underneath. The
manuscript records that Shankar–Sridharan searched "more than ten billion
positions" in `5×6`, `4×8`, `3×10`, `2×16`; Uiterwijk–Barton did all connected
positions through 15 cells; Huntemann–Maciosowski did arbitrary subgrids on
`5×5`/`5×6`, `6×6` with ≤20 empty cells, and a genetic search on `7×7`. All
reported nothing above 2. And:

> That 26-cell core has a 9×8 bounding box, outside the exhaustive regimes just
> listed.

The witness was never in anyone's search space. More compute inside the same
frame would never have found it. That is an argument about *how a refutation
search is scoped*, and it is not one our `refuter` role currently has any way to
record or revisit.

## 2. How the witness enters Lean

**Jackson: no generated data at all.** The witness is a three-line Boolean
function, hand-written and human-readable:

```lean
def candidateAdj (u v : Vertex) : Bool :=
  (v.1 == nextLayer u.1 && !(isX u.2 && isX v.2)) ||
  (u.1 == nextLayer v.1 && isX u.2 && isX v.2)

def candidateTournament : Digraph Vertex := Digraph.mk' candidateAdj
```

on `Vertex := Fin 4 × Fin 3`. There is no `Generated/` folder, no certificate,
no digest — because there is nothing to generate. Total witness data: roughly
six lines, counting the `nextLayer`/`prevLayer` tables.

Instead the file *re-exposes* the object in a second, independently readable
form and proves the two agree, so a reader who mistrusts the formula can check
the graph. Six lemmas give the full 6×6 adjacency matrix row by row:

```lean
theorem candidate_matrix_row0 (j : Fin 6) :
    candidateTournament.Adj (sideA 0) (sideB j) ↔ j = 1 ∨ j = 2 ∨ j = 3 := by
  revert j
  decide
```

plus `candidateArcCount_eq_thirtySix : candidateArcCount = 36`, whose docstring
says it is *"The exact number of directed arcs, independently exposed for
auditing."* That is a control: the witness is stated twice, in a compressed form
the proof uses and an enumerated form a human audits, with `decide` closing the
gap.

**`decide` vs `native_decide`, counted here:** 33 `decide` occurrences across
the six files (Graph 21, Parity 6, Matchings 4, ReversedEdges 2), **zero
`native_decide`**, and — unlike Sendov's 1,010 `decide +kernel` — **zero
`+kernel`**; plain `decide` suffices because every decided goal is tiny. The
idiom is uniform: `revert` the finite variables, then `decide`. 29 of the 33 are
literally a `revert` followed by `decide`. What is decided is never the theorem;
it is always a `Fin`-small fact — the adjacency matrix, `outdegree = 3`,
`suppressXMap_bijective` over `Equiv.Perm (Fin 3)` (6 cases), and the closed
`S₂` fact:

```lean
/-- Closed `S₂` fact: a fixed-point-free permutation of two labels is odd. -/
theorem finTwo_sign_eq_neg_one_of_no_fixed (f : Equiv.Perm (Fin 2))
    (h : ∀ a, f a ≠ a) : Equiv.Perm.sign f = -1 := by
  revert f
  decide
```

The axiom profile is `[Classical.choice, Quot.sound, propext]`, `hasSorryAx:
false`, `hasUnexpectedAxioms: false`, against a named allowlist
`"allowedAxiomProfile": "classical_mathlib_standard"`.

**Argument versus data, in lines.** The evidence JSON counts 6 files, 1,662
non-blank lines, 1,855 physical, 67,309 bytes; `Parity.lean` is the main file at
472. Split by role:

| Layer | Lines | What it is |
| --- | ---: | --- |
| `Basic.lean` | 66 | the statement vocabulary — no witness, no proof |
| `Graph.lean` | 184 | ~6 lines of witness + `decide`d re-exposure of it |
| `HamiltonCycle.lean` | 109 | `k` is forced to 3, for arbitrary `k` |
| `Matchings.lean` | 401 | boundary matchings, local parity |
| `ReversedEdges.lean` | 430 | the four reversed arcs occur in opposite pairs |
| `Parity.lean` | 472 | the two sign computations collide |

So the ratio is roughly **6 lines of data to 1,656 of argument** — the exact
inverse of Sendov's 110k certificate against 10k of reduction. The reason is
structural and worth naming: `¬ HasHamiltonDecomposition` quantifies over all
`k` and all `Equiv.Perm Vertex`, so it is *not a decidable proposition* and no
amount of enumeration would close it. A brute force over three-cycle families
would have to be re-expressed as a decision procedure with its own soundness
theorem; instead the run found a parity obstruction. The last three lines of the
proof are the whole shape of a refutation:

```lean
  have hpos := boundarySign_product_eq_one hD
  have hneg := boundarySign_product_eq_neg_one hD
  rw [hpos] at hneg
  norm_num at hneg
```

Two independent computations of `∏ εᵢ`, one giving `+1` and one `−1`.

**Berlekamp: the opposite choice, and it is the Sendov shape.** Here the object
is small (28 cells) but the *value* computation is enormous, so the certificate
pattern reappears — and, from the manuscript, in exactly the four-part form this
repository already requires:

> A Lean development defines the finite normal-play Domineering model, derives
> both players' move masks from the coordinate boards, and proves a sound
> checker for balanced difference-game certificates. Ordinary kernel reduction
> validates all 995,069 certificate states, arranged in 128 balanced subtrees
> and 32 proof modules.

> The development contains no `sorry`, custom mathematical axiom, or
> native-decision shortcut. Python output and file hashes are not theorem
> evidence.

Generated modules, a *proved* checker, kernel reduction, and an explicit
statement that the Python side is not evidence. Pins: Lean `v4.30.0-rc2`,
Mathlib `5450b53e5ddc…`, source commit `9edd1d307a62…`, final declaration
`Domineering.berlekamp_conjecture_false`.

## 3. What produced the witness, and what is recorded about the search

This is where both pages are thinnest, and I will not guess.

**Jackson: nothing is published about the search.** The advances page says the
dossier connects *"The explicit tournament, parity obstruction, independent
enumeration, formal theorem, and source"*, and the theorem page carries one
boundary line about it:

> Computational minimum order is independent corroboration and is not
> Lean-verified.

So an enumeration establishing that 12 is the minimum order exists and was run.
Its code, its bounds, its runtime and its outputs are **not on the site** —
`sitemap.xml` lists exactly three Jackson pages (theorem, evidence record,
source listing) and the ZIP contains only the six Lean files, the evidence JSON,
the licence and manifests. No record of failed candidate witnesses at all. The
credit record says Mazur *"contributed the explicit class-size-three
obstruction, matching-parity proof, and checked Lean formalization"* while
Granet is upstream for the construction — so the *witness itself came from the
literature*, and what the run added was the obstruction proof. That is a
different provenance from "a search found it", and the page is careful to say
so.

**Berlekamp: the search space is described only through what it was not.** The
manuscript gives the prior art's search regimes in detail (§"Historical context
and consequences", quoted above) and states the witness's bounding box falls
outside all of them, but says nothing about how *this* position was arrived at.
The disclosure box credits GPT-5.6 Pro with *"mathematical exploration, exact
computation, adversarial auditing, and exposition"* and Mazur with selecting and
reconciling outputs — the same curated-survivor caveat as Sendov. **No failed
candidate is recorded anywhere in either artifact.**

What *is* recorded is a bounded minimality probe, and the boundary on it is
exemplary:

> For every one-cell deletion from the 26-cell core, the exact computed
> temperature is below 2; the maximum is 15/8, attained at two cells. This is
> deletion-local evidence only. It does not prove global size minimality,
> uniqueness, or any minimality statement for the 28-cell reachable witness.

Termination, in both cases, was not a search halting. It was a human deciding
the witness was good enough to formalise.

## 4. Why Berlekamp is still an unverified manuscript

The naive answer — "it isn't formalised" — is wrong, and that is the whole
value of the contrast. It *is* Lean-checked, with a bigger certificate than
Jackson's by five orders of magnitude. What it lacks is different.

**(a) The Lean theorem does not entail the informal claim.** This is the real
gap:

> The exact theorem uses the deliberately narrow predicate
> `HasValueTemperature`: it exhibits a thermographed representative
> game-equivalent to the board. The development proves that equality and the
> representative's thermograph, but does not yet include a general theorem that
> thermographs are invariant under every game-equivalence presentation.

So Lean proves: *this board equals this explicit game, and this explicit game's
thermograph has temperature 33/16*. To get *the board has temperature 33/16* you
need thermograph-invariance under game-equivalence, which is not formalised. A
reader supplies that step from the literature. Jackson has no such step: the
`Prop` proved is the counterexample statement itself.

**(b) Parts of the argument are formalised and parts are not, and the split is
named:**

> The Lean theorem does not formalize the one-cell deletion audit or separately
> formalize the checkerboard unreachability of the raw 26-cell core B.

The checkerboard-imbalance argument is precisely what makes `B'` rather than `B`
the right witness — i.e. the statement-alignment step — and it lives in prose.

**(c) The review gates are not passed.** Jackson has four accepted reviews
(formal evidence, statement alignment, result boundary, public wording) bound to
an `accepted.…` record. Berlekamp has **one** — public wording — whose recorded
limitation is *"does not independently rerun Lean or Python, establish
accepted-result status, specialist peer review, historical priority, rights,
publication, or deployment authority."*

**(d) Independence is missing, and is called missing.** Both pages record
`"independenceStatus": "not_asserted"` for every reviewer. Berlekamp's five
verification routes are described honestly:

> The minimax checker and Lean development are structurally separate from the
> discovery engines. All routes were nevertheless coordinated in the same
> author-directed audit workflow. … Their agreement is strong algorithmic
> evidence but is not the same as unrelated external replication.

> ProofAtlas has matched the selected public files to the supplied manifest, but
> has not rerun the substantial Lean build for this page update.

**The cost boundary this draws.** Formalising a refutation is cheap when the
refuting *property* is decidable or elementary, and expensive when the property
is defined through a theory you would also have to formalise. Jackson's whole
predicate stack — tournament, regularity, Hamilton cycle, decomposition — is
first-order over a 12-element type and cost 1,662 lines including the entire
parity argument, one 134.5-second build. Berlekamp's witness is comparably
small, but "temperature" is defined via canonical forms, thermographs, and
game-equivalence — and formalising *that theory* is a research project of its
own, so the run stopped at a narrow interface and shipped the manuscript.
Restated as a rule: **the cost of formalising a refutation scales with the depth
of the theory the refuted predicate is stated in, not with the size of the
witness.** A run should estimate that depth *before* committing to a
counterexample hunt, because it decides whether the outcome will be a theorem or
a manuscript.

## 5. What a run should have to produce before a claimed disproof is believed

Read off the two pages, in the order they would stop a bad claim:

1. **The negated statement, formalised from primitives, with the quantifiers at
   full strength** — `∃ k`, not `k = 3`. Written in the artifact, not in prose.
2. **A named alignment target**: which formulation of the conjecture, from which
   source, and what nearby theorem the counterexample must *not* contradict
   ("does not conflict with Granet's theorem for all sufficiently large
   orders").
3. **The witness twice**: the compressed form the proof uses, and an enumerated
   form a human reads, with a checked lemma tying them together.
4. **The obstruction, not the enumeration** — or, where enumeration is
   unavoidable, the four-part certificate with a proved checker and ordinary
   kernel reduction.
5. **An axiom profile against an allowlist**, plus `no-sorry`, plus toolchain,
   Mathlib and source-commit digests.
6. **A `does-not-cover` list**, written by the person who did the work: which
   corroborating computation is *not* theorem evidence, which reviewer did not
   rerun the build, which minimality is only deletion-local.
7. **The search frame, recorded** — what space was swept, and which prior
   exhaustive searches the witness falls outside of.

Item 7 is the one neither page fully supplies and the one our harness is
best placed to supply, since it is the only party with the search log.

## What this would change in this repository

Six proposals. Nothing here restates the five controls built after the Sendov
reading; the certificate arm, `reductions`, `identity`, `thesis` and the
clearing discipline are assumed present.

**1. A `refutation` claim kind with a required `refutes` field.**
*Failure it stops:* a counterexample filed as a claim with no statement of what
it contradicts — so a witness to a mis-stated or already-restricted conjecture
reads as a disproof. Today `refute.rs` files a Vampire verdict under
`code/out/refute/` and the prose becomes a `CLAIMS.md` row; nothing forces the
run to name the formulation refuted or the nearby theorem that must survive.
*Where:* `orchestrator/claims.rs` (a `Status`/kind variant), rendered in
`derived/CLAIMS.md`; parsed field `refutes:` alongside the existing `holds-here`.
*Cost:* one enum variant, one field, one parse arm and its test. Small. The
awkward part is deciding whether an unfilled `refutes` blocks the claim or
degrades it — it should block, on the `thesis`/`refuted-by` precedent.

**2. `statement_strength` check in `lemmas.rs`: refuse a refutation whose
formal statement instantiates a bound the conjecture leaves free.**
*Failure it stops:* the `k = 3` shortcut. A run refuting "no `Fin 3` family
decomposes T" has refuted something nobody conjectured. Mechanically this is
detectable in the weak, useful form: if the negated goal contains a numeral
where the cited statement of the conjecture had a quantified variable, name it.
*Where:* `orchestrator/lemmas.rs`, beside `uncleared_divisions`, and reported in
`derived/LEMMAS.md`.
*Cost:* medium, and it is advisory rather than blocking — like the clearing
discipline, it names a suspicion. Honest caveat: full alignment checking is the
mathematics and cannot be mechanised; this catches only the arity/numeral case.

**3. A `witness` ledger entry that requires the object stated twice.**
*Failure it stops:* a witness that exists only as a compressed generator, so
nobody — reviewer or later run — can read the object. Jackson's six matrix rows
and `candidateArcCount = 36` are the pattern. The entry carries the definition,
an enumerated re-exposition, and the `decide`d lemma identifying them; a
`witness` with no second form is reported incomplete.
*Where:* `orchestrator/ledger/registry.rs` as a new axis with its
`derived/WITNESS.md`, read by `lemmas.rs` for the identifying lemma.
*Cost:* one ledger axis plus the cross-check into the Lean surface — the
cross-check is the real work, and without it the axis is a prompt instruction.

**4. `search_frame` on a refutation: what was swept, and which published
exhaustive searches it lies outside.**
*Failure it stops:* re-running someone else's completed search and reporting
"none found" as evidence, which is exactly the trap the Domineering literature
sat in for 22 years. The Berlekamp witness has a 9×8 bounding box and every
prior sweep was `≤ 7×7` or `≤ 20` empty cells; that fact is worth more than any
individual computation on the page. Our runtime is the only party that *has*
the sweep parameters, and today they evaporate when the container exits.
*Where:* a field on the `refutation` claim (proposal 1), populated by `refute.rs`
and by any `tool_builder` enumeration; the librarian's job is to fill the
"outside which published regime" half.
*Cost:* small in code, real in prompt/role work — `refuter.md` and `librarian.md`
both change. This is the proposal I would build first: it is cheap and it is the
item both ProofAtlas pages fail to supply.

**5. `formalisation_depth` gate before a counterexample hunt is authorised.**
*Failure it stops:* spending a run's budget on a witness whose refuted predicate
is defined through a theory Mathlib does not have — arriving, as Berlekamp did,
at a narrow interface and a manuscript. The check is a question the `lean_prover`
must answer before the hunt starts: *can the negated statement be written from
Mathlib primitives today, or does it need an intermediate theory?* If the
latter, the missing theory becomes a `backward` entry and the hunt is scheduled
behind it, or the run declares up front that its outcome will be conditional.
*Where:* `orchestrator/solutions_routing.rs` — a precondition on the route into
the refuter — with the answer filed as a `backward` lemma.
*Cost:* medium-high; it adds a turn before every refutation attempt. Justified
only if we see the Berlekamp failure once. Worth writing down now as the thing
to watch for.

**6. `does-not-cover` as a required field on any claim whose evidence is a
program.** Note (d) of the Sendov reading proposed a `witnessed-by` /
`does-not-cover` pair; these two pages make the case sharper, because both put
the *negative* half in the artifact and in identical language — "Computational
minimum order is independent corroboration and is not Lean-verified", "Python
output and file hashes are not theorem evidence", "This is deletion-local
evidence only."
*Failure it stops:* an executed program and a checked `.lean` file being read as
one piece of evidence when they cover different statements — which our own
"every attempt ends with a checked `.lean` file *and* an executed program" rule
makes *more* likely, not less, since the two artifacts always arrive together.
*Where:* `orchestrator/claims.rs`; required when `status` is `Checked`.
*Cost:* small, and it is the second one I would build.

**Not proposed, and why.** A "review gates" analogue of ProofAtlas' four
accepted reviews — formal evidence, statement alignment, result boundary, public
wording — is tempting and I am leaving it out. Every reviewer on both pages is
an AI with `"independenceStatus": "not_asserted"`, reviewing artifacts produced
in the same workflow; the gates are a *presentation* discipline for a public
record, and we would be building four more model calls that agree with the run
that produced the claim. The parts of that discipline worth having are the ones
above, which are fields on evidence rather than opinions about it.
