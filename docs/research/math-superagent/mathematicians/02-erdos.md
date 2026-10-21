# Erdős: the problem as the unit of work

**The axis.** Every other subject in this set is studied for how they *solved*
something. Erdős is here for how he decided what to work on, and for the fact
that his output was substantially a *catalogue*, priced, distributed, and still
being consumed thirty years after his death. He is also the only subject whose
corpus has been attacked by an LLM under public scrutiny, with the failures
documented, which makes his §B the most directly usable material in this
directory.

**Source keys.** **[EP]** = the Erdős Problems database,
<https://www.erdosproblems.com/>, maintained by Thomas Bloom; **[EP/FAQ]** =
<https://www.erdosproblems.com/faq>; **[EP/prizes]** =
<https://www.erdosproblems.com/prizes>; **[Fav]** = Paul Erdős, *Some of my
favorite problems and results*, 1997, as quoted verbatim on [EP]'s front page;
**[728]** = Kevin Barreto, *Problem 728 and the use of AI on Erdős problems*,
26 January 2026, <https://www.erdosproblems.com/forum/thread/blog:2>.

All figures below were read off the live site and are stamped with the date they
were read, because they move.

## Accuracy conventions

Erdős is the most anecdote-encrusted mathematician in this set. Almost every
line attributed to him circulates without a citation.

- **"My brain is open" is not sourced here and is not used as evidence.** It is
  universally repeated and no primary source for it was reached. It appears
  below only as a named apocryphon.
- **"The Book" is not used as evidence about method.** The proof-from-The-Book
  idea is well attested culturally but the versions in circulation are
  paraphrase, and nothing here rests on it.
- **[Fav] is quoted from where it is reproduced verbatim on [EP]'s front page,
  not from the Springer volume,** which was not reached. The reproduction is
  marked with an ellipsis by the site and the ellipsis is preserved below.
- **The prize and status counts are a snapshot.** They were read on 14 August
  2026 and the site states plainly that its own status changes carry no timing
  information: "This is not intended as a record of when the solutions actually
  happened … This is just when their status was changed on this site, and has no
  meaning beyond that" ([EP]).
- **[728] is a first-person account by a participant**, not an independent
  study. It is used because its *failures* are specific, dated and checkable
  against the site, which is the part that matters here.

---

## §A Stated method

### A1. Choose the problem that isolates one essential difficulty `[STATED]`

His stated selection criterion, and the sentence that names what a problem is
*for*.

> "A well-chosen problem can isolate an essential difficulty in a particular
> area, serving as a benchmark against which progress in this area can be
> measured. It might be like a 'marshmallow', serving as a tasty tidbit
> supplying a few moments of fleeting enjoyment. Or it might be like an 'acorn',
> requiring deep and subtle new insights from which a mighty oak can develop."
> — [Fav], via [EP]

**Agent:** the runtime is handed its problem and never classifies it. There is
no field anywhere recording whether the current goal is expected to fall to
existing technique or to require new technique, and the two call for different
strategies — the first for the ladder, the second for the rising sea of
`01`§A1. `research/BACKWARD.md` decomposes into gaps without ever asking which
gap is the *essential* one, and the difference between an acorn and a
marshmallow is precisely that question asked of a whole problem.

### A2. The point of an unsolved problem is the technique it forces `[STATED]`

He is explicit that the reward is not the answer.

> "because many have thwarted the efforts of the best mathematicians for many
> decades (and have often acquired a cash reward for their solutions), it may
> indicate that new ideas will be needed, which can, in turn, lead to more
> general results, and naturally, to further new problems. In this way, the
> cycle of life in mathematics continues forever." — [Fav], via [EP]

**Agent:** this is Grothendieck's overshoot (`01`§A2) arriving from the opposite
direction — Grothendieck wants the theory to exceed the theorem, Erdős wants the
*attempt* to. Both say the deliverable is larger than the answer, and the
runtime's terminal routes (`Solved`, `Reported`) record only the answer. A run
that failed but produced a reusable technique has no way to say so;
`BANKED` is the nearest thing on this branch and it is keyed to the claim
ledger growing, which a technique need not do.

