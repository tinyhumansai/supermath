# Gowers: the mathematician who wrote the harness

**The axis.** Every other subject here has to be *read* for a method. Gowers
wrote his down, then implemented it. Ganesalingam–Gowers built a fully automatic
problem solver whose design constraint was that it may not do anything a good
human mathematician would not do — a move-type ladder ordered by *safety*, with
backtracking banned outright. He also designed Polymath, which is a
specification for a collective solver, and named the theory-builder /
problem-solver split that the rest of this directory is organised around.

He is the most directly usable subject in the set. His paper is not evidence
about mathematics that has to be translated into a harness requirement; it *is*
a harness, with a priority order, a failure mode, and a published account of
where it performs badly.

**Source keys.** **[GG]** = M. Ganesalingam and W. T. Gowers, *A fully automatic
problem solver with human-style output*, arXiv:1309.4501,
<https://arxiv.org/abs/1309.4501> (PDF fetched and read; page references are the
PDF's); **[Poly]** = Gowers, *Is massively collaborative mathematics possible?*,
27 January 2009,
<https://gowers.wordpress.com/2009/01/27/is-massively-collaborative-mathematics-possible/>;
**[Bundy]** = Alan Bundy, *The Science of Reasoning*, quoted at length in [GG]
§1.4 and cited here as [GG] reports it.

## Accuracy conventions

- **[GG] is quoted from the arXiv PDF**, converted to text and read directly.
  The arXiv abstract page gives the title as *A fully automatic theorem prover
  with human-style output*; the PDF's running head and body use *problem
  solver*. Both are used in the literature; this file uses the PDF's.
- **[Poly] quotations came through a summarising fetch of the blog post.** They
  are reproduced as that fetch returned them and are marked with the source key;
  they were not re-verified against the page's HTML, so treat any [Poly]
  wording as *reported verbatim* rather than independently confirmed.
- **The Bundy passage is Bundy's, quoted by Gowers.** It is used here because
  Gowers endorses it explicitly — "we believe that many of his comments apply
  more generally" ([GG] §1.4) — but the words are not Gowers's.
- **Nothing here rests on Gowers's blog commentary about AI more generally.**
  Secondary summaries of his recent views on machine mathematics were found and
  deliberately not used; [GG] is a primary source that says more.

---

## §A Stated method

### A1. Rank the available moves by *safety*, and always take the safest applicable one `[STATED]`

The organising principle of the whole program, and the thing this runtime most
obviously lacks.

> "A broad overarching principle that gives a theoretical backing to many of our
> choices is this: the program prefers safe moves to dangerous moves. The
> picture we have here is one where at any stage there is a choice of moves that
> can be made, and we have to make an assessment of how likely any given choice
> is to form part of the argument one is looking for. The greater this
> likelihood, the safer the move." — [GG] §2.5

The mechanism is a fixed, totally ordered list of move types, applied greedily:

> "Move types are ranked in order of attractiveness or priority, and the basic
> operation of the program consists of repeatedly choosing the most attractive
> move type that can be applied, generating the moves of that type, and applying
> the most attractive one." — [GG] §2.2

They note the ancestry — this is the Boyer–Moore waterfall — and that the order
was fixed by introspection: "Like Boyer and Moore, we chose the move types and
their priority by examining our own reactions to many different problems"
([GG] §2.2). Notably it did not need tuning: "they suggested an order before we
started, and we found that we did not have to modify the order when we tried
further problems" ([GG] §2.5).

**Agent:** the runtime has a routing ladder and it routes on *state* — attempt
count, unproductive count, verdict kind — never on *what move is available*. An
attempt is a model call that decides for itself what to try. The Gowers ladder
is the missing half: given the current problem state, the *set of applicable
moves* is computable, and their order is a policy the runtime could own rather
than a judgement the model makes silently every turn. This is also `../tao/04`
R5's cheap-first ladder arriving from a completely different tradition, and it
is a stronger argument for it, because Gowers's ordering is by *probability of
being part of the final argument* rather than by cost.

