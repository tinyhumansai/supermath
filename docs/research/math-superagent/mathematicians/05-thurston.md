# Thurston: the output is understanding, not the proof

**The axis.** Every other subject here is studied for how they got to a proof.
Thurston wrote an essay arguing that the proof is not the point and that
optimising for it degrades the field — and he has the receipt: he proved his way
into a subfield so hard that everyone left it. He is the strongest argument in
this directory against the runtime's own success criterion, and he is careful
enough to be worth taking seriously, because he explicitly refuses the sloppy
version of his own position.

**Source key.** **[TH]** = William P. Thurston, *On proof and progress in
mathematics*, Bull. AMS (N.S.) 30 (1994) 161–177, arXiv:math/9404236,
<https://arxiv.org/abs/math/9404236>. The PDF was fetched and read in full;
every quotation below is from it, and section numbers are the essay's own.

## Accuracy conventions

- **This is a single-source file, and the source is primary and complete.** No
  paraphrase chain, no translation, no second-hand attribution. Where §B rests on
  events, it rests on Thurston's own account of them, which is a first-person
  claim and is labelled as one.
- **The essay is a reply.** It answers Jaffe and Quinn's *Theoretical
  mathematics* (math.HO/9307227), which was not fetched. Thurston's
  characterisation of their position is his; it is quoted as his.
- **Do not read this file as an argument against rigour.** Thurston forecloses
  that reading explicitly and the foreclosure is quoted in A4. Any proposal
  citing this file that ends in "so the runtime should relax its proof
  standard" has misused it.

---

## §A Stated method

### A1. The product is human understanding, not answers `[STATED]`

The essay's thesis, and its most quotable illustration is about computers.

> "when Appel and Haken completed a proof of the 4-color map theorem using a
> massive automatic computation, it evoked much controversy. I interpret the
> controversy as having little to do with doubt people had as to the veracity of
> the theorem or the correctness of the proof. Rather, it reflected a continuing
> desire for human understanding of a proof, in addition to knowledge that the
> theorem is true." — [TH] §1

And the everyday version:

> "They might print out a table of the first 10,000 primes, only to find that
> their printout isn't something they really wanted after all. They discover by
> this kind of experience that what they really want is usually not some
> collection of 'answers'—what they want is understanding." — [TH] §1

**Agent:** this repository's own `CLAUDE.md` states the same standard in its
second paragraph — the product should help a reader understand *why* an answer
is true, not merely produce a plausible final expression. Thurston is the
citation for that sentence, and the gap is that the code does not enforce it.
`reflection`'s `SOLVED` requires a specific final answer, a second independent
route, an executable program on disk, and internal consistency. Every one of
those is a check on the answer. None asks whether the derivation explains
anything, and `research/ROOT.md` — "what the library means" — is the only
artefact in the whole workspace whose job is understanding, is agent-written,
and is read by no verdict.

### A2. The definition-theorem-proof model is a caricature, and it cannot say where questions come from `[STATED]`

He names the model the runtime implements.

> "D. mathematicians start from a few basic mathematical structures and a
> collection of axioms 'given' about these structures, that T. there are various
> important questions to be answered about these structures … and P. the task of
> the mathematician is to seek a deductive pathway from the axioms to the
> propositions or to their denials. We might call this the
> definition-theorem-proof (DTP) model of mathematics. A clear difficulty with
> the DTP model is that it doesn't explain the source of the questions." —
> [TH] §1

He credits Jaffe and Quinn with adding speculation — "making conjectures,
raising questions, and making intelligent guesses and heuristic arguments about
what is probably true" — and says the augmented model still fails:

> "We are not trying to meet some abstract production quota of definitions,
> theorems and proofs. The measure of our success is whether what we do enables
> people to understand and think more clearly and effectively about
> mathematics." — [TH] §1

**Agent:** `research/BACKWARD.md` is DTP made mechanical: a goal, a skeleton,
and gaps to be discharged. `research/APPROACHES.md` is the speculation layer, so
the runtime is at Jaffe–Quinn's DSTP. Thurston's objection lands exactly where
the ledgers stop: nothing records whether the run *understands* its problem
better than it did at the start, and the run's only source of new questions is
`request_research`, which asks for facts rather than for framings.

### A3. One object, many irreducibly different understandings `[STATED]`