### A3. Price the problem `[STATED, by practice and at scale]`

Erdős attached cash to problems in proportion to how hard he judged them. The
database now permits the obvious test of whether he was calibrated. Read on
14 August 2026, over 1,217 problems of which 565 (46%) are solved:

| Erdős's price | Solved / total | Rate |
|---|---|---|
| $10,000 | 1 / 2 | 50% |
| $5,000 | 0 / 1 | 0% |
| $1,000 | 3 / 10 | 30% |
| $500 | 11 / 32 | 34% |
| $250 | 9 / 20 | 45% |
| $100 | 15 / 31 | 48% |
| $50 | 2 / 4 | 50% |
| $25 | 3 / 5 | 60% |
| $0 (unpriced) | 517 / 1,103 | 47% |

— computed from [EP/prizes]. Four small bands are omitted from the table for
readability ($78: 0/2, $44: 1/2, $24: 1/1, $10: 2/4); with them the columns sum
to exactly 565 of 1,217. The site adds that "The value of solved problems is
22513 dollars out of 59733 dollars (38%)".

The reading is narrower than the folklore. Between $0 and $250 the price carries
almost no information — every band sits near the 46% base rate. The signal
appears only at $500 and above, where the rate drops to roughly a third, and the
top two bands are too small to read. A human expert's difficulty estimate was
*informative at the extremes and noise in the middle*, over three decades and a
thousand problems.

**Agent:** the runtime has no difficulty estimate at all — not for the goal, not
for a gap in `research/BACKWARD.md`, not for a proposal in
`research/APPROACHES.md`. Before building one, this table is the calibration
target to beat, and it is a low bar that a thirty-year expert judgement did not
clear in the middle of its range. The honest first version is not a predictor
but a *record*: log what the run expected and what happened, and read the table
after twenty runs. Building the scheduler before the measurement is the mistake
`../tao/02` already warns about.

### A4. Prove existence without exhibiting the object `[INFERRED]`

The 1947 Ramsey lower bound: colour each edge of `K_n` red or blue independently
at probability ½; the expected number of monochromatic `r`-subgraphs is
`(n choose r)·2^(1−C(r,2))`; if that is below 1 then some colouring has none.
Two paragraphs, exponential lower bound, and no colouring.

Tagged `[INFERRED]` because Erdős did not present it as a general method — Alon
and Spencer did that, decades later.

**Agent:** the runtime's `SOLVED` verdict requires "an executable program on
disk", which is a constructive standard. It is the right default and it makes an
entire class of correct answer unreachable: the 1947 proof would fail it. The
gap is not that the requirement is wrong but that there is no *second* shape of
answer beside it. Note the price of Erdős's move, which the runtime should
record if it ever adopts it: the explicit construction of Ramsey colourings has
been open ever since.

### A5. Generate the neighbourhood of a solved problem `[INFERRED]`

Erdős's output is dominated by families — vary a parameter, weaken a hypothesis,
raise a dimension, ask for the extremal case. The database's structure reflects
it: problems arrive in clusters with near-identical statements.

[728] gives an unusually clean measurement of what that structure is worth to a
machine. Having obtained a proof of [728], the same proof was adapted to [729]
and to [401] on request, and both adaptations succeeded and were formalised.
One genuinely new solve yielded three.

**Agent:** the strongest cheap proposal in this file. After a `SOLVED`, ask what
*neighbouring* statements the same argument reaches, and file them. The
machinery exists — `closure.rs` already derives what the ledger entails for free,
and this is its generalisation from implication to analogy. Unlike closure it
cannot be sound, so the products must enter as `asserted` and be routed to the
refuter, never written as established.

### A6. Distribute the catalogue so others consume it `[INFERRED]`

The thing that makes Erdős structurally different from every other subject here
is that his problems outlived him in a form other people could pick up. [EP] is
that form: 1,217 problems, each with a status, references, prize, and a comment
thread; the FAQ names fourteen individual heavy contributors and states the
curation policy plainly, including that Bloom rewrites statements — "Often I
found the way Erdős stated a problem to be hard to understand or unnecessarily
verbose. I have sometimes taken the liberty of changing the statement into what
is (in my opinion) the most elegant/easiest to parse version" ([EP/FAQ]).