### A2. Define the tractable class by what needs no backtracking, then forbid backtracking `[STATED]`

The scoping move, and it is a piece of engineering discipline worth more than
most heuristics.

> "we define a routine problem to be one that a good human mathematician will
> typically solve easily without backtracking. If a program is to satisfy the
> main constraint, then it too will have to solve routine problems without
> backtracking, so we can simply ban it. Although this is a significant scaling
> down of ambition, it also makes the project far more realistic in the short
> term." — [GG] §2.1

**Agent:** the runtime's equivalent of backtracking is `Route::Retry` and the
restart path, and it has no notion of a class of subgoal that should be
dischargeable without either. Naming one would be informative in both
directions: a gap in `research/BACKWARD.md` that took three attempts either was
not routine, or the ladder above it is wrong. Today a gap that takes three
attempts and one that takes one are indistinguishable in the ledger.

### A3. Do not let the machine use its speed `[STATED]`

The constraint they impose on themselves, and the reason for it.

> "we would describe our work as belonging not just to the human-oriented
> tradition, but as belonging to the extreme human end of the machine-human
> spectrum. In practice, this means deliberately not allowing ourselves to
> exploit the speed of computers, for example by letting them carry out large
> searches or perform very complicated calculations. We hope that by submitting
> to this restriction, we will force ourselves to develop a number of useful and
> important techniques while the problems we are tackling are still relatively
> simple." — [GG] §1.4

**Agent:** the runtime's instinct is the opposite, and `../tao/02` already warns
about it — computation was decisive in one of eleven Tao programmes and
decorative in most of the rest. `COMPUTATIONAL_THRESHOLD` exists to catch a run
that keeps scaling one method, which is the symptom; Gowers is prescribing
against the cause. The strong version — withhold `execute_command` for the first
`n` attempts, the way `MATH_AGENT_RESEARCH` withholds search — is a real
proposal and would be enforced the same way, by not registering the tool.

### A4. Combinatorial explosion is the main problem, and humans avoid it by some mechanism worth copying `[STATED]`

Their first argument for the human-oriented approach, and it is an argument
about architecture, not about taste.

> "It is in the nature of solving a complex mathematics problem that one throws
> up other problems that need solving, which in turn throw up further problems,
> and so on. If a generous amount of search is permitted, then this recursive
> nature of problem solving naturally leads to the search being iterated, and
> thus to a combinatorial explosion. Somehow, in ways that we do not fully
> understand, humans manage to avoid this difficulty by keeping search strictly
> under control." — [GG] §1.4

**Agent:** the runtime's recursion is `spawn_agent`, and its control is a budget
— `RunBudget` caps calls and wall-clock. That is a *cutoff*, not the mechanism
Gowers is pointing at, and a cutoff arriving mid-recursion is how a run loses
everything in flight. The Gowers answer is that the branching factor should be
small because most moves are never generated, not because a counter stopped
them.

### A5. Two kinds of proof, and they want different machinery `[STATED]`

The single most useful distinction in the paper for this runtime, because the
runtime pretends there is one kind.

> "machine-oriented methods are better for finding certain kinds of proofs, and
> human-oriented methods are better for finding other kinds. … Some proofs seem
> to consist of a succession of somewhat arbitrary and unpredictable steps,
> while others can be discovered by means of what mathematicians would describe
> as 'key ideas'." — [GG] §1.5

On the first kind, using Robbins as the example: "There are no obvious measures
of progress that tell us that some initial segments of sequences are 'obviously
right' or at least 'getting warmer', and in the absence of such clues there is
not much for it but to undertake a huge search" ([GG] §1.5).

On the second: "for the majority of proofs that mathematicians find, there is
some kind of 'story' to tell of the ideas that give rise to the proof. Typically,
such a story will be a high-level overview of the main difficulty and how it is
overcome, where 'overcome' means that the problem is reduced to one or more
problems where that difficulty no longer occurs" ([GG] §1.5).

