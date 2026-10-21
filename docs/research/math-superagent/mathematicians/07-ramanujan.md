# Ramanujan: the conjecture generator, and what a wrong one is worth

**The axis.** Every other subject produces arguments. Ramanujan produced
*statements* — thousands of them, without proof, at a hit rate nobody has
matched, and a century of other people's careers has gone into checking them.
He is the pure case of the thing the runtime's `pattern_finder` gestures at and
does not do: generate candidate truths from experiment, at volume, and let
verification be a separate, later, differently-resourced activity.

He also supplies the only measured answer in this directory to a question the
runtime cannot currently ask: **what is a false conjecture worth?**

**Source keys.** **[LN]** = *Ramanujan's lost notebook*,
<https://en.wikipedia.org/wiki/Ramanujan's_lost_notebook>; **[BDRZ]** = Berndt,
Dixit, Roy and Zaharescu, *New pathways and connections in Number Theory and
Analysis motivated by two incorrect claims of Ramanujan*, arXiv:1608.03670,
<https://arxiv.org/abs/1608.03670>.

## Accuracy conventions

This is the thinnest-sourced file in the directory and the section is long in
proportion.

- **Ramanujan wrote essentially nothing about his method.** §A is therefore
  almost entirely `[INFERRED]`, read off the artefacts and off what his editors
  report, and this is stated per entry rather than assumed. Per
  [`00-conventions.md`](00-conventions.md), manufacturing §A entries for a
  subject who left no methodological writing would be inventing the finding.
- **The goddess Namagiri attribution is not used.** The claim that formulas came
  to him in dreams from a family deity is widely repeated, is at best
  second-hand, and carries no operational content whatever. It is named here so
  it is not mistaken for an omission.
- **Hardy's "2/3" estimate is reported at second hand and is contested by the
  people who did the work.** It is used below *with* its correction, because the
  correction is the interesting number.
- **Berndt's "Beethoven's tenth symphony" line is quoted via [LN]**, not from
  Berndt's own text, which was not reached — the Royal Society article
  *Living with Ramanujan for 40 years* returned HTTP 403.
- **No count of Ramanujan's errors is given here as a rate**, because none was
  found in a fetchable source. Berndt and Chan's *Questionable claims found in
  Ramanujan's lost notebook* exists and was not fetched; a proposal needing an
  error rate must get it from there, not from this file.

---

## §A Method, read off the artefacts

### A1. Record the statement; do not record the derivation `[INFERRED]`

The notebooks are the evidence. The lost notebook alone is "more than one
hundred pages written on 138 sides" with "over six hundred mathematical formulas
listed consecutively without proofs" ([LN]). The earlier notebooks are larger
and the same in kind. Andrews and Berndt then spent from 2005 to 2018 across
five volumes supplying proofs ([LN]).

**Agent:** the runtime's ledgers are built on the opposite assumption. A claim
block requires a `status`, a `bearing` and an `anchor`; a `research/backward`
gap requires a first move; `research/approaches` requires a mechanism. Every
schema demands justification structure at the moment of writing. There is no
place to put six hundred statements believed for reasons the author is not going
to write down — and `note_scratch` is not it, because scratch is deliberately
unreachable from durable recall.

That design is right for what it protects against and it forecloses this. The
question the runtime should be able to answer and cannot is: *has this run
produced any statements it believes and cannot yet justify?* Today the answer is
always no, by construction, because there is nowhere to write one.

### A2. Volume with selection, not volume alone `[INFERRED]`

The important qualification, and it is what separates this from noise
generation. Hardy, asked what proportion of the notebook entries were
rediscoveries of known results, estimated two thirds. Berndt and his
collaborators, having spent twenty years proving them, found that "well over
1/2 of the results were new" — Hardy's estimate is reported as too high
(summarised, from search results; see conventions).

Take the weaker of the two figures and it is still remarkable: a majority of
hundreds of unproved assertions, produced without literature access, were both
true and new.

**Agent:** this is the number any generative arm has to be measured against, and
it is the number that makes A1 tolerable. A generator with a 5% hit rate that
files its output as `asserted` poisons the ledger; one with a 50% hit rate is
the most valuable thing in the system. So the requirement is not "add a
conjecture arm" but "add a conjecture arm *with its hit rate recorded from the
first run*". `research/CLAIMS.md` already distinguishes `asserted` from
`established`, so the promotion rate of a generator's output is derivable from
existing data if the generator's claims are tagged with their origin — which
returns to `06`§A6, provenance on a claim.

### A3. Numerical experiment as the discovery instrument `[INFERRED]`

The content is the evidence: q-series and mock theta functions, "modular
equations and singular moduli" at roughly a third of the lost notebook, plus
"integrals, Dirichlet series, congruences, and asymptotics" ([LN]). These are
domains where a statement can be *tested* to high precision cheaply and where an
identity that holds to twenty terms is very unlikely to be accidental.

