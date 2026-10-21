# Zeilberger: the certificate, and the price of certainty

**The axis.** Thurston (`05`) says the runtime is optimising the wrong output
because a proof is not understanding. Zeilberger says it is optimising the wrong
output for the opposite reason: certainty is a purchasable good with a price,
and a runtime that always pays full price is wasting money it could spend
discovering more facts. He is the only subject in this set who *built the
machine*, published under its name, and then wrote the manifesto arguing that
the machine's cheap partial answers should count as results.

He is also the subject whose framework maps most directly onto code. His
proposal — a theorem carries a price, and prices compose under deduction — is a
schema change to `research/CLAIMS.md`, statable in a sentence.

**Source key.** **[TFP]** = Doron Zeilberger, *Theorems for a Price: Tomorrow's
Semi-Rigorous Mathematical Culture*, Notices of the AMS, October 1993, reprinted
in *The Mathematical Intelligencer*; PDF at
<https://sites.math.rutgers.edu/~zeilberg/mamarim/mamarimPDF/priced.pdf>,
also arXiv:math/9301202. Fetched and read in full; every quotation is from it.

## Accuracy conventions

- **Single primary source, fetched complete.** Zeilberger's *Opinions* series
  and his later writing were not fetched and nothing here rests on them. Where
  this file needs a position he holds elsewhere, it says so and does not quote.
- **The essay is deliberately provocative and dates from 1993.** Its
  predictions ("an abstract of a paper, c. 2100") are read here as a *design
  proposal*, which is what makes them usable, and not as forecasting to be
  scored. The distinction matters because the forecast has partly come true by
  other means — probabilistic and certificate-based methods are ordinary now —
  while the specific institution he proposes, priced theorems, has not.
- **"Shalosh B. Ekhad" is Zeilberger's computer, and he lists it as an author.**
  Publications cited in [TFP] under [E] and [ET] are attributed to it. This is
  not a joke to be quietly dropped from the record; it is the authorship
  position, and it is stated in his own footnote.
- **[TFP] is a reply to Jaffe and Quinn**, the same target as Thurston's essay
  (`05`), and cites it as [JQ]. Two of the ten subjects here are answering the
  same paper from opposite directions, which is the reason both are in the set.

---

## §A Stated method

### A1. A theorem is bought, and the price should be on the label `[STATED]`

The central proposal, and it is a data-model proposal.

> "This will happen after a transitory age of semi-rigorous mathematics, in
> which identities (and perhaps other kinds of theorems) will carry
> price-tags." — [TFP]

> "It would be then OK to rely on such a priced theorem, provided that the price
> is stated explicitly. Whenever statement A, whose price is p, and statement B,
> whose price is q, are used to deduce statement C, the latter becomes a priced
> theorem priced at p + q." — [TFP]

**Agent:** `research/CLAIMS.md` has a `status` field with a small vocabulary —
`asserted`, `established`, `catalogued`, and on this branch `Formalised` — and
it is categorical where Zeilberger's is numeric and *compositional*. The
compositionality is the part worth having: `closure.rs` already walks
`follows-from` edges to a fixed point, and a fixed-point walk that carries a
cost along each edge is the same walk with an accumulator. A claim's price would
then be derived, not asserted, which is this repository's stated preference.

What price *means* here is the open question and it should not be answered by
analogy. Zeilberger's is CPU time to full certainty. A runtime's honest
equivalents are: model calls spent, whether a kernel accepted it, how many
independent routes reached it, and how many `asserted` links the chain passes
through. The last is the cheapest and the most informative — a claim resting on
a chain containing one unverified link is not established, and today the ledger
computes standing as a minimum over `rests-on`, which is the categorical version
of exactly this.

### A2. Almost-certainty for an epsilon of the cost, by random specialisation `[STATED]`

The mechanism, and it is specific enough to implement.

The WZ proof reduces to showing an inhomogeneous linear system with symbolic
coefficients is solvable. Solving it symbolically is expensive. So:

> "By plugging in specific values for n and the other parameters, if present,
> one gets a system with numerical coefficients, which is much faster to handle.
> Since it is unlikely that a random system of inhomogeneous linear equations
> with more equations than unknowns can be solved, the solvability of the system
> for a number of special values of n and the other parameters is a very good
> indication that the identity is indeed true. It is a waste of money to get
> absolute certainty, unless the conjectured identity in question is known to
> imply the Riemann Hypothesis." — [TFP]