His famous list of seven ways to understand a derivative — infinitesimal,
symbolic, logical, geometric, rate, approximation, microscopic — with the
warning attached:

> "This is a list of different ways of thinking about or conceiving of the
> derivative, rather than a list of different logical definitions. Unless great
> efforts are made to maintain the tone and flavor of the original human
> insights, the differences start to evaporate as soon as the mental concepts
> are translated into precise, formal and explicit definitions." — [TH] §2

He then adds entry 37 as a joke about how far the list runs, and notes "one
person's clear mental image is another person's intimidation" ([TH] §2).

**Agent:** the sentence about evaporation is a direct statement about what
formalisation costs, and it is the most precise version of that claim in this
directory. `lean_check` produces a kernel-verified artefact in which every one
of the seven readings has become the same object. That is what makes it a
control and it is also what makes it lossy. The runtime should hold both, and
today it holds only one: `Status::Formalised` exists and there is no field
anywhere recording *which reading of a claim the run is working with*.

Note this is also Grothendieck's A7 (`01`§A7) — multiply the viewpoints — with a
different emphasis. Grothendieck wants several viewpoints because their
conjunction sees further. Thurston wants them because a person needs the one
that fits their head.

### A4. Validity is social before it is formal — and this is a description, not a proposal `[STATED]`

The passage most often misquoted, so it is given at length with its own
disclaimer.

> "Within any field, there are certain theorems and certain techniques that are
> generally known and generally accepted. When you write a paper, you refer to
> these without proof. … Many of the things that are generally known are things
> for which there may be no known written source. As long as people in the field
> are comfortable that the idea works, it doesn't need to have a formal written
> source." — [TH] §4

> "Mathematical knowledge and understanding were embedded in the minds and in
> the social fabric of the community of people thinking about a particular
> topic. This knowledge was supported by written documents, but the written
> documents were not really primary." — [TH] §4

> "People are usually not very good in checking formal correctness of proofs,
> but they are quite good at detecting potential weaknesses or flaws in proofs."
> — [TH] §4

And the foreclosure, in his words:

> "I am not advocating any weakening of our community standard of proof; I am
> trying to describe how the process really works. … The kind of change I would
> advocate is that mathematicians take more care with their proofs, making them
> really clear and as simple as possible so that if any weakness is present it
> will be easy to detect." — [TH] §4