**Agent:** the runtime has this instrument and points it at the wrong target.
`analyze_sequence`, `find_linear_recurrence` and `oeis_lookup` on
`pattern_finder` are exactly the tooling, and the method policy directs the run
to build a naive oracle *for the goal*. Ramanujan's use is different: compute in
the neighbourhood and see what identities fall out, with no goal in hand. That
is `../tao/04b`'s conjecture-generation literature — Graffiti, TxGraffiti, the
Ramanujan Machine — and the Dalmatian triviality filter remains the cheapest
unbuilt item across both research directories.

### A4. A conjecture is worth stating before it is worth believing `[INFERRED]`

The strongest evidence in this file, and it is a published paper title.

[BDRZ] — Berndt, Dixit, Roy and Zaharescu — takes three pages of the lost
notebook and reports:

> "We focus on three pages in Ramanujan's lost notebook, pages 336, 335, and
> 332, in decreasing order of attention. On page 336, Ramanujan proposes two
> identities, but the formulas are wrong -- each is vitiated by divergent
> series." — [BDRZ], abstract

The paper's own title is what matters: *New pathways and connections in Number
Theory and Analysis motivated by two incorrect claims of Ramanujan*. What the
wrong claims produced, per [BDRZ]: corrected convergent formulations, an
extension of the Voronoi summation formula, generalisations involving `σ_s(n)`,
new Bessel and Lommel identities, and a new class of integral transforms the
authors name Koshliakov transforms.

Two false statements, a hundred years old, generated a research paper's worth of
true ones.

**Agent:** the runtime destroys this. A refuted claim in `research/CLAIMS.md` and
a `refuted` entry in `research/APPROACHES.md` are terminal states — the refuter
arm on this branch files a verdict and the matter ends. Nothing asks the
question [BDRZ] asks: *what would have to be true for this to have been nearly
right, and is that thing true?* A refutation that produces a corrected statement
is a different and better outcome than a refutation that produces a dead row,
and the ledger cannot tell them apart.

This is also `04`§B1 — Wiles's abandoned Iwasawa approach — arriving from a
third direction. Three subjects now say the same thing: the runtime's absorbing
states absorb too much.

### A5. Nobody could use the output until someone else did the proofs `[INFERRED]`

The cost side, and it is enormous. Andrews found the manuscript in 1976 in the
Wren Library, in Watson's papers, catalogued as "A 139 page manuscript by
S. Ramanujan on q-series" ([LN]). Berndt's assessment of the find is quoted via
[LN]: "The discovery of this 'Lost Notebook' caused roughly as much stir in the
mathematical world as the discovery of Beethoven's tenth symphony would cause in
the musical world."

The five volumes of proofs ran from 2005 to 2018 ([LN]). Berndt's earlier work
on the ordinary notebooks took about twenty years.

**Agent:** the honest counterweight to A1–A4. An unproved-statement store is only
valuable if something eventually consumes it, and the consumption is the
expensive part. A runtime that files conjectures and never returns to them has
built a landfill. So any implementation must pair the store with a *consumer* —
the natural one is the refuter arm, which already runs on a cadence against open
gaps and could run against the conjecture store instead of only against the
current statement.

---

## §B Anatomy

### B1. The lost notebook (written c. 1919–20; rediscovered 1976)

**(a)** No problem was posed. This is the entry's defining feature and the
reason it is here: the artefact is an *unprompted* output.

**(b)** No reframing, because there was no frame. Over six hundred formulas
listed consecutively, without proofs ([LN]).

**(c)** Not determinable from the artefact, which is exactly the finding of A1 —
the imports are invisible because the derivation is absent.

**(d)** Not recorded, and almost certainly extensive. The domains — q-series,
modular equations, singular moduli — are ones where numerical verification is
cheap and where the *pattern* of what he asserted implies computation was done.
This is an inference and is labelled as one.

**(e)** The ladder is a *consumption* ladder rather than a discovery one, and it
is unique in this directory for that reason:

| Date | Event |
|---|---|
| c. 1919–20 | Written |
| 1976 | Andrews finds it in Watson's papers at the Wren Library ([LN]) |
| 2005–2018 | Andrews and Berndt publish five volumes of proofs ([LN]) |
| ongoing | Mock theta functions turn out to bear on black hole entropy ([LN]) |

Roughly ninety years from statement to systematic verification, and the
application that made the mock theta functions famous arrived from physics,
decades after that.

