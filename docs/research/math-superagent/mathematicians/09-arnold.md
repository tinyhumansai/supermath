# Arnold: mathematics is the part of physics where experiments are cheap

**The axis.** The counterweight to everything Grothendieck and Scholze stand
for, and the only subject here who attacks the deductive method itself. Arnold's
position is that mathematics is an experimental science, that a long chain of
deductions is *less* reliable rather than more, and that the correct discipline
is external control by examples — the same discipline as in physics. He calls
the alternative "criminal".

He is in the set because the branch built `lean_check` and gave the kernel
authority, and this is the strongest available argument that the kernel is the
wrong place to put it. He is also, unexpectedly, the strongest argument *for*
the runtime's method policy, which opens by demanding a naive oracle.

**Source key.** **[OTM]** = V. I. Arnold, *On teaching mathematics*, extended
text of an address at the Palais de Découverte, Paris, 7 March 1997,
<https://www.karlin.mff.cuni.cz/~spurny/doc/articles/arnold.htm>. Fetched, HTML
stripped, and read in full; every quotation is from it.

## Accuracy conventions

- **Single source, fetched complete, read directly.** No summarising fetch, no
  paraphrase chain.
- **This is a polemic and reads like one.** Arnold calls his opponents
  "criminal algebraists-axiomatisators" and their work "ugly scholastic
  pseudo-mathematics" ([OTM]). The rhetoric is his and is reproduced where it
  carries the argument, but a proposal citing this file should rest on his
  *mechanisms* — external control by examples, the reliability argument about
  chain length — and not on his invective.
- **It is a talk about teaching, used here as evidence about research.** Arnold
  makes that move himself — "here as so often he sees math research, exposition,
  and teaching as all the same" is McLarty's line about Grothendieck (`01`), and
  it is at least as true of Arnold — but the extension is this file's and is
  flagged.
- **The essay is in translation** and the translator is not named on the page.
  Wording that carries an argument should be treated as approximately his.
- **Arnold's mathematical results are not covered.** KAM theory and Hilbert's
  13th are alluded to nowhere below, because no fetchable first-person account
  of *how he found them* was reached. §B is therefore built from the essay's own
  worked examples, which is honest but is a weaker §B than the other files have.

---

## §A Stated method

### A1. Mathematics is experimental, and its experiments are cheap `[STATED]`

The opening, and the whole position in three sentences.

> "Mathematics is a part of physics. Physics is an experimental science, a part
> of natural science. Mathematics is the part of physics where experiments are
> cheap." — [OTM]

> "The Jacobi identity (which forces the heights of a triangle to cross at one
> point) is an experimental fact in the same way as that the Earth is round
> (that is, homeomorphic to a ball). But it can be discovered with less
> expense." — [OTM]

**Agent:** the runtime's method policy already directs the first step to a naive
oracle, and `../tao/02`'s reading of eleven Tao programmes says computation was
decisive in exactly one and decorative in most of the rest — which reads as an
argument against the policy. Arnold is the counter-argument, and it is not the
one the runtime's policy actually makes. The policy builds an oracle *to check
the goal*. Arnold's experiments are performed to *discover what is true*, which
is `07`§A3 and the Ramanujan use. Three subjects now separate goal-checking
computation from discovery computation, and the runtime only does the first.

### A2. The research cycle is observe → conjecture → hunt counterexamples → formulate `[STATED]`

His explicit scheme, and it is a loop the runtime could run.

> "The scheme of construction of a mathematical theory is exactly the same as
> that in any other natural science. First we consider some objects and make
> some observations in special cases. Then we try and find the limits of
> application of our observations, look for counter-examples which would prevent
> unjustified extension of our observations onto a too wide range of events" —
> [OTM]

His example is chosen to sting, and it is worth reproducing because it is the
argument for the step: the number of partitions of `1, 3, 5, 7, 9` into an odd
number of natural summands gives `1, 2, 4, 8, 16` — "but then comes 29" ([OTM]).

> "As a result we formulate the empirical discovery that we made (for example,
> the Fermat conjecture or Poincaré conjecture) as clearly as possible. After
> this there comes the difficult period of checking as to how reliable are the
> conclusions." — [OTM]