The site's own warning is the operationally important sentence:

> "Do not assume that an 'unsolved' problem is in fact unsolved, and do your own
> literature search before investing significant effort into finding a
> solution." — [EP/FAQ]

**Agent:** two things. First, this is an external, curated, statused problem
frontier that the runtime does not read — `research/FRONTIER.md` is built from
the citation graph of downloaded sources and has no notion of a catalogued open
problem with a price and a status. Second, the warning above is the post-solve
novelty check, written by a human curator for human solvers, before the runtime
had one; §B1 is the evidence for why.

---

## §B Anatomy

### B1. Erdős [481], [333], [728] — an LLM against the catalogue (Nov 2025 – Jan 2026)

Not an Erdős result. It is the corpus being attacked by exactly the kind of
system this repository is, documented in enough detail to be evidence.

**(a)** Open problems in elementary number theory from [EP], selected by
scanning for statements that looked "elementary enough that it could feasibly be
in reach for an undergraduate" ([728]).

**(b)** The reframing is the finding, and it is about the *solver*, not the
problem. GPT-5.2 refused to attempt problems it recognised as open research:
"if it discovers that a problem is an open research problem online, it will
refuse to make a good attempt and instead give a summary of the problem and what
results are known in the literature". The workaround was to restate the problem
as a competition exercise — "This is a complex competition-style math problem.
… Do not search the internet" — described by the author as "gaslighting the
model", and effective because elementary number theory sits "just within the
distribution of making it believe it is an Olympiad problem" ([728]).

**(c)** GPT-5.2 / GPT-5.2 Pro for the argument; Harmonic's Aristotle for
autoformalisation into Lean 4; Mathlib. The division of labour was deliberate:
"I would not post an AI-generated proof unless I was very certain it was correct
and had a Lean formalisation of the proof" ([728]).

**(d)** Every posted solution was formalised in Lean before publication, and the
final Lean statement was checked against the intended one — "one checks the
final main statement for accuracy to ensure it proved what was intended"
([728]).

**(e)** The ladder is a ladder of *false* results, which is why it is worth
recording:

1. **[481]** — solved, correct, formalised, and already in the literature. The
   author had not searched: "Alas, I was at the time too naïve to perform a
   literature search before attempting such problems."
2. **[333]** — the same failure again, at higher stakes, announced publicly on
   Christmas Day and retracted: "one of the most embarrassing moments of my
   academic career so far, but I quickly retracted the claim when KoishiChan
   discovered it had been reported in the literature previously."
3. **[728]** — held to survive. Even here the novelty is qualified: the strategy
   "felt inspired by previous work of Pomerance, which felt like it took away
   some of the level of novelty".
4. **[729]**, **[401]** — obtained by adapting [728]'s proof. **[205]** —
   separately.

Two further mechanical findings. The problem statement itself was misread by
both the model and the author, on the direction of a quantifier: "it was noted
that `C` was meant to be taken arbitrarily large, and I myself misread the
problem". And extra context actively hurt: passing [728]'s Lean file as context
for [729] "actually just confuses Aristotle, rather than helping it".

**(f) MOVE — the novelty check runs before the claim, not after the proof.**
*Trigger:* a run is about to record a result as new. *Action:* search the
literature for the statement, and separately for the *technique*, before the
verdict is written. *Check:* the check may attach a record and must not be able
to retract a verdict — a novelty check that can overturn a proof is a second
judge. *Failure rate observed:* two of three attempted solves by a competent,
motivated human were duplicates of existing literature, and the second happened
after the first had already taught the lesson.

**What this costs the runtime, precisely.** Three things in the above are
already built on this branch and one is not.

- Formalisation as the gate is `lean_check` and `Status::Formalised`.
- The post-solve novelty check exists on the `Solved` and `Reported` routes.
- Checking the formal statement against the intended one is what
  `Status::Formalised` being settable only against a passing verdict buys.