**(f) MOVE — separate generation from justification, and resource them
differently.** *Trigger:* a run's instruments can cheaply test statements in a
domain. *Action:* generate and record candidate statements without requiring a
derivation, tagged by generator and by the evidence that supports them.
*Check:* three, all mandatory. The store must be unreachable from anything that
derives an established ledger; the generator's promotion rate must be recorded
from the first run (A2); and a consumer must exist that actually attacks the
store (A5), or the store is a landfill.

### B2. Pages 335–336 — the productive error (asserted c. 1920, resolved 2016)

**(a)** Two identities asserted on page 336 of the lost notebook, bearing on the
extended divisor problem.

**(b)** The reframing is [BDRZ]'s, ninety-odd years later: do not discard the
claims, *repair* them. Both "are wrong -- each is vitiated by divergent series"
([BDRZ]), and the paper concentrates on correcting one.

**(c)** Voronoi summation; Bessel and Lommel functions; classical circle and
divisor problems; Koshliakov's work, which supplies the name for the new
transforms ([BDRZ]).

**(d)** Not stated in the abstract.

**(e)** ~96 years from the false claim to the paper it motivated. The output per
[BDRZ]: corrected convergent series, new series and integral identities, a
generalisation of Voronoi summation, and the Koshliakov transforms as a novel
class.

**(f) MOVE — repair the refuted statement rather than filing it.** *Trigger:* a
claim is refuted, whether by the refuter arm, a counterexample, or a solver.
*Action:* before writing the refutation, ask what the nearest true statement is
— which hypothesis, convergence condition or quantifier would have to change —
and file *that* as a new claim. *Check:* the repair must be a genuine weakening
or correction that the original refutation does not also kill, and it enters as
`asserted` like any other conjecture. The failure mode is obvious and must be
guarded: an unconstrained repair loop will weaken any statement until it is
vacuous, so the repaired statement must still entail something the run wanted.

---

## §C Against Tao

| Tao (`../tao/01`) | Ramanujan | Which, when |
|---|---|---|
| §10 look for the counterexample first | A4/B2: and when you find it, extract the corrected statement | The refuter arm implements Tao's half. B2 is the missing second half, and it is where the value was in the one measured case |
| §19 a short proof of a famous problem raises the prior it is known | A2: over half of hundreds of unproved assertions were both true and *new* | A useful bound on the prior. The novelty check is still right; Ramanujan is evidence that "surely someone has done this" is not always the way to bet |
| §21–23 a proof is what the kernel accepted | A1: six hundred formulas, no proofs, mostly right | The sharpest disagreement available. Reconciled by A5: the notebooks were *useless* for ninety years except as a work queue. That is exactly what an unproved-claim store is, and it is not nothing |
| §16 record which techniques are known not to apply | A4: record what a *wrong claim* nearly got right | Same ledger, opposite direction. Immunity records a closed door; a repaired conjecture records a door that was almost open |
| §20 numerics before theory | A3: numerics *as* the theory-generator, with no goal in hand | Tao's is goal-directed reconnaissance; Ramanujan's is undirected exploration. The runtime does the first and `pattern_finder` is one policy change from the second |

**The one-line version.** The runtime can only write down things it can justify,
so it can never be wrong on the record — and the one measured case in this file
says that a wrong statement, kept, was worth a paper. The gap is not a missing
generator; it is a missing *place to put an unjustified belief* and a missing
consumer for it.

---

## Sources

Fetched for this file:

- *Ramanujan's lost notebook* —
  <https://en.wikipedia.org/wiki/Ramanujan's_lost_notebook> (page and formula
  counts, subject breakdown, the 1976 rediscovery, the Berndt quotation)
- Berndt, Dixit, Roy, Zaharescu, *New pathways and connections in Number Theory
  and Analysis motivated by two incorrect claims of Ramanujan*,
  arXiv:1608.03670 — <https://arxiv.org/abs/1608.03670> (abstract; the wrong
  identities and what they produced)

Attempted and not reached — `[UNVERIFIED]` as sources:

- Bruce C. Berndt, *Living with Ramanujan for 40 years*, Phil. Trans. R. Soc. A
  378 (2020) 20180437 — HTTP 403. This is where the new-versus-rediscovered
  figures and Berndt's own account of Ramanujan's method properly live, and the
  A2 figures here came through a search summary instead
- K. Srinivasa Rao, chapter 3 on the notebooks,
  <https://www.imsc.res.in/~rao/ramanujan/images/KSRchap3.pdf> — fetched but not
  text-extractable
- Berndt & Chan, *Questionable claims found in Ramanujan's lost notebook* — the
  correct source for any error-rate figure, not fetched here
- Andrews & Berndt, *Ramanujan's Lost Notebook*, Parts I–V (Springer, 2005–2018)

Named as apocryphal and unused: the goddess Namagiri attribution; "an equation
means nothing to me unless it expresses a thought of God", which circulates
without a primary source.