The last clause is the safety check, stated by him: pay full price when the
consequence is large.

**Agent:** the runtime has the ingredients and does not have the pattern. It has
`execute_command`, a Python stack, `sat_solver`, `smt_solver` and `symbolic_math`
— everything needed to instantiate a symbolic obligation at random parameter
values and check it. What is missing is a *status for the result*: an obligation
verified at forty random specialisations is not `asserted` in the pejorative
sense and is not `established` either, and there is no third thing to write. The
gap is a vocabulary gap, and it is exactly A1.

Note the connection to `01`§A5. Grothendieck also declines to pay for the last
step, on the grounds that once the statement is understood the proof is trade.
Zeilberger declines on the grounds that the evidence is already overwhelming and
the remaining cost is not worth it. Two subjects, two arguments, one missing
ledger status.

### A3. The proof is a certificate to be checked, not an argument to be followed `[STATED]`

> "Any such identity is proved by exhibiting a proof certificate, that reduces
> the proof of the given identity to that of a finite identity among rational
> functions, and hence, by clearing denominators, to that between specific
> polynomials." — [TFP]

He generalises the ambition:

> "I speculate that similar developments will occur elsewhere in mathematics,
> and will 'trivialize' large parts of mathematics, by reducing mathematical
> truths to routine, albeit possibly very long, and exorbitantly expensive to
> check, 'proof certificates'. These proof certificates would also enable us, by
> plugging in random values, to assert 'probable truth' very cheaply." — [TFP]

**Agent:** this is the design principle behind every solver already in the
image — a SAT certificate, an SMT model, a Vampire SZS status, a Lean term.
The runtime treats each as a per-tool artefact: `refute.rs` parses SZS into four
findings, `lean.rs` files a verdict under `code/out/lean/`, and there is no
common notion of *a claim's supporting certificate*. A single `certificate:`
field naming the artefact and the checker that accepted it would unify four
existing mechanisms and is the smallest schema change in this directory with a
real payoff.

### A4. Triviality is a moving property of the ambient framework `[STATED]`

The most philosophically interesting argument in the essay, and the one that
connects him to Grothendieck.

> "All the above identities are trivial, except possibly the last two, which I
> think quite likely will be considered trivial in two hundred years." — [TFP],
> where the last two are the Atiyah–Singer index theorem and the Riemann
> hypothesis

His account of why `2 + 2 = 4` is trivial: "It is a general, abstract theorem,
that contains, as special cases, many apparently unrelated theorems … It was
also realized that in order to prove it rigorously, it suffices to prove it for
any one special case, say, marks on the cave's wall" ([TFP]). And of `(a+b)³`:
it "is completely routine when viewed literally, in the syntactic sense, i.e. in
which a and b are no longer symbols denoting numbers, but rather represent
themselves, qua (commuting) literals. This shift in emphasis roughly corresponds
to the transition from Fortran to Maple" ([TFP]).

Identities 5–8 "were, until recently, considered genuine non-trivial identities,
requiring a human demonstration. … All such identities are now routinely
provable" ([TFP]) — and two of them imply the first Rogers–Ramanujan identity
and Jacobi's four-square theorem.

**Agent:** this is `01`§A4 — find the world the problem is native to — with a
different payoff. Grothendieck moves to the world where the theorem is a
consequence; Zeilberger moves to the world where it is a *decision procedure*.
Both are the same missing capability in the runtime: nothing proposes a change
of ambient setting. Zeilberger's version has the sharper trigger, because
"is this statement an instance of a decidable class?" is a question with an
answer, and the runtime has the deciders — CP-SAT, Z3, cvc5, Vampire, the
symbolic stack — and asks nothing.

### A5. Use the decision procedure to *find* identities, not only to check them `[STATED]`

The half of the WZ machinery that is a discovery method rather than a
verification method.

> "we can also use the algorithm to find new identities. If a given sum yields a
> first-order recurrence, it can be solved, as mentioned above, and the sum in
> question turns out to be explicitly evaluable. If the recurrence obtained is
> of higher order, then most likely the sum is not explicitly-evaluable (in
> closed form), and Petkovsek's algorithm, that decides whether a given linear
> recurrence (with polynomial coefficients) has closed form solutions, can be
> used to find out for sure." — [TFP]