**Agent:** the counterexample hunt is placed *between* observation and
formulation, not after the proof attempt. The refuter arm on this branch runs
against open gaps and the current weakened rung — that is, against statements
the run is already trying to prove. Arnold's placement is earlier and cheaper:
it bounds a pattern before the pattern is promoted to a goal.
`pattern_finder` produces the observations and has no refutation step at all,
which means every pattern it emits enters the ledger untested. `1, 2, 4, 8, 16,
29` is precisely what that failure looks like.

### A3. Longer proofs are less reliable, not more `[STATED]`

The reliability argument, and it is the sharpest attack on formalism in this
directory because it is quantitative in shape rather than aesthetic.

> "a small change in axioms (of which we cannot be completely sure) is capable,
> generally speaking, of leading to completely different conclusions than those
> that are obtained from theorems which have been deduced from the accepted
> axioms. The longer and fancier is the chain of deductions ('proofs'), the less
> reliable is the final result." — [OTM]

> "Complex models are rarely useful (unless for those writing their
> dissertations)." — [OTM]

**Agent:** this bears directly on `closure.rs`, built on this branch to close the
claim ledger under `follows-from` to a fixed point — deliberately not stopping
at one hop, because "stopping at one hop discards every sound step above the
first, which is most of the 37×". Arnold's objection is that a long derived
chain inherits the uncertainty of every axiom it passes through, so a claim
established at depth twelve is not the same object as one established at depth
one, and the derived ledger presents them identically.

The fix is small and follows from `06`§A1: `research/ENTAILMENT.md` should carry
*depth* beside standing. `claims.rs` already folds standing as a minimum over
`rests-on`; carrying a maximum depth along the same walk costs nothing and makes
Arnold's objection visible instead of arguable.

### A4. Control yourself by examples, or half your signs will be wrong `[STATED]`

The practical version, and it is an empirical claim about error rates in human
work.

> "Every working mathematician knows that if one does not control oneself (best
> of all by examples), then after some ten pages half of all the signs in
> formulae will be wrong and twos will find their way from denominators into
> numerators." — [OTM]

> "The technology of combatting such errors is the same external control by
> experiments or observations as in any experimental science and it should be
> taught from the very beginning" — [OTM]

He extends it past human fallibility on purpose: "one cannot forget about the
inevitability of logical mistakes in long arguments (say, in the form of a
computer breakdown caused by cosmic rays or quantum oscillations)" ([OTM]).

**Agent:** this is a *different* control from `lean_check` and the runtime has
neither in the form Arnold means. Lean checks that a proof follows; Arnold wants
the *statement* checked against instances, continuously, mid-derivation. The
runtime has the tooling — `execute_command`, the Python stack, `/workspace/code`
on `PYTHONPATH` — and no discipline requiring it. `reflection`'s `SOLVED` demands
"an executable program on disk", which is this check applied once at the end. His
claim is that once at the end is ten pages too late.

Compare `06`§A2: Zeilberger's random specialisation is exactly Arnold's external
control, made into a formal method with a cost argument attached. The two
subjects most opposed on the value of rigour agree completely on this mechanism.

### A5. Formalisation is a modelling step, and the model is not the thing `[STATED]`

The philosophical core, and it is the passage a formalisation-first runtime
should have to answer.

> "When constructing a model, the following idealisation is made: certain facts
> which are only known with a certain degree of probability or with a certain
> degree of accuracy, are considered to be 'absolutely' correct and are accepted
> as 'axioms'. The sense of this 'absoluteness' lies precisely in the fact that
> we allow ourselves to use these 'facts' according to the rules of formal
> logic, in the process declaring as 'theorems' all that we can derive from
> them." — [OTM]

> "The mathematical technique of modelling consists of ignoring this trouble and
> speaking about your deductive model in such a way as if it coincided with
> reality." — [OTM]

**Agent:** the runtime already knows this in one place and has not generalised
it. `ContradictoryAxioms` — the Vampire status that justified the refuter arm —
is exactly Arnold's failure: everything follows from a broken axiomatisation, so
a bad encoding looks like a triumph. The branch turned that from a prompt
instruction into a status the runtime reads, for the refuter. Nothing does the
equivalent for `lean_check`: a Lean file that compiles proves what its
*statement* says, and whether the statement encodes the intended mathematics is
unchecked. `02`§B1 records that exact failure happening — "one checks the final
main statement for accuracy to ensure it proved what was intended" was a manual
step, performed by a human, and it caught a misread quantifier.