**Agent:** the runtime has both kinds of machinery — Vampire, E, Z3, cvc5 and
the SAT stack on one side; the reasoning roles on the other — and *no
classifier*. Nothing asks which kind of proof the current gap wants, so the
choice is made implicitly by whichever role happens to be delegated to. The test
Gowers gives is usable: **does a measure of progress exist on partial
arguments?** If not, it is a search problem and should go to the solvers; if so,
it is a story problem and should go to the reasoners. `reflection`'s
`KIND: MATHEMATICAL | COMPUTATIONAL` is the nearest existing field and it
classifies the attempt *after the fact*, not the subgoal in advance.

### A6. The intermediate statement is found by approximation, not search `[STATED]`

His account of how the key step is actually found, and it is a loop.

> "The intermediate statement itself is typically found not by means of a
> brute-force search but by a process of approximation: one might make a guess,
> find that it is unhelpful, understand why it is unhelpful, and use that
> understanding to guide the search for a better intermediate statement." —
> [GG] §1.5

**Agent:** this is the `reducer` arm's job description, written better than
`reducer.md` writes it, and with the load-bearing clause the prompt omits:
*understand why it is unhelpful*. `research/BACKWARD.md` records `killed-by` per
gap, which is the field this would populate, and nothing requires a killed gap
to state what its failure taught. A killed gap with an empty diagnosis is the
loop's most common wasted step.

### A7. A simple general statement that is not obviously true is almost certainly false `[STATED]`

One of the two working assumptions they extract from watching a human, and it is
directly implementable.

> "to simplify statements before deciding whether they are likely to be true,
> and to make the working assumption that a sufficiently simple general
> statement that is not obviously true is almost certainly false." — [GG] §1.4

Paired with the Reiter observation that motivates it: "before we invest time in
proving a statement, we like to feel that that statement has at least some
plausibility" ([GG] §1.4).

**Agent:** the refuter arm on this branch implements the *response* to this and
not the *trigger*. It runs on a cadence against open gaps. Gowers's rule is a
gate: a newly proposed lemma that is simple, general and not obviously true
should go to the refuter *first*, before any effort is spent proving it. That is
a routing change, not a new capability, and `research/BACKWARD.md` already
carries every new gap through one place.

### A8. If a hypothesis is essential and can be used in only one way, use it `[STATED]`

The forward-reasoning counterpart, and the cheapest rule in the paper.

> "if a statement appears to be essential and can be applied in only one way,
> then there is no harm in applying it, even if you cannot see what good it will
> do." — [GG] §1.4

**Agent:** `closure.rs` derives what the ledger entails through `follows-from`
edges. This rule says the same walk should run over *hypotheses with a unique
application* even when no one asked for the consequence. It is forward closure
where the existing one is a deduction, and it is the difference between a ledger
that answers questions and one that volunteers facts.

### A9. Prefer forward reasoning, and be reluctant to switch direction `[STATED]`

An ordering claim with a stated evidential base and a stated exception.

> "The psychology literature suggests that when it is safe, humans tend to
> prefer forwards reasoning to backwards reasoning, though this appears to be a
> question more of style than of problem-solving efficacy … Since forwards
> reasoning tends to be safe for the highly routine problems our program
> tackles, we have given all forwards reasoning a higher priority than all
> backwards reasoning. This also has the beneficial effect of making the program
> reluctant to switch direction — too much switching from forwards to backwards
> or vice versa would again be bad mathematical style." — [GG] §2.5

They then publish the case where this hurts them: proving that `H ∩ K` is a
subgroup, the program makes "silly" forward deductions such as `x⁻¹ ∈ H`, and
they name the fix as either restricting term creation or switching to backwards
reasoning when several forward moves look equally relevant ([GG] §2.5).

**Agent:** the runtime *is* backwards-first — `reducer` and
`research/BACKWARD.md` are its spine, and there is no forward arm at all. Gowers
ran the opposite experiment and found the failure mode: unconstrained forward
reasoning generates irrelevant true statements. The synthesis is his own
proposed fix — direction should switch when the number of applicable moves rises,
because that is the signal that relevance has been lost.