**Agent:** the runtime has `analyze_sequence` and `find_linear_recurrence` on
`pattern_finder`, which is the entry point to exactly this, and the second half —
*decide whether the recurrence admits a closed form* — is not there. Petkovšek
is a named, implementable algorithm, and it converts "we found a recurrence"
into "there is no closed form, stop looking", which is a **no-go result**, which
is what `BANKED` was built to score. This is the most concrete unbuilt item in
this file.

### A6. The machine is an author `[STATED, by practice]`

His footnote, verbatim, is the whole position:

> "For example, my computer Shalosh B. Ekhad, and its friend Sol Tre, already
> have a non-trivial publication list" — [TFP], footnote 2

and the papers cited under [E] and [ET] carry Ekhad as author in *J. Comb. Theo.
Ser. A*.

**Agent:** the runtime's workspace commits the derivation, the programs and the
notes, which is the record of authorship, and nothing in it distinguishes what a
model wrote from what a tool computed. `Status::Formalised` is the one place
provenance is enforced. Zeilberger's position argues for extending that: a claim
should record *what produced it*, not because credit matters to a run but
because the reader's evaluation of an unverified claim depends entirely on
whether it came from a model or from `find_linear_recurrence`.

### A7. Renounce absolute certainty in exchange for volume `[STATED]`

The trade he is proposing, stated plainly.

> "In the future, not all mathematicians will care about absolute certainty,
> since there will be so many exciting new facts to discover" — [TFP]

> "As absolute truth becomes more and more expensive, we would sooner or later
> come to grips with the fact that few non-trivial results could be known with
> old-fashioned certainty. Most likely we will wind up abandoning the task of
> keeping track of price altogether, and complete the metamorphosis to
> non-rigorous mathematics." — [TFP]

**Agent:** the last sentence is Zeilberger arguing against his own proposal's
stability, and it should be read as the warning it is. A runtime that adds a
cheap status will find that most of its ledger acquires that status, because the
cheap status is cheap. Any implementation of A1 must therefore make the *price
visible at the point of use* — the derived ledger must render a chain's total
cost beside the claim, not only the claim's own — or the mechanism decays into
what he predicts here. `claims.rs` already computes standing as a minimum over
`rests-on`, which is the structure that resists the decay, and is the reason a
numeric price should be derived the same way rather than stored per claim.

---

## §B Anatomy

### B1. The WZ method (Wilf–Zeilberger, c. 1990)

**(a)** Prove hypergeometric identities — sums `Σ_k F(n,k)` where
`F(n+1,k)/F(n,k)` and `F(n,k+1)/F(n,k)` are rational in `(n,k)`. The class
covers "most of the identities between the classical special functions of
mathematical physics" ([TFP]).

**(b)** Stop proving identities and prove a *theorem about the class*. The
Fundamental Theorem of Algorithmic Hypergeometric Proof Theory asserts that for
any proper hypergeometric term there exist polynomials `p_i(n)` and rational
functions `R_j` such that a telescoping relation holds; hence the sum satisfies
a linear recurrence with polynomial coefficients ([TFP]). Proving an identity
then reduces to: derive both sides' recurrences, check they agree, check finitely
many initial values.

**(c)** Gosper's algorithm and creative telescoping from symbolic computation;
Petkovšek's algorithm for closed-form solutions of such recurrences; Maple as
the substrate. He locates the whole shift as "the transition from Fortran to
Maple, i.e. from numeric computation to symbolic computation" ([TFP]).

**(d)** Everything. The certificate is machine-produced and machine-checkable,
and the checking reduces to polynomial identity. Ekhad's one-line proof of
Dixon's theorem and the Rogers–Ramanujan verification are the published outputs
([E], [ET], via [TFP]).

**(e)** The ladder is the interesting part. Identities 5–8 in his list "were,
until recently, considered genuine non-trivial identities, requiring a human
demonstration", and he singles out Cartier–Foata's "particularly nice human
proof" of one of them ([TFP]). Two of the four imply Rogers–Ramanujan and
Jacobi's four-square theorem. So a decision procedure retired a class of results
that had individually been considered research-level, including at least one with
an admired human proof.

The honest limit is stated too: "It is easy, however, to concoct artificial
examples for which the running time, and memory, are prohibitive" ([TFP]).

**(f) MOVE — promote the problem to its class and look for a decision
procedure.** *Trigger:* the goal is an instance of a syntactically
characterisable family. *Action:* ask whether the family is decidable rather
than whether this instance is provable; if it is, run the decider and keep the
certificate. *Check:* membership in the family must be checkable *syntactically*
— "proper hypergeometric term" is a syntactic condition — or the promotion is a
guess. Where the decider is too expensive on this instance, fall back to A2's
random specialisation and record that you did.