### A6. Prefer the concrete definition that a person can hold `[STATED]`

His running example is the smooth manifold. Against "a topological space which
satisfies a long series of axioms" he sets Poincaré's:

> "A smooth k-dimensional submanifold of the Euclidean space R^N is its subset
> which in a neighbourhood of its every point is a graph of a smooth mapping of
> R^k into R^(N-k)" — [OTM]

and observes that Whitney's theorem makes the abstract notion redundant: "There
are no 'more abstract' finite-dimensional smooth manifolds in the world"
([OTM]). His verdict on the alternative: "It is impossible to understand an
unmotivated definition but this does not stop the criminal
algebraists-axiomatisators" ([OTM]).

**Agent:** Grothendieck's A3 says take the definition that carries no
hypotheses; Arnold says take the one a reader can picture, and cites a theorem
proving the two are equivalent here. The runtime has no notion of a definition's
*form* at all — `definitions.rs` exists and stores them. Where this becomes
operational is `05`§A3: Thurston's seven derivatives. If a claim recorded which
formulation of a definition it is working with, Arnold's preference and
Grothendieck's would be a *choice the run makes* rather than an accident of
which role wrote the note.

### A7. Value the connection between unlike things above the theorem `[STATED]`

His account of what is worth teaching, and it is a statement about what a result
is *for*.

> "These discoveries of connections between heterogeneous mathematical objects
> can be compared with the discovery of the connection between electricity and
> magnetism in physics or with the discovery of the similarity between the east
> coast of America and the west coast of Africa in geology." — [OTM]

> "Jacobi noted, as mathematics' most fascinating property, that in it one and
> the same function controls both the presentations of a whole number as a sum
> of four squares and the real movement of a pendulum." — [OTM]

And on the Riemann-surface facts he was taught as a first-year student: "even
given without any proofs) they give a better and more correct idea of modern
mathematics than whole volumes of the Bourbaki treatise" ([OTM]).

**Agent:** "even given without any proofs" is Ramanujan's position (`07`§A1)
arriving from a hostile direction, and the cross-domain connection is what the
runtime's `inventor` is nominally for. `research/APPROACHES.md` records an idea
with a `mechanism` and a `precedent`; nothing records that two *established*
claims in the ledger are instances of one phenomenon. `closure.rs` finds
implications. Arnold is asking for analogies, which are not sound and therefore
belong wherever `07`§B1's conjecture store belongs.

### A8. The Arnold Principle `[STATED]`

Included because it is a real caution about citation.

> "The Arnold Principle. If a notion bears a personal name, then this name is
> not the name of the discoverer." … "The Berry Principle. The Arnold Principle
> is applicable to itself." — [OTM]

**Agent:** `research/CLAIMS.md` has a `catalogued` status precisely so a lookup
may confirm an answer and never be the reason for one. The Arnold Principle
sharpens it: a named theorem's name is not evidence about its provenance, so a
`catalogued` claim whose citation rests on a name rather than a document is
weaker than the ledger renders it. `research/FRONTIER.md` stores the citing
sentence for each anchor, which is the right raw material and is not used this
way.

---

## §B Anatomy

§B here is built from the essay's worked examples rather than from Arnold's
research papers — see the accuracy conventions. Both examples are chosen by him
to make a methodological point, which is what makes them usable.

### B1. `1, 2, 4, 8, 16, 29` — the counterexample as a scheduled step

**(a)** Count the partitions of consecutive odd numbers `1, 3, 5, 7, 9` into an
odd number of natural summands.

**(b)** No reframing. The point is the shape of the data: `1, 2, 4, 8, 16` — and
then `29` ([OTM]).

**(c)** Nothing.

**(d)** Everything. It is an experiment, and the sixth term is the result.

**(e)** No ladder. Arnold uses it as the canonical instance of the failure his
scheme's third step prevents.