### A10. Expanding a definition is expensive because it is irreversible `[STATED]`

A specific and non-obvious ranking, worth quoting because it is the kind of rule
no prompt would produce.

> "expanding a definition is substantially less safe: sometimes it is possible
> to reason in a high-level way without expanding, and since we do not allow
> 'de-expansion' in this program … expanding a definition is closing off the
> option of such high-level arguments." — [GG] §2.5

**Agent:** the runtime has no notion of an irreversible step. Every move a role
makes writes into the workspace and the workspace is append-and-derive, so
nothing distinguishes a step that forecloses options from one that does not.
This is the most transferable single line in the paper: *rank moves by what they
close off*, not only by what they open.

### A11. Collaboration: post the half-formed idea `[STATED]`

The Polymath design, and its rules are a specification for a multi-role system.

> "try to resist the temptation to go away and think about something and come
> back with carefully polished thoughts: just give quick reactions" — [Poly]

> "The ideal outcome would be a solution of the problem with no single
> individual having to think all that hard." — [Poly]

His three reasons are probability of luck, the union of what different people
know, and specialisation — "A hugely collaborative project would make it
possible for people to specialize", with some generating ideas, others
criticising, testing details, reformulating or synthesising ([Poly]).

**Agent:** the runtime is Polymath by construction — nineteen tool-boundaried
roles, which `../tao/02` F2 already identifies as its real strength. What it
does not have is the *half-baked contribution*. Every ledger the runtime derives
demands a well-formed block: a claim needs a status and a bearing, an approach
needs a mechanism and a first step, a gap needs a next move. There is no place
to put a quick reaction, and `note_scratch` is deliberately unreachable from
durable recall, so a provisional thought cannot be seen by another role at all.

---

## §B Anatomy

### B1. The Ganesalingam–Gowers prover (published 2013)

**(a)** Prove elementary statements — mostly metric space theory — *and* emit a
write-up indistinguishable from a human's: "presents solutions that are hard to
distinguish from solutions that might be written by human mathematicians"
([GG], abstract).

**(b)** The reframing is the design constraint: "we do not allow our programs to
do anything that a good human mathematician wouldn't do" ([GG] §2.1). Human-style
output then costs nothing extra, because it is a transcript of what the program
actually did — [GG] §1.1 notes the output "would not be able to produce
human-style [output]" were the internals otherwise.

**(c)** The Boyer–Moore waterfall for the architecture; Bundy's proof plans and
science of reasoning for the philosophy; Reiter's model-pruning for the
plausibility check; an LCF-style prover state, deliberately diverged from where
a human would represent things differently — a conjunctive target does *not*
split the hypothesis list, because "A human would think in terms of one ambient
primary collection of 'facts that are known'" ([GG] §2.2).

**(d)** No large search, no heavy calculation, by rule (A3). Statements carry
annotations recording what has already been used — "logically unnecessary, it is
indispensable in human reasoning" ([GG] §2.2).

**(e)** The tradition is long and the paper reads its history as a warning.
Bundy's caricature of conventional ATP is quoted approvingly: heuristics are
added to prune losing branches until "eventual deadlock as different proofs pull
the heuristics in different directions", against which proof-plan systems make
"slower initial progress" but have "no eventual deadlock to block the indefinite
improvement" ([Bundy], via [GG] §1.4). Kerber's observation is the other side:
machine-oriented systems settled Robbins, yet "observing the blind search
behaviour of such a system as it fails to solve a problem that seems trivial to
us as humans can be disappointing" ([GG] §1.5).

**(f) MOVE — make the priority order explicit and greedy, and publish where it
fails.** *Trigger:* a system chooses among available actions by model
judgement every turn. *Action:* enumerate the action types, order them once by
how likely each is to appear in the final argument, and apply the highest
applicable one without search. *Check:* the order must be fixed *before* the
test problems and must survive them unmodified — [GG] met this and says so — and
the cases where it misbehaves must be published, as the `H ∩ K` example is.