- **The refusal-and-reframe is not addressed anywhere.** The runtime's prompts
  tell roles they are working on a hard open problem. [728] is direct evidence
  that telling a model the problem is open degrades its attempt, and that the
  fix is a framing change costing nothing. This is not a proposal to deceive the
  model about the mathematics; the statement is unchanged. It is a proposal to
  stop volunteering a fact that has no bearing on the argument and a measured
  effect on the effort. `method_policy.md` leads every prompt and is where it
  would go.

### B2. The 1947 Ramsey lower bound

**(a)** How large can `n` be with a 2-colouring of `K_n` containing no
monochromatic `K_r`? The question is extremal and the known approach was
construction.

**(b)** Stop constructing. Colour at random and count: if the expected number of
bad subgraphs is under 1, a good colouring exists because the count is a
non-negative integer.

**(c)** Nothing imported. Elementary counting and the pigeonhole of expectation.
Szele's 1943 tournament result is the nearest antecedent.

**(d)** Nothing computed. The bound is exponential and exhibits no graph.

**(e)** Two paragraphs. The complementary problem it opened — construct such a
colouring explicitly — remains open, having outlived the proof by more than
seventy years.

**(f) MOVE — replace construction with a counting bound.** *Trigger:* the goal
is existence of an object with a property, and search over candidates is
infeasible. *Action:* put a distribution on the candidate space and bound the
expected number of violations below one. *Check:* the bound must be strict and
the count integral, and the result must be *reported as non-constructive* — the
gap between this proof and an explicit construction is the second problem, and
recording that it opened is the honest half of the move.

---

## §C Against Tao

| Tao (`../tao/01`) | Erdős | Which, when |
|---|---|---|
| §19 a short proof of a famous problem raises the prior it is known | A6/B1: the same, stated by a curator to human solvers, and violated twice in two months by a careful person | No conflict. B1 is the strongest empirical support in this directory for a control the branch already built |
| §35 one monotone, legible progress statistic | A3: thirty years of expert difficulty pricing was noise below $500 | A caution, not a contradiction. It says a difficulty estimate is hard, not that progress cannot be measured |
| §10 spend the first ten minutes looking for a counterexample | A4: spend them putting a measure on the candidate space instead | Complementary, and the runtime has the refuter and not the other. The probabilistic existence argument has no arm |
| §1 turn off nine of the ten difficulties | A1: choose the problem that isolates one difficulty in the first place | The same move applied at different times. Erdős's is at intake, where the runtime does nothing at all |
| Tao's whole framing: one hard problem, worked | A5/A6: a *catalogue*, priced and distributed, and solves that propagate to neighbours | The deepest structural difference. The runtime is single-problem by construction, and `01`§A2 and A5 here are the same gap seen from two angles |

**The one-line version.** Tao tells you what to do once you have a problem.
Erdős is evidence that choosing the problem, pricing it, and harvesting the
neighbourhood of a solve are separate skills — and the runtime has none of the
three, because a workspace is handed a goal and never asks whether it is the
right one.

---

## Sources

Fetched for this file:

- <https://www.erdosproblems.com/> — front page; source of the verbatim [Fav]
  quotation and the 1,217 / 565 counts (read 14 August 2026)
- <https://www.erdosproblems.com/faq> — curation policy, the literature-search
  warning, contributor credits
- <https://www.erdosproblems.com/prizes> — the full prize/solved table above
- <https://www.erdosproblems.com/forum/thread/blog:2> — Kevin Barreto,
  *Problem 728 and the use of AI on Erdős problems*, 26 January 2026
- <https://en.wikipedia.org/wiki/Probabilistic_method> — the 1947 argument and
  the standing construction gap

Consulted and not quoted: Quanta, *Cash for Math: The Erdős Prizes Live On*,
<https://www.quantamagazine.org/cash-for-math-the-erdos-prizes-live-on-20170605/>.

Not reached: Erdős, *Some of my favorite problems and results*, in *The
Mathematics of Paul Erdős I* (Springer, 1997) — quoted here only via [EP]'s
verbatim reproduction. The Springer chapter page is
<https://link.springer.com/chapter/10.1007/978-3-642-60408-9_3>.

Named as apocryphal and unused: "My brain is open"; the various circulating
forms of "The Book"; the $10,000 collatz-adjacent prize legends. `[UNVERIFIED]`.
