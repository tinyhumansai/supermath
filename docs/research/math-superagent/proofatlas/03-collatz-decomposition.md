# ProofAtlas' Collatz programme, read as a decomposition

Sources, all fetched 17 August 2026 and read in full, including the `/proofs/`
sub-pages, the `data/formalizations/*.json` continuation records and the
`*.evidence.json` checker records behind them:

- `proofatlas.ai/formalizations/tao-almost-bounded-orbits/`
- `proofatlas.ai/formalizations/rhin-phase-gap-log2-three/`
- `proofatlas.ai/formalizations/terras-log-time-power-saving/`
- `proofatlas.ai/formalizations/natural-density-log-time-collatz/`
- `proofatlas.ai/collatz-predecessor-090/`

Same curator as the Sendov bundle ([`01-sendov-bundle-anatomy.md`](01-sendov-bundle-anatomy.md)),
same Lean toolchain (`v4.30.0-rc2`), a different problem — and a completely
different answer to the question that note ended on. Sendov is one theorem with a
110k-line certificate under it. This is *five* published theorems, in one Lean
package (`Erdos1135`), that between them do not prove Collatz and say so five
different ways. The subject here is the cut, not the mathematics.

## The five results and what each one is

| Page | Endpoint | Files | Lines | Main file |
| --- | --- | ---: | ---: | ---: |
| Rhin phase gap | `∃ c, PhaseGap c (143/10)` | 40 | 8,926 | 56 |
| Terras power saving | failure ratio `≤ 10⁷·N^(−1/100)` for `N ≥ 15552` | 20 | 2,511 | 201 |
| Tao almost-bounded | log-density-one orbit minima below any diverging `f` | 397 | 124,019 | 110 |
| ND log time | natural density one, clocks `< 145` and `< 436` | 599 | 182,625 | 224 |
| Predecessor 0.90 | three eventual lower bounds `Pₜ(x) ≥ x^0.90` | 58 | 10,237 | 39 |

Line counts are the site's own: non-blank first-party import closure at a pinned
commit, comments included, Mathlib excluded.

## How an unattackable conjecture was cut

Not by weakening the conclusion once. By weakening it along **four independent
axes at the same time**, and then publishing each combination that could be
closed. Read across the five pages, the axes are:

1. **Which density.** Logarithmic (Tao) → ordinary natural (ND) → odd-relative
   among odd starts (the Syracuse clause of ND) → none at all, an explicit
   proportion for every `N ≥ 15552` (Terras).
2. **What "descent" means.** Reaching `1` (the conjecture) → orbit minimum below
   a diverging threshold (Tao) → below a fixed target (ND31) → below `√N` (the
   bracket) → below the starting value (Terras) → reaching a fixed `t` at all
   (predecessor).
3. **Whether time is counted.** Tao's theorem has no clock. ND is Tao plus a
   clock: `145·log N` odd-to-odd steps, `436·log N` raw steps. The square-root
   bracket adds a *lower* clock, `log N/(2 log 2) < m`.
4. **Which map.** Standard Collatz, accelerated `T(n) = (3n+1)/2`, and odd
   Syracuse are three different maps and every page names which one it counts.

Each published theorem is a point in that grid, and the boundary text on each
page is, almost exactly, the list of neighbouring grid points it is *not*.

## The boundary is data, and it names its neighbours

This is the strongest control on the site and it is not a disclaimer.

Every result carries a `boundary` string and a `nonClaims` array in
`data/formalizations/<slug>.json`, and the page renders them — the header
"Scope:" line, the "Result boundary" panel and the poster's "EXACT SCOPE" block
are three renders of one string. Derive, never restate, applied to prose.

The six ND non-claims, verbatim from the JSON:

> - This does not prove the Collatz conjecture, convergence for every start, or
>   arrival at 1; a density-zero exceptional set may remain.
> - The threshold must tend to infinity but need not be monotone, and the hit
>   below it is strict.
> - The odd-relative Syracuse conclusion and ordinary-density raw Collatz
>   conclusion have different domains and must not be collapsed into one claim
>   about all positive starts.
> - The 145 bound counts odd-to-odd Syracuse steps, while the 436 bound counts
>   raw Collatz steps including halvings.
> - The square-root lower clock belongs only to the companion square-root
>   corollary, not to the general growing-threshold theorem.
> - The separate Terras power-saving finite-stopping theorem is a companion
>   comparison, not an input to this proof.