### B2. Polymath1 — the design, not the result (2009)

**(a)** Can a large open collaboration solve a research problem? Gowers chose a
"middle ground" problem, ruling out both extremes: "it seems highly unlikely
that one could persuade lots of people to share good ideas about the Riemann
hypothesis", and equally that the problem should not be "very minor and
specialized" ([Poly]).

**(b)** The reframing is that the *unit of contribution* shrinks. Rather than
a proof, the unit is a reaction — undeveloped, possibly wrong, posted quickly.

**(c)** Nothing imported mathematically. The imports are social: blog comment
threads, a wiki, an explicit rule set.

**(d)** Not applicable.

**(e)** `../tao/02` F4 records the cautionary companion result — Polymath8
drove the prime gap from 70,000,000 to 4,680 over thirteen months while Maynard
reached 600 independently, discarding the machinery the collaboration had built.
The two together are the honest picture: the design works, and it does not
follow that the collective branch is the one to fund.

**(f) MOVE — lower the bar for a contribution below the bar for a result.**
*Trigger:* a system's only channel for an idea requires the idea to be
well-formed. *Action:* add a channel whose entries are explicitly provisional,
readable by other roles, and never promotable without passing the normal gate.
*Check:* the provisional channel must not be reachable by anything that derives
an established ledger — which is precisely why `note_scratch` was separated from
durable recall, and why the fix here is to make scratch *shared between roles*,
not to relax what a claim requires.

---

## §C Against Tao

| Tao (`../tao/01`) | Gowers | Which, when |
|---|---|---|
| §3 try anything, the stupider the better | A1: never make an unsafe move when a safe one applies | Opposed, and reconcilable by phase. Tao's rule is for a cold start with no reading; Gowers's ladder assumes the state is legible. A runtime should use Tao's only when the applicable-move set is empty |
| §20 numerics before theory | A3: deliberately refuse the machine's speed | The sharpest split in the directory alongside `01`§C. Note that Gowers is not anti-computer — he built one — he is against letting search substitute for the ladder |
| §10 look for a counterexample first | A7: only when the statement is simple, general and not obviously true | Gowers's is Tao's with a firing condition attached, and the firing condition is what the refuter arm currently lacks |
| §16 record which techniques are known not to apply | A6: and record *why* the intermediate statement was unhelpful | Same field, and Gowers names the harder half. `killed-by` exists; the diagnosis does not |
| §34 modularise so no participant needs the whole argument | A11: and let participants post things that are not arguments | Agreement on structure, disagreement on the entry bar. The runtime enforces Tao's and not Gowers's |

**The one-line version.** Tao says what to try. Gowers says what to try *first*,
and gives the ordering principle — how likely is this move to appear in the
final argument — that turns a list of heuristics into a policy. The runtime has
the heuristics in prompts and the policy nowhere.

---

## Sources

Fetched for this file:

- M. Ganesalingam and W. T. Gowers, *A fully automatic problem solver with
  human-style output*, arXiv:1309.4501 — <https://arxiv.org/abs/1309.4501>,
  PDF read in full for every [GG] quotation
- Gowers, *Is massively collaborative mathematics possible?* —
  <https://gowers.wordpress.com/2009/01/27/is-massively-collaborative-mathematics-possible/>
  (summarising fetch; see accuracy conventions)

Quoted through [GG] and not fetched directly: Alan Bundy, *The Science of
Reasoning*; M. Kerber; R. Reiter's 1970s model-pruning system; Boyer and Moore's
waterfall. Cited as [GG] reports them.

Consulted and not used: secondary summaries of Gowers on AI in mathematics
(<https://aichats.substack.com/p/can-ai-do-mathematics-part-iii-timothy>) and
of *The Two Cultures of Mathematics*. The two-cultures distinction is used in
[`12-cross-cutting.md`](12-cross-cutting.md) as a framing device only, and no
quotation is attributed to that essay anywhere in this directory —
`[UNVERIFIED]` as a source.