**Agent:** the last sentence is a design requirement and it is *not* the one
`lean_check` implements. Lean makes a proof checkable by a kernel; Thurston asks
that a proof be written so a *reader* can find the weakness. These come apart —
a Lean file can pass and be unreadable, which is precisely the state the
Erdős-728 episode describes (`02`§B1: "I would not ask a working mathematician
to go through pages of potential AI slop"). The runtime has one control and not
the other, and the missing one is cheap: a derivation could be scored on whether
its weakest step is *identified by the author*.

His own example of the mechanism is worth keeping: "Andrew Wiles's proof of
Fermat's Last Theorem is a good illustration … The experts quickly came to
believe that his proof was basically correct on the basis of high-level ideas,
long before details could be checked" ([TH] §4). Written in 1994 — that belief
was, at the time of writing, wrong. See `04`§B1.

### A5. Software has a higher correctness standard than mathematics, and still has bugs `[STATED]`

An aside, and a useful corrective for anyone building this runtime.

> "The standard of correctness and completeness necessary to get a computer
> program to work at all is a couple of orders of magnitude higher than the
> mathematical community's standard of valid proofs. Nonetheless, large computer
> programs, even when they have been very carefully written and very carefully
> tested, always seem to have bugs." — [TH] §4

**Agent:** the runtime's own repeated lesson stated from outside — a control that
passes is not a guarantee. It is the argument for `lean_check` running at
ledger-derivation time rather than write time, so that a claim whose Lean file
was later edited into a `sorry` loses its standing on the next derivation.

### A6. A proof's role depends on where the field is, and most proofs are temporary `[STATED]`

> "Not all proofs have an identical role in the logical scaffolding we are
> building for mathematics. This particular proof probably has only temporary
> logical value, although it has a high motivational value in helping support a
> certain vision for the structure of 3-manifolds." — [TH] §6

**Agent:** `research/CLAIMS.md` has no notion of a claim's *role*. `bearing` is
the nearest field and it records what the claim bears on, not whether the claim
is scaffolding that a later, better argument will discard. A run that proves a
special case in order to see the shape of the general one has done something the
ledger will over-credit.

### A7. Proofs are addressed to an audience, and the expansion factor is enormous `[STATED]`

The Bowdoin 1980 workshop, and the clearest statement of a cost the runtime pays
constantly.

> "It became dramatically clear how much proofs depend on the audience. We prove
> things in a social context and address them to a certain audience. Parts of
> this proof I could communicate in two minutes to the topologists, but the
> analysts would need an hour lecture before they would begin to understand it.
> … And there were many other parts of the proof which should take two minutes
> in the abstract, but that none of the audience at the time had the mental
> infrastructure to get in less than an hour." — [TH] §6

> "there is sometimes a huge expansion factor in translating from the encoding
> in my own thinking to something that can be conveyed to someone else." —
> [TH] §6

**Agent:** the runtime has nineteen roles with different tool grants and
different context files, which is nineteen audiences. `CONTEXT.md` is the shared
brief and is budgeted at 10k tokens by `shared_context.rs`, so the expansion
factor is not a metaphor here — it is a hard constraint that is enforced by
truncation. Thurston's finding is that the *right* expansion differs per
audience, and `role_context()` already selects different workspace files per
role, so the machinery exists to differentiate and does not.

---

## §B Anatomy

### B1. Foliations — the negative result (early 1970s)

The only entry in this directory where the *failure* is that the mathematics
went too well.

**(a)** Foliations, then "a big center of attention among geometric topologists,
dynamical systems people, and differential geometers" ([TH] §6).

**(b)** No reframing. He proved a classification theorem giving a necessary and
sufficient condition for a manifold to admit a foliation, plus "a number of
other significant theorems", fast — "It was hard to find the time to write to
keep up with what I could prove, and I built up a backlog" ([TH] §6).

**(c)** "I did not hesitate to draw on any of the mathematics I had learned from
others" ([TH] §6) — which is the mechanism of the failure below.

**(d)** Nothing computational.

**(e)** The outcome:

> "Within a couple of years, a dramatic evacuation of the field started to take
> place. I heard from a number of mathematicians that they were giving or
> receiving advice not to go into foliations—they were saying that Thurston was
> cleaning it out. People told me (not as a complaint, but as a compliment) that
> I was killing the field." — [TH] §6

His diagnosis is two *ecological* causes, not intellectual exhaustion — "I do
not think that the evacuation occurred because the territory was intellectually
exhausted". First, the write-ups: "documented in a conventional, formidable
mathematician's style. They depended heavily on readers who shared certain
background and certain insights. … I also threw out prize cryptic tidbits of
insight, such as 'the Godbillon-Vey invariant measures the helical wobble of a
foliation', that remained mysterious to most mathematicans who read them. This
created a high entry barrier" ([TH] §6). Second, credit: "When I started working
on foliations, I had the conception that what people wanted was to know the
answers. … But that's only one part of the story. More than the knowledge,
people want personal understanding" ([TH] §6).

**(f) MOVE — measure the artefact by whether the next worker can use it.**
*Trigger:* a run closes a goal and writes its derivation. *Action:* check that
the derivation's dependencies are stated rather than assumed, and that its
"cryptic tidbits" — the compressed insights — are expanded or flagged as
unexpanded. *Check:* the test is whether a *different role*, with a different
context set, can act on the file. The runtime can actually run this check, which
almost nothing else in this directory can be said of: hand the derivation to a
role that did not write it and see whether it can state the next step.

### B2. Geometrization — the deliberate correction (late 1970s–1980s)

**(a)** Conjecture, then prove, that all 3-manifolds carry a geometric
structure. He notes it "went against the trends in topology for the preceding 30
years, and it took people by surprise" ([TH] §6).

**(b)** The reframing is what he did *after* proving the Haken case, not before.
Having watched foliations empty out, he inverted his priorities:

> "In reaction to my experience with foliations and in response to social
> pressures, I concentrated most of my attention on developing and presenting
> the infrastructure in what I wrote and in what I talked to people about. …
> I wrote some papers giving the substantive parts of the proof of the
> geometrization theorem for Haken manifolds—for these papers, I got almost no
> feedback." — [TH] §6

**(c)** "several mathematical theories that fed into the cluster of ideas:
three-manifold topology, Kleinian groups, dynamical systems, geometric topology,
discrete subgroups of Lie groups, foliations, Teichmüller spaces, pseudo-Anosov
diffeomorphisms, geometric group theory, as well as hyperbolic geometry" ([TH]
§6). Nine source fields for one theorem.