Only the first is a generic "we did not prove Collatz". The other five each name
a *specific adjacent statement in the same package* and refuse it. Three of the
six are about confusions with the programme's own sibling results. Tao's page,
which has no siblings sharing its clock, has only two non-claims. The boundary is
sized to the confusability of the neighbourhood, not to the fame of the problem.

The claim is bounded in the Lean too, not only on the page. Terras' exceptional
set is bounded by `10000000 * (N:ℝ) ^ (-(1/100:ℝ))` — a bound that exceeds 1
until `N` is astronomically large, which the page states rather than hides:

> The constants and exponent are explicit but deliberately coarse and are not
> claimed optimal; the numerical bound is informative only for very large `N`.

And Rhin's page volunteers the fact that its own constant is useless:

> The witness `c` is existential; the theorem exposes no numerical value for it.

A separate review lane exists for exactly this — "result boundary · Checks the
explicit limitations and non-claims", accepted per family rather than per
theorem, whose stated job is that the boundary "keeps nearby stronger or commonly
confused claims out of scope".

## The dependency chain, and the edge they refused to draw

The reviewed path, quoted from the relationship panel:

> Reviewed dependency path: Rhin phase gap → ND31 main → ND31 bounds →
> same-exponent rate → fixed rate → the two sibling density-family endpoints.
> Six retained `depends_on` edges support this contracted path.

Concretely, from the eight-stage proof route on the ND page:

```
Rhin large-height estimate  →  existsPhaseGapRhin : ∃ c, PhaseGap c (143/10)
  →  scheduled-passage separation (NDM2Bounds 1/32000, 6993/200000)
  →  telescoped at one fixed exponent d = 6993/200000 < 5/143
  →  ND31LogTimeBounds : oddSyracuseLogTimeBadRatio ≤ C·(log N₀)^(−d)
  →  ND16 (odd-relative, C_syr)  →  two-adic lift  →  ND13 (ordinary, C_coll)
  →  √N bracket (adds a deterministic lower clock)
```

Three things about this chain are worth stealing.

**The exponent is the same object at every step.** The Diophantine exponent
`κ = 143/10` in the phase gap becomes the cap `d < 5/143` on the density rate,
and the run pays `d = 6993/200000 = 0.034965` against `5/143 = 0.03496503…` — a
reserve of `3.5 × 10⁻⁸`, kept strictly positive on purpose. Stage 4 says why:

> The proof pays each local error into one quantitative envelope instead of
> weakening the rate at every stage. The strict cap matters: the checked exponent
> has reserve and the endpoint is not attained.

One scalar carried unweakened through five stages, with its slack stated. This is
the `reductions` ledger's shape, and it is the first live example I have seen of
a *chain* of reductions all collapsed onto the same scalar rather than a new one
per step.

**Terras is deliberately not an edge.** The Terras result shares source files with
the ND closure — `Erdos1135/Terras/Parity` (503 lines) and `Terras/Density` (358)
are inside both the Tao and ND import closures — and it is about the same map and
the same kind of conclusion. It is still recorded as a companion:

> No inferred edge: shared source files, a common subject, or historical
> background do not create a theorem dependency.

A shared import is not a dependency. Our `backward` ledger has no way to say that
today, and "these two lemmas are in the same file" is exactly the accident that
would create a phantom edge in it.

**Imported known results are re-proved, not cited.** This is the finding that
matters most for us, and it cuts against our `Cited` axiom design.

Rhin's 1987 linear-form estimate for `log₂ 3` — a genuine literature result — is
*formalised from scratch*: `Erdos1135/NumberTheory/Rhin/` is 14 files and 5,960
lines, of which `LargeHeight.lean` alone is 2,654. Its evidence record reports

```
"allowedAxiomProfile": "classical_mathlib_standard",
"axioms": ["Classical.choice", "Quot.sound", "propext"],
"hasSorryAx": false, "hasUnexpectedAxioms": false
```