**(f) MOVE — bound the pattern before promoting it.** *Trigger:* an observed
regularity is about to be recorded as a conjecture or used to direct search.
*Action:* extend the computation past the point where the pattern was noticed,
by a margin, specifically hunting for the break. *Check:* the extension must
reach a genuinely new regime, not merely more of the same — five terms of a
sequence that doubles are consistent with at least three closed forms, and the
sixth is where they part. For the runtime this is `analyze_sequence` and
`find_linear_recurrence` gaining an obligation rather than a new capability.

### B2. The smooth manifold — two definitions of one object

**(a)** Define a smooth manifold.

**(b)** Two answers: Veblen's abstract axioms, and Poincaré's submanifold-of-`R^N`
formulation, which Arnold reproduces in one sentence ([OTM]).

**(c)** Whitney's embedding theorem, which is what makes the choice a matter of
taste rather than of content — "An 'abstract' smooth manifold is a smooth
submanifold of a Euclidean space considered up to a diffeomorphism" ([OTM]).

**(d)** Nothing.

**(e)** The historical claim is his and is sharp: Poincaré's *Analysis Situs*
already contains "an absolutely clear definition of a smooth manifold which is
much more useful than the 'abstract' one", and the abstract version arrived only
in the late 1920s ([OTM]).

He also reports a teaching experiment as evidence: staying "as close as possible
to physics", in half a year he took Moscow schoolchildren to the Abel theorem on
the unsolvability of the general quintic, covering complex numbers, Riemann
surfaces, fundamental groups and monodromy on the way ([OTM]).

**(f) MOVE — carry more than one formulation of a definition, and choose per
use.** *Trigger:* a definition the run depends on admits both a concrete and an
abstract form, provably equivalent. *Action:* record both and the theorem
relating them; use the concrete one for computation and counterexample hunting,
the abstract one where generality is being exploited. *Check:* the equivalence
must be a theorem the run can cite, not an assumption — without Whitney, this
move is a category error rather than a choice.

---

## §C Against Tao

| Tao (`../tao/01`) | Arnold | Which, when |
|---|---|---|
| §20 numerics before theory | A1/A2/A4: numerics *throughout*, as the only reliable control | Arnold is Tao's strongest ally in this set on this one point, and goes further than Tao does. The runtime does the weak version, once, at the end |
| §21–23 a proof is what the kernel accepted | A3/A5: a long chain is less reliable, and a formal model is not the thing modelled | The sharpest disagreement in the directory. But note A5 is what justified the refuter arm's `ContradictoryAxioms` handling — the branch already agrees with him in one place |
| §10 look for a counterexample first | A2: and place the hunt between observation and conjecture, not after | Same move, earlier. The runtime's refuter runs on statements it is already committed to |
| §5 the seven named ways to modify a problem | A6/B2: and the seven ways to state a *definition*, which nobody enumerates | The gap this exposes is real: the runtime mutates goals and never formulations |
| §27 archive everything | A8 the Arnold Principle: the archive's names are unreliable evidence about provenance | A caution on `catalogued` claims, cheap to act on given `FRONTIER.md` already stores citing sentences |

**The one-line version.** Every control the branch added this year checks that a
derivation is *sound*. Arnold's claim is that the dangerous error is not
unsoundness but a true derivation from a statement that does not mean what the
run thinks — and the only defence against that is to keep testing the statement
against instances, all the way through, which the runtime does exactly once.

---

## Sources

- V. I. Arnold, *On teaching mathematics*, Palais de Découverte, Paris,
  7 March 1997 —
  <https://www.karlin.mff.cuni.cz/~spurny/doc/articles/arnold.htm> (fetched and
  read in full; sole source; translator not named on the page)

Other hosted copies located and not separately fetched:
<https://engineering.purdue.edu/CEMT/article-on-math.html>,
<https://www.maia.ub.es/~vieiro/fitxers/teaching-math-arnold.pdf>.

Not reached — `[UNVERIFIED]` as sources, and the gap that makes §B here weaker
than the other files': any first-person account by Arnold of how he found KAM
theory or his solution of Hilbert's 13th problem. *Arnold's Problems* is the
obvious next source for a study of his problem-*posing*, which would pair
naturally with `02`.