### B2. Random specialisation as a cost-reduction (the same paper)

**(a)** Even inside the decidable class, the full proof may cost more than it is
worth: the symbolic linear system is "very time-consuming to solve".

**(b)** Instantiate. Plug in specific `n` and parameters, solve numerically,
repeat. "Since it is unlikely that a random system of inhomogeneous linear
equations with more equations than unknowns can be solved, the solvability of
the system for a number of special values … is a very good indication that the
identity is indeed true" ([TFP]).

**(c)** Nothing beyond linear algebra. The essay notes the family resemblance to
transparent proofs and probabilistically checkable proofs from theoretical
computer science, and to Gödel's speed-up results — short theorems with
arbitrarily long proofs ([TFP], and its footnote 3).

**(d)** By definition, all of it.

**(e)** No ladder; it is a technique, not a result. Its status in 1993 was a
provocation and its status now is unremarkable practice — which is itself the
finding.

**(f) MOVE — separate "we know how to prove this" from "we have proved this".**
*Trigger:* the run has a proof *procedure* whose execution is too expensive.
*Action:* record the obligation, the procedure that would discharge it, and the
cost estimate, then discharge it probabilistically and continue. *Check:* the
statement must be marked as carrying an outstanding obligation, and everything
downstream must inherit the mark — which is A1's composition rule and is the
only thing preventing this move from being ordinary sloppiness.

---

## §C Against Tao

| Tao (`../tao/01`) | Zeilberger | Which, when |
|---|---|---|
| §21–23 a proof is what the kernel accepted; everything else is a reason to believe | A1/A2: reasons to believe have prices, and paying for the kernel is sometimes irrational | Not opposed as sharply as it reads. Both want the *distinction recorded*. Tao wants two statuses; Zeilberger wants a number. The runtime has the two statuses and no number |
| §20 numerics before theory | A2: numerics *instead of* theory, when the theory is priced out | Zeilberger goes further than anyone in the set. The safety check is his: pay full price when the consequence is large |
| §2 count your debts and abandon if there are several | A1: debts compose additively and you carry them | Directly complementary. Tao counts debts to decide whether to continue; Zeilberger prices them so a result can ship carrying them |
| §16 record which techniques are known not to apply | A5/Petkovšek: *decide* that no closed form exists | The strongest form of the same idea, and the only one in this directory that is an algorithm rather than a discipline |
| §1 turn off nine of ten difficulties | A4: or move to the framework in which they are not difficulties | `weakener` does Tao's. Nothing does Zeilberger's, and his has a mechanical trigger where `01`§A4's does not |

**The one-line version.** Tao and the branch's `lean_check` treat verification as
binary. Thurston (`05`) attacks that from the side of meaning. Zeilberger
attacks it from the side of cost, and his attack comes with a data model: prices
compose along deduction. Of the two attacks, his is the one the runtime could
implement next week, because `claims.rs` already folds standing along
`rests-on` and `closure.rs` already walks the graph.

---

## Sources

- Doron Zeilberger, *Theorems for a Price: Tomorrow's Semi-Rigorous
  Mathematical Culture*, Notices AMS, October 1993 —
  <https://sites.math.rutgers.edu/~zeilberg/mamarim/mamarimPDF/priced.pdf>
  (fetched and read in full; sole source)
- Index of Zeilberger's papers, consulted for provenance and not quoted —
  <https://sites.math.rutgers.edu/~zeilberg/papers1.html>

Cited within [TFP] and not fetched — `[UNVERIFIED]` as sources: Wilf &
Zeilberger's [WZ2]; Petkovšek's algorithm [P]; Gosper; Cartier's Bourbaki exposé
[Ca]; Cartier & Foata [CF]; Ekhad [E] and Ekhad & Tre [ET]; Almkvist &
Zeilberger [AZ]; Arora et al. on probabilistically checkable proofs [ALMSS],
[AS]; Jaffe & Quinn [JQ].

Noted and not used: *Proofs for a price: Tomorrow's ultra-rigorous mathematical
culture*, Bull. AMS 61 (2024) — a direct answer to [TFP] published thirty-one
years later, <https://www.ams.org/journals/bull/2024-61-03/S0273-0979-2024-01823-0/viewer/>.
Worth reading before any proposal in this file is built; it was not fetched
here.