Same for Tao 2019: 397 files, 124,019 lines, axiom profile clean, no `sorry`. The
imported/new line is drawn **by namespace, not by axiom**: `Erdos1135/Tao/*` is
Tao's paper, `Erdos1135/Terras/*` is Terras', `Erdos1135/NumberTheory/Rhin/*` is
Rhin's, and `Erdos1135/ND/*` is the new work. Nothing anywhere in the programme
is an axiom standing in for a citation.

The cost of that choice is visible and large:

| Layer | Files | Lines | Share of ND closure |
| --- | ---: | ---: | ---: |
| `Tao/Renewal` | 167 | 75,780 | 41.5% |
| `ND/Band` | 72 | 22,193 | 12.2% |
| `ND/Discrepancy` | 39 | 12,953 | 7.1% |
| `ND/Fourier` | 44 | 12,067 | 6.6% |
| `Tao/Section5` | 55 | 10,580 | 5.8% |
| `Tao/Fourier` | 32 | 9,968 | 5.5% |
| `Tao/Probability` | 37 | 8,159 | 4.5% |
| `Tao/Section6` | 39 | 7,300 | 4.0% |
| `Tao/Syracuse` | 37 | 6,409 | 3.5% |
| `NumberTheory/Rhin` | 14 | 5,960 | 3.3% |
| `Tao/Section3` (+ leaf) | 15 | 4,233 | 2.3% |
| `ND/LogTime` | 12 | 2,351 | 1.3% |
| everything else | 36 | 4,672 | 2.6% |

**Two thirds of the ND package is imported mathematics re-proved.** `182,625 −
124,019 = 58,606` lines are new; the Tao closure it sits on is 124,019. And a
single renewal/Fourier block, `Tao/Renewal`, is 75,780 lines — 61% of Tao's own
closure — for one proposition, the primitive-frequency polynomial decay of stage
04. The new mathematics that produced the headline is `ND/Band` +
`ND/Discrepancy` + `ND/Fourier` + `ND/LogTime` ≈ 49,564 lines in 167 files.

The publishable surface is `Paper.lean`, **224 lines**: 0.12% of the closure.

## Three theorem variants from one certificate

The predecessor page publishes three Lean theorems off the same level-18
certificate, differing only in cutoff type and exponent:

```lean
theorem predecessor_count_lower_bound_090_nat  {target} (htarget) (hmod) :
  ∀ᶠ cutoff : Nat in Filter.atTop, (cutoff:Real)^((9:Real)/10) ≤ predecessorCount target cutoff

theorem predecessor_count_lower_bound_090      {target} (htarget) (hmod) :
  ∀ᶠ x in Filter.atTop, x^((9:Real)/10) ≤ predecessorCountReal target x

theorem predecessor_count_lower_bound_0901     {target} (htarget) (hmod) :
  ∃ constant : Real, 0 < constant ∧
    ∀ᶠ x in Filter.atTop, constant * x^((901:Real)/1000) ≤ predecessorCountReal target x
```

The proof route's stage 08 is titled "Spend the exponent reserve and expose three
endpoints", and it explains the whole design:

> Because 0.901 exceeds 0.90, eventual growth absorbs the unknown positive
> constant and yields unit-coefficient real and natural cutoff bounds alongside
> the stronger constant form. The asymptotic reserve is exactly 0.001.

So the *real* result is `∃ cₜ > 0, cₜ·x^0.901 ≤ Pₜ(x)`. The clean-looking
`x^0.90 ≤ Pₜ(x)` is that result with 0.001 of exponent spent to buy away an
unnamed constant. Publishing the family rather than the headline is what makes
that trade visible; publishing only `x^0.90` would hide both the stronger
exponent and the fact that the constant is unknown. The poster says it outright —
"The 0.001 exponent reserve—not `cₜ = 1`—yields the unit-coefficient 0.90
endpoint" — and one of the three non-claims is "The target-dependent positive
constant in the exponent 901/1000 theorem is not claimed to equal 1."

What the family buys a search process, in order of how much I believe it:

1. **Type-matching for downstream work.** `Nat` cutoff and `Real` cutoff are
   different statements in Lean. A later proof needing the integer form and
   finding only the real form pays a coercion argument; publishing both removes
   a class of dead ends that are pure friction, not mathematics.
2. **It localises where the strength is.** Three endpoints, one certificate,
   and the diff between them is exactly `0.001` of exponent. A search process
   reading the family knows precisely which knob has slack — and the page's
   "Open route 1" is to push exactly that knob.