**(d)** He mentions computer evidence supporting the general conjecture, and
elsewhere that he "spent a fair amount of effort during periods of my career
exploring mathematical questions by computer" ([TH] §4). Not load-bearing for
the Haken proof.

**(e)** The distribution mechanism is the finding: notes written alongside a
graduate course, mailed to a list that grew to about 1,200 people, revised
against feedback that ran "Your notes are really inspiring and beautiful, but I
have to tell you that we spent 3 weeks in our seminar working out the details of
§n.n. More explanation would sure help" ([TH] §6).

The result, in his accounting:

> "By concentrating on building the infrastructure and explaining and publishing
> definitions and ways of thinking but being slow in stating or in publishing
> proofs of all the 'theorems' I knew how to prove, I left room for many other
> people to pick up credit." — [TH] §6

> "What mathematicians most wanted and needed from me was to learn my ways of
> thinking, and not in fact to learn my proof of the geometrization conjecture
> for Haken manifolds." — [TH] §6

He names the two failure modes he avoided: "either for me not to let on that I
discovered what I discovered and proved what I proved, keeping it to myself
(perhaps with the hope of proving the Poincaré conjecture), or for me to present
an unassailable and hard-to-learn theory with no practitioners to keep it alive"
([TH] §6). The first is Perelman's shape (`10`); the second is B1.

**(f) MOVE — publish the infrastructure ahead of the theorem.** *Trigger:* a run
has established machinery that would let a different run attack a different
problem. *Action:* write the machinery up separately from the result that
motivated it, in terms that do not mention the motivating problem. *Check:* the
write-up must be usable by someone who does not know the original goal — which
is exactly the discipline `../tao/04` R13's shared technique library would
require, and the reason that item is an operational decision rather than a code
change.

---

## §C Against Tao

| Tao (`../tao/01`) | Thurston | Which, when |
|---|---|---|
| §21–23 a proof is what the kernel accepted | A3/A4: formalisation evaporates the differences between understandings; validity is social and the useful discipline is *legibility*, not checkability | Both, and the runtime has only one. Thurston is not arguing to drop the kernel — he says so — he is arguing that passing it is not the whole of the goal |
| §35 one monotone, legible progress statistic | A1/A2: the measure of success is whether people can think more clearly. Not a statistic | Thurston would reject the framing. The reconciliation is that a statistic on *legibility* is possible and nobody has one |
| §27 archive everything; reuse what is proved | B2: and publish the infrastructure *before* the theorem, so others can reuse it | Agreement, with Thurston naming the ordering. This is `../tao/04` R13 with a mechanism |
| §19 a short proof of a famous problem is probably known | A6: and a proof of a special case may be scaffolding that a general proof will make obsolete | Two different reasons to discount a result. The runtime implements the first and cannot express the second |
| §34 modularise so no participant needs the whole argument | A7: and the right modularisation differs per audience; the expansion factor is the cost | The runtime modularises by *tool authority* and never by audience, though `role_context()` could |

**The one-line version.** The runtime scores an answer. Thurston's whole essay
is the claim that scoring the answer is what killed his first field, and that
the artefact worth optimising is the one a later worker can pick up — which is
also, independently, `../tao/04` R13, `01`§A11 and `03`§A11. Four subjects
reaching the same gap by four routes is the strongest signal in this directory.

---

## Sources

- William P. Thurston, *On proof and progress in mathematics*, Bull. AMS (N.S.)
  30 (1994) 161–177 — <https://arxiv.org/abs/math/9404236>, PDF fetched and read
  in full. Sole source for this file.

Referenced by Thurston and not fetched: Arthur Jaffe and Frank Quinn,
*"Theoretical mathematics": toward a cultural synthesis of mathematics and
theoretical physics*, math.HO/9307227. `[UNVERIFIED]` as a source; Thurston's
characterisation of it is quoted as his.