3. **Review granularity.** Statement-alignment and formal-evidence reviews are
   recorded per form; the result-boundary review is recorded once as "Shared
   result-boundary review for all three theorem forms". Strength is checked per
   statement, scope is checked per family. That is the right split, and it is
   cheap.
4. **A weaker statement survives a later strengthening.** More on this below —
   it is the same idea, applied across time rather than across a family.

`Paper.lean` shows the across-time version. When the timed ND result superseded
the earlier untimed one, the package did not delete the old statement; it proved
the old one *from* the new one, and shipped the implication as part of the
headline conjunction:

```lean
/-- Forgetting the Syracuse clock turns timed ND31 into the frozen ND31
statement without changing the exponent. -/
theorem nd31Bounds_of_logTimeBounds {d Ctime} (h31 : ND31LogTimeBounds d Ctime) : ND31Bounds d
```

Three such bridges (`nd31Bounds_of_`, `nd16Statement_of_`, `nd13Statement_of_`)
feed `ndChainStatement_of_logTimeChainStatement`, and `NDChainStatement` — "the
previously frozen ND chain" — is the fourth conjunct of the published theorem.
The earlier published result stays true, stays checked, and is visibly subsumed.

## Constants: where they come from and what they cost

| Constant | Where it comes from |
| --- | --- |
| `κ = 143/10`, gap `q^(−133/10)` | Rhin's published large-height estimate; the exponent is his |
| `d = 6993/200000` | `5/143` truncated to five decimals, so `d < 5/143` strictly |
| `cHit = 1/32000` | the passage-estimate budget, carried as a literal in `NDM2Bounds` |
| `C_syr = 501501/(5000 log 2) ≈ 144.703` | `taoAlpha · ndSyracuseAmbientTimeConstant`, computed |
| `C_coll = 1509503/(5000 log 2) ≈ 435.550` | `collatzLogTimeConstant C_syr`, computed |
| `145`, `436` | round integers above the computed constants, each its own theorem |
| `N ≥ 15552 = 2⁶·3⁵`, `10⁷`, `1/100` | Terras: "deliberately coarse", chosen to make the final comparison easy |
| `901/1000`, then `9/10` | the certificate proves 0.901; 0.90 is what the reserve buys |

Two habits here are transferable and neither is about mathematics.

**Every constant is an exact rational or a closed form in `log 2` — never a
decimal, never `O(·)`.** `501501/(5000 * Real.log 2)` is a term Lean can reason
about; `≈ 144.7` is not, and `O(log N)` is not a Lean statement at all. This is
the same discipline as Sendov's `⌈2⁶⁴·f⌉` integer ceilings: the constant is
carried in a form whose comparisons are decidable. An `O(·)` result cannot be
stated as a theorem about a specific number, so it cannot be a link in a chain
that ends in `< 145`.

**The headline constant and the working constant are different objects, joined by
a theorem.** `Paper.lean` proves the closed form
(`ndSyracuseLogTimeConstant_eq_501501_div`) and then, separately,
`ndSyracuseLogTimeConstant_lt_145`. The proof carries the exact value; the public
statement carries the round one; the bridge is checked. Nobody has to choose
between a legible headline and an exact proof.

The cost of explicit constants is that *every* estimate must be effective. There
is no `for sufficiently large N` anywhere in Terras' chain — hence `15552`, and
hence a bound of `10⁷·N^(−1/100)` that exceeds 1 for all `N` a human would ever
type. The programme accepted an embarrassing constant to keep the statement
effective, and then wrote the embarrassment into the boundary.

## Three placements of the finite computation, in one programme

Sendov put its certificate *inside* Lean: 110,406 lines of generated `def`s,
hand-written checkers, `decide +kernel` 1,010 times, `native_decide` zero times.
This programme does two other things, and the difference is declared each time.

| Result | Finite computation | Where it lives | Axiom profile |
| --- | --- | --- | --- |
| Terras / Rhin / Tao / ND | none | — | `classical_mathlib_standard`, clean |
| Predecessor 0.90 | 129,140,163 LP rows (`= 3¹⁷`) + 215,233,605 adaptive-potential inequalities (`= 5·3¹⁶`), 344,373,768 total | outside Lean | **two named `native_decide` axioms** |

The predecessor page does not hide the second row. It counts the checks on the
page, states "zero failures in retained checks", records "Independent Python and
C++ reruns agree with those calculations, but do not replace the disclosed
dependencies", and puts this in the non-claims list:

> The native-assisted certificate checks are not described as kernel-clean.

It also files "reduce the computation trust boundary" as an open route:

> Replace or complement the native finite checks with smaller kernel-checkable
> certificates, independently replayable proof objects, or reusable verifier
> lemmas.

So the same curator, at the same time, shipped a kernel-clean certificate
(Sendov), a disclosed native-assisted one (predecessor), and three results with
no certificate at all — and the *verdict vocabulary* distinguishes them. Ours
does not. `lean_check` refuses `native_decide` outright, which is the right
default and would have refused this result rather than grading it.

What is genuinely new here for us is not the refusal. It is that a run can be
allowed to bank a native-assisted result *at a lower grade*, with the trust
boundary named, the replay stacks named, and "reduce this boundary" filed as a
frontier entry. Today that result would either be silently downgraded to prose or
rejected.

## What the pages do not say

Stated plainly rather than guessed at:

- **Nothing about how the decomposition was chosen.** The AI contribution is
  recorded as "route proposals, proof-route narrowing, finite probes, stress
  tests, and read-only audits" under the roles *Computation, Gap or error
  discovery, Proof strategy, Research direction* — and the release scope says
  "AI development credit remains aggregate because stable historical model and
  run identities were not retained." No run logs, no dead ends, no model names,
  no indication whether the four-axis grid was designed up front or fell out.
- **No timing beyond the check.** Recorded build times are 2.3 s and 2.0 s for
  the ND artifacts and 1.3 s for Rhin — these are *evidence-collection* builds
  from a warm cache, labelled "one machine-dependent evidence run, not a
  benchmark". Nothing says what a cold `lake build` of 599 files costs, and
  nothing says how long the programme took.
- **No negative controls.** Sendov's mutation tests have no analogue here.
  Neither the LP certificate nor the adaptive-potential certificate is reported
  as having been corrupted and rejected; "zero failures in retained checks" is a
  positive-test statement.
- **Reviewer independence is denied, not asserted.** Every review record reads
  "Reviewer: Claude Fable 5 · Independence: Independence not asserted", with an
  explicit limit — "The reviewer did not rerun Lean; it recomputed retained
  source, import-closure, and transcript bindings." That is a model reviewing a
  model's work, and the record says so rather than implying peer review.
- **Priority is refused, not claimed.** "ProofAtlas does not independently
  convert that search limitation into an unqualified priority claim"; "No
  independent external priority anchor is recorded."

## What this would change in this repository

Five proposals. Three are small; two are not.

### 1. `boundary` / `not-claimed` fields on a claim, naming the neighbour

**Failure it stops.** A run establishes a density-one statement and a later turn
reads it as a universal one; or two ledger rows about different maps get combined
because both say "descent". Our `holds-here` column records hypotheses, not the
adjacent statements a claim will be confused with. `01`'s gap (d) asked for
`witnessed-by` / `does-not-cover`; this is the sharper half of it — a non-claim
must **name a specific neighbouring statement**, ideally another row's id, not
say "this does not prove the conjecture".

**Where.** `ledger/registry.rs` for the fields; `derived/CLAIMS.md` renders them
under the claim, capped like every other section; the `verify` ranking is where a
claim with an empty `not-claimed` on a partial result gets flagged.

**Cost.** One required field on partial-result claims, one rendering block,
roughly a day. The risk is generic boilerplate — "does not prove the conjecture"
on every row — which is worth a lint: a non-claim that does not name another
ledger id or a named statement is not a non-claim.

### 2. A typed edge in `backward`, with shared imports explicitly excluded

**Failure it stops.** `backward` currently holds lemmas that recombine into the
goal. It cannot express *companion* — a result about the same object, in the same
files, that is not an input. The ProofAtlas line, "shared source files, a common
subject, or historical background do not create a theorem dependency", is the
exact accident our co-location invites: two lemmas in one `code/lean/Lib/` module
read as a dependency because they are neighbours.

**Where.** An edge kind on `backward` entries — `depends-on` / `strengthens` /
`companion` — with `depends-on` required to name the lemma actually used.
`derived/BACKWARD.md` renders the three lanes separately, as the atlas does.
`lemmas.rs` already parses the Lean; the dependency it can *see* is the import
graph, which is exactly the thing that must not be promoted to an edge.

**Cost.** Small, and it makes `BACKWARD.md` more honest immediately: today an
entry with no thread reads the same as an entry with a companion result beside
it.

### 3. Subsumption theorems: a strengthened result must prove its predecessor

**Failure it stops.** A run improves an exponent or adds a clock, the old claim
is superseded, and the old row is either deleted or left dangling with no
mechanical link. Both are bad: the deleted version loses the record of what was
established, and the dangling one is a second answer to what is true.

`Paper.lean` is the pattern, and it is three lemmas and one conjunct:
`nd31Bounds_of_logTimeBounds`, `nd16Statement_of_logTimeStatement`,
`nd13Statement_of_logTimeStatement`, then `NDChainStatement` as a conjunct of the
published theorem. The strengthening carries its own proof that nothing was lost.

**Where.** When a claim is filed with `supersedes: <id>`, `lean_prover` owes a
checked implication `new → old` before the new claim can earn `formalised`; the
old row stays, status `subsumed`, with the bridge lemma named. `lemmas.rs` can
check that the named bridge exists and elaborates.

**Cost.** Real mathematical work per strengthening — sometimes the bridge is
`exact ⟨_, _⟩`, sometimes it is a genuine argument. That is the point: if the new
statement does not imply the old one, they are different results and both belong
in the ledger.

### 4. A `citation surface` file as the run's deliverable

**Failure it stops.** Nothing in our workspace answers "what did this run
establish" in one readable page. `derived/CLAIMS.md` is a ledger; `code/out/` is
artifacts; `LEMMAS.md` is derived from every module. The Collatz package puts the
whole publishable claim in a 224-line leaf that imports the closure and states:
the exact rational exponent, closed forms for both constants, integer upper
bounds for each, and the implication back to the previously frozen result — all
as one conjunction, `ndRhinLogTimePaperPackage`.

**Where.** `code/lean/Lib/Paper.lean` (or `Surface.lean`), written by
`lean_prover`, required non-empty once any claim reaches `formalised`. It is a
Lean file, so it is checked, so it cannot drift from the ledger the way a summary
would. `LEMMAS.md` gains a line naming it.

**Cost.** Nearly free, and it gives `archivist` something concrete to compare
across candidates: two candidates' surface files are two conjunctions, and their
difference is the whole argument about which to adopt.

### 5. Two-tier constants: exact form and headline bound, joined by a lemma

**Failure it stops.** A run that computes `≈ 144.7` and writes it down has
written a number Lean cannot use; a run that keeps `501501/(5000·log 2)` has a
statement nobody can read. The package does both and proves the bridge
(`_eq_501501_div`, then `_lt_145`). The general habit — **never let a decimal
approximation be the only record of a constant** — is worth a lint, because it is
mechanically checkable: a numeric literal in a claim body with no exact
counterpart in the Lean is a smell, the same class as `lemmas::uncleared_divisions`.

**Where.** `lemmas.rs`, advisory like the clearing-discipline check. `reductions`
gains a place for the exact form beside its two bounds — it already carries the
bounds as separate fields, so this is a third field, not a new shape.

**Cost.** Hours. It is the cheapest item here and probably the one that changes
the most attempts, because "carry the constant exactly" is a discipline our runs
break constantly.

### What this note does *not* propose

Not a `Cited`-axiom change. The programme re-proved Rhin and Tao from scratch —
5,960 and 124,019 lines — which is the honest thing to do and which our runtime
cannot afford. `Cited` + `conditional` remains the right trade for us. What the
comparison does show is that the *cost* of the shortcut should be legible: a
result standing on a `Cited` axiom is standing on something ProofAtlas would have
spent 124k lines to remove, and `CLAIMS.md` should say which axioms a
`conditional` verdict rests on, by name, the way the evidence records list theirs.

Nor a `native_decide` change. Our refusal is right. What is missing is a grade
*below* `formalised` for a result whose certificate is real, replayed by two
independent stacks, and not kernel-clean — with the trust boundary named and its
reduction filed as a frontier entry, exactly as the predecessor page does.
