# Grothendieck: the rising sea

**The axis.** Tao's method is local and opportunistic — enumerate the
difficulties, switch nine off, try the stupidest thing. Grothendieck's is the
refusal of that. He enlarges the surrounding theory until the theorem is a
consequence of it, declines the direct assault even when the remaining work is
small, and treats the proof as the least interesting part of a result. He is
here because five of the things built on this branch are trick-side and he would
have built none of them.

**Source keys.** **[McL]** = Colin McLarty, *The Rising Sea: Grothendieck on
Simplicity and Generality*, in *Episodes in the History of Modern Algebra*,
<https://www.landsburg.com/grothendieck/mclarty1.pdf>; **[ReS]** =
*Récoltes et Semailles*, 1985–87, in the English translation hosted at
<https://web.ma.utexas.edu/users/slaoui/notes/recoltes_et_semailles.pdf>
(section numbers are that translation's); **[ReS/McL]** = *Récoltes et
Semailles* as translated by McLarty in [McL], who states "All translations in
this paper are my own" and cites the French pagination.

## Accuracy conventions

Grothendieck is heavily mythologised and the myths read like quotations. This
section exists so a later session does not re-litigate it.

- **Two different translations are used here and they are labelled.** McLarty
  translates the nut and rising-sea passage himself from the French pages
  552–555; the Texas-hosted translation covers other parts of the work. Where
  the wording carries the argument, the key says which translation it is. Do not
  splice a sentence from one into a quotation attributed to the other.
- **"I never solve a problem, I dissolve it" is not sourced here and is not
  used.** The verb *dissolve* in this file is McLarty's, describing the effect
  of the rising sea: the theorem is "submerged and dissolved by some more or
  less vast theory" ([ReS/McL], p. 555). The epigram is not in any source
  reached for this file; treat it as apocryphal.
- **The "57 is prime" anecdote is folklore about his indifference to examples,
  not evidence about his method,** and is not used. His own account of his
  weakness at computation is available first-hand and is used instead: "I was
  never very good at computations, I must say, ever since I left high school"
  ([ReS] §2.1).
- **Serre's and Deligne's characterisations are theirs, and are attributed.**
  McLarty sources Serre's remark on Grothendieck's originality to a conversation
  in 1995 and several of Deligne's to e-mail, which is weaker provenance than a
  publication and is flagged where it appears.
- **The mystical register of [ReS] is not evidence of a method.** The yin/yang,
  child-and-Mother and maternal-gestation language is load-bearing to
  Grothendieck and unusable as a control. Where an entry below rests on such a
  passage it is tagged `[INFERRED]` and the operational content is stated
  separately from his imagery.

---

## §A Stated method

### A1. The rising sea, against the hammer and chisel `[STATED]`

His own statement of the two styles, and the single most transferable thing in
this file. A theorem is a nut; you may crack it or you may soften it until it
opens without force.

> "put the cutting edge of the chisel against the shell and strike hard. If
> needed, begin again at many different points until the shell cracks—and you
> are satisfied". … "I can illustrate the second approach with the same image of
> a nut to be opened. The first analogy that came to my mind is of immersing the
> nut in some softening liquid, and why not simply water? From time to time you
> rub so the liquid penetrates better, and otherwise you let time pass. The
> shell becomes more flexible through weeks and months—when the time is ripe,
> hand pressure is enough, the shell opens like a perfectly ripened avocado!
> … the sea advances insensibly in silence, nothing seems to happen, nothing
> moves, the water is so far off you hardly hear it. . . yet it finally
> surrounds the resistant substance." — [ReS/McL], pp. 552–3

He names the other style as Serre's, without disparagement: Serre is the
"incarnation of elegance" ([ReS/McL], p. 969), and Grothendieck says that from
1955 to 1970 Serre was at the origin of most of his ideas ([ReS/McL], p. 982).
The two styles are a pair, not a ranking.

**Agent:** this is a *strategy* field the runtime does not have. Every arm on
this branch is chisel-side — `weakener` lowers the target, `refuter` attacks the
statement, `searcher` hits the score function repeatedly. A rising-sea arm would
be scored on whether the *surrounding theory* grew, not on whether the goal
moved, and would have to survive many attempts producing no progress on the
goal at all. The routing ladder cannot express that today: `STUCK_THRESHOLD = 2`
diversifies away from exactly the behaviour this move requires.

### A2. The theorem should be a byproduct of a theory that overshoots it `[STATED]`

The success condition is not the theorem. It is a theory large enough that the
theorem is one of its smaller consequences.

> the theorem is "submerged and dissolved by some more or less vast theory,
> going well beyond the results originally to be established" — [ReS/McL], p. 555

**Agent:** *overshoot* is measurable and nothing measures it. A run that
establishes a general lemma discharging its goal plus four unasked questions has
done something a run that discharges only its goal has not, and
`research/CLAIMS.md` scores them identically. The `bearing` field in a claim
block is the nearest existing hook.

### A3. Take the definition that applies to everything, not the one that applies to your case `[STATED]`

The consistent move across abelian categories, schemes and toposes: when a
choice arises between a definition carrying hypotheses that make the current
proof work and one carrying none, take the one carrying none.

Serre's testimony is the sharpest evidence, and it is Serre's:

> Grothendieck's originality, according to Serre, was that no one but him
> thought it could work in all generality. Serre thought the rings "should meet
> some conditions, at least be Noetherian". — [McL], reporting a conversation of
> 1995

Grothendieck applied the same rule against himself in print, treating
paracompactness — the hypothesis under which the classical theory was
comfortable — as a "restrictive condition" to be pushed out of the basic theory,
and citing the Weil conjectures as the reason ([Grothendieck 1957], p. 120, via
[McL]).

**Agent:** a firing rule. When a lemma is stated with a hypothesis, ask whether
the hypothesis is used or inherited. `research/BACKWARD.md` already records
`rests-on` per gap, so an inherited hypothesis is detectable: it is one that no
step below cites. Nothing looks. This is the cheapest Grothendieck-derived
check available and it is a graph walk over a ledger the runtime already
derives.

### A4. Find the world the problem is native to, then read the answer off it `[STATED]`

The three-step shape McLarty extracts, with Grothendieck's own statement of the
step that matters:

> (1) Find the natural world for the problem (e.g. the étale topos of an
> arithmetic scheme). (2) Express your problem cohomologically … (3) The
> cohomology of that world may solve your problem, like a ripe avocado bursts in
> your hand. — [McL]'s outline

> "The crucial thing here, from the viewpoint of the Weil conjectures, is that
> the new notion [of space] is vast enough, that we can associate to each scheme
> a 'generalized space' or 'topos' … Certain 'cohomology invariants' of this
> topos ('childish' in their simplicity!) seemed to have a good chance of
> offering 'what it takes' to give the conjectures their full meaning" —
> [ReS/McL], p. P41

**Agent:** the runtime has no representation of *the setting a problem is stated
in*. `inventor` proposes routes to the goal and `reducer` proposes lemmas
sufficient for it; neither proposes a change of category, encoding or ambient
structure in which the goal is restated. Note this is not `weakener`: weakening
lowers the target, reframing keeps it and moves the ground under it. Of the
three directions the branch now has, none is this one.

### A5. Stop when the statement is understood; the proof is trade `[STATED]`

The entry most likely to be misread, and the most consequential. Grothendieck
says that once the statement and its context are properly understood he often
declines to write the proof, because at that point it is routine.

> "Often, notions and statements mesh in such a perfect way, that there can be
> no doubt in my mind as to their validity (give or take small adjustments at
> most) - so that often, when it boils down to 'travail sur pièces' destined for
> publication, I refrain from going further, and from taking the time to flesh
> out a proof that often, once the statement and its context are
> well-understood, consists of no more than a matter of 'trade', not to say
> routine." — [ReS] §2.6

The practice is documented, not merely claimed: he found Grothendieck–Riemann–
Roch in 1957 and left Borel and Serre to publish the proof ([McL]).

**Agent:** this is a direct challenge to `Status::Formalised` and to
`reflection`'s `SOLVED` conditions, and it should not be adopted naively — an
LLM asserting that the rest is routine is the exact failure `lean_check` was
built to catch, and Grothendieck's confidence was backed by a track record no
run has. What survives the challenge is narrower and worth having: a *standing*
for a result whose statement and setting are established and whose proof is
outstanding-but-believed. The ledger has `asserted` for this and treats it as a
deficiency. For a run, distinguishing "asserted because nobody checked" from
"asserted because the remaining step is mechanical and named" is real
information that the schema currently discards.

### A6. Prefer a fertile viewpoint to a key theorem `[STATED]`

His own account of what he contributes, and it is not theorems.

> "I am led more towards the discovery of fertile viewpoints than towards the
> discovery of questions, notions, and statements … Even more so than what we
> call 'key theorems' in mathematics, it is the fertile viewpoints which, in our
> art, constitute the most powerful tools of discovery - or rather, they are not
> tools, but they are the very eyes of the researcher" — [ReS] §2.6

**Agent:** `research/APPROACHES.md` is the closest ledger — it holds ideas with
a `mechanism` and a lifecycle — but its statuses are `proposed → grounded →
refuted / adopted / spent`, every one of them measured against the current goal.
A viewpoint that reorganised the run's understanding while adopting nothing has
no status to be in. `spent` is what it would be given, and that is the wrong
answer.

### A7. Multiply the viewpoints; a converging sheaf of them is worth more than any one `[STATED]`

> "It is when complementary viewpoints of a common reality are conjugated, that
> is, when our 'eyes' are multiplied, that the gaze is able to penetrate further
> ahead … a sheaf of viewpoints converging to a unique and vast scenery, gives
> rise to a novel thing; a thing which transcends each of the partial
> perspectives" — [ReS] §2.6

**Agent:** the runtime already fans out after an attempt — judge, reflect,
patterns, invention, refutation, reduction — and the merge folds counters by
delta. What it does not do is look for *agreement between arms as a signal*. Two
arms reaching the same lemma by different routes is the cheap version of this,
and `closure.rs` now has the machinery to notice: a claim reachable through two
disjoint `follows-from` chains is a different object from one reachable through
one.

### A8. Work from your own eyes, not from the authority of the group `[STATED]`

The lesson he draws from three years of isolated work in Montpellier, and he
states it as a skill rather than a temperament.

> "I learned in those crucial years to 'be alone'. That is, I learned to
> approach the things which I want to know with my own eyes, rather than rely on
> the expressed or implicit ideas that eminate from the group with which I
> identify, or a group to which I attribute authority." — [ReS] §2.2

**Agent:** the counterweight to research gating's opposite failure. `MEMORY.md`
and `CLAIMS.md` already separate `catalogued` from `established` — a lookup may
confirm an answer but never be the reason for one — which is this principle
encoded. Grothendieck's addition is that the *frame* is as borrowed as the
answer: a run that adopts a paper's parameterisation has taken more from the
literature than the ledger records.

### A9. Rediscovering known work is not wasted `[STATED]`

He spent three years alone reconstructing Lebesgue measure, was told he had
wasted his time, and did not agree.

> "According to the two or three experts to whom I mentioned my work (or even
> showed a manuscript), I had just wasted my time redoing something 'already
> known'. I actually do not recall being disappointed." … "Unknowingly, I
> learned in solitude what is essential to the work of a mathematician -
> something no master could truly teach." — [ReS] §2.2

**Agent:** the exact inverse of the post-solve novelty check built on this
branch out of `../tao/04` R33. Both are right and they bound each other: a
*reported result* that duplicates the literature is a reporting failure, while
*internal machinery* that duplicates the literature may be the run's only real
understanding of it. The novelty check must therefore run on the reported
answer, never on the derivation — which is how it was in fact built, and this
entry is the argument for keeping it that way.

### A10. Do not rush the last mile once the outcome is determined `[INFERRED]`

By 1958 Serre had produced the one-dimensional Weil cohomology and Grothendieck
had convinced him it extended to all dimensions. The remaining work was, on his
own view, settled.

> Grothendieck's optimism grew from his method: Cohomology is uniquely
> determined, once you know what you want the cohomology of. … So the job was
> finished in principle—from Grothendieck's viewpoint—but he did not rush to
> work it all out. That would be striking hard at the chisel. — [McL]

Instead he built Grothendieck topologies and toposes, and the conjectures took
until 1974 and were finished by Deligne.

**Agent:** tagged `[INFERRED]` because it is McLarty's reading, and included
because it is the honest cost of A1. The rising sea took sixteen years and
someone else closed it. A runtime with a wall-clock ceiling and
`MAX_ATTEMPTS = 8` cannot run this strategy, and should not pretend otherwise:
proposing a rising-sea arm means proposing a budget policy, not a prompt.

### A11. The generative phase goes unrecorded, and that is a loss `[STATED]`

His complaint about mathematical writing, which is precisely a complaint about
the artefact this runtime produces.

> "this 'most creative part of all' within a work of discovery … is reflected
> almost nowhere in the texts and monologues which are supposed to present work
> of this kind … a sort of 'conspiracy of silence' surrounding these
> 'unspeakable labors' which precede the birth of each new idea" — [ReS] §3.6

He extends it to disdain for anything "that hasn't been written and published in
black on white, in the form of plain statements, classifiable and classified,
ready to be incorporated into the 'databases'" — which is, read literally, a
complaint about a derived ledger ([ReS] §3.6).

**Agent:** the runtime is unusually well placed here and half-uses it. The
workspace commits the derivation, the programs and the per-run notes, and
`.workspace-history` checkpoints every write, so the generative record exists.
But `reflections/` is loop-owned and indexed, `trace.jsonl` is gitignored as
several megabytes of noise, and no derived ledger reads the abandoned attempts.
The record is kept and not used. See A6: this is the same gap.

---

## §B Anatomy

### B1. Tôhoku — abelian categories (1955–57)

**(a)** Sheaf cohomology existed in several incompatible constructions; the
Séminaire Cartan sought unity. The concrete question was which categories admit
a derived-functor cohomology.

**(b)** Redefine the object rather than the theorem. A category of sheaves
became "any Abelian category with a generator and enough injectives" ([McL]),
so the cohomology theorems stopped depending on what sheaves *are*. Deligne's
summary: "Grothendieck had shown that, given a category of sheaves, a notion of
cohomology groups results" ([Deligne 1998], p. 16, via [McL]).

**(c)** Cartan–Eilenberg homological algebra; Serre's 1951 clarification of
spectral sequences. Grothendieck's own report on acquiring the tool is worth
recording as an attitude: "I am rid of my horror of spectral sequences"
(letter to Serre, via [Colmez & Serre 2001], p. 7, quoted in [McL]).

**(d)** Nothing computed. The paper is definitional.

**(e)** Roughly two years. A few pages of the Séminaire Cartan became 102 pages
of category theory; "Many people found the work completely disproportionate to
the problem" and it took two years to place ([McL]). The generality was
speculative at the time — the axioms "go far beyond topological and group
cohomology, in principle, though in fact there were few if any known examples
outside that framework when they were given" ([McL]).

**(f) MOVE — axiomatise the ambient category, not the object.** *Trigger:* a
theorem is being reproved separately in several settings that differ only in
what the objects are made of. *Action:* isolate the properties the proofs
actually use, promote them to axioms on the ambient structure, and reprove once.
*Check:* the axioms must be satisfiable by a structure outside the motivating
family, or the generalisation is a renaming. Note the honest failure mode
recorded above — Grothendieck could not meet this check in 1957 and was right
anyway, which is why the check is a caution and not a gate.

### B2. Schemes (1958–)

**(a)** Algebraic geometry lacked a definition of its spaces adequate to
arithmetic. Weil's varieties and Serre's FAC varieties were each tied to a field.

**(b)** Every commutative ring defines a space. `Spec(R)`, points are prime
ideals, schemes are pasted from spectra — with no finiteness, no Noetherian
hypothesis, no ground field.

**(c)** Serre's FAC structure sheaves; Zariski's topology on prime ideals, which
Grothendieck himself called "classical" at the 1958 ICM and "well known" the
next year in the Séminaire Bourbaki ([McL]). The construction was not the
contribution.

**(d)** Nothing. The bet was definitional, and Deligne states the payoff in
those terms: "if the decision to let every commutative ring define a scheme
gives standing to bizarre schemes, allowing it gives a category of schemes with
nice properties" ([Deligne 1998], p. 13, via [McL], Deligne's emphasis).

**(e)** The prior ladder is dense and was not the obstacle: Chevalley–Nagata
schemes and Cartier's spectra already covered the constructions. The obstacle
was that nobody would drop the hypotheses. Grothendieck and Dieudonné's own
warning names the cost — readers "will no doubt have some trouble before they
are accustomed to the language of schemes", the difficulty being "psychological"
([Grothendieck & Dieudonné 1960], p. 9, via [McL]). By 1962 Serre could say at
the ICM that algebraic geometry meant "the theory of schemes" ([Serre 1963],
p. 190, via [McL]).

**(f) MOVE — drop the hypothesis that only convenience justifies.** *Trigger:* a
definition carries a hypothesis, and no step of the theory below it invokes that
hypothesis. *Action:* delete it and admit the degenerate objects that appear.
*Check:* the new objects must be *closed under the operations you need* — that
is the whole of the payoff Deligne names, and the "bizarre schemes" are the
price. If deleting the hypothesis breaks closure rather than improving it, the
hypothesis was load-bearing.

### B3. Étale cohomology and the Weil conjectures (1958–1974)

**(a)** Weil (1949) conjectured that counting points of varieties over finite
fields behaves as though those point sets were topological manifolds. Given a
cohomology with a Lefschetz fixed-point theorem, the conjectures reduce to
"a pair of graduate exercises in linear algebra" ([McL]). No such cohomology
existed and the spaces were "everywhere discontinuous" ([McL]).

**(b)** Do not construct the cohomology. Enlarge what a *space* is until the
required cohomology is the automatic one. Grothendieck topologies, then toposes;
the étale topos of a scheme, whose cohomology is simply the abelian groups in
that world.

**(c)** Serre's cohomological restatement of the conjectures, which
Grothendieck says was the precondition of his interest: "Serre explained the
Weil conjectures to me in cohomological terms around 1955—and it was only in
these terms that they could possibly 'hook' me" ([ReS/McL], p. 840). Serre's
1958 isotrivial coverings supplied `H¹`; Serre thought he had "brutally forced"
the maps and was "absolutely unconvinced" it generalised ([McL], conversation,
1995).

**(d)** Nothing, at any stage.

**(e)** ~25 years from Weil's conjectures, with a long ladder of special cases
in dimension 1 and for hypersurfaces. Grothendieck proved the first and second;
Deligne finished the Riemann-hypothesis analogue in 1974. Grothendieck did not
close his own programme.

**(f) MOVE — enlarge the ambient notion until the wanted structure is forced.**
*Trigger:* the problem needs a construction that provably cannot exist on the
objects as currently defined. *Action:* generalise the *ambient notion* — not
the objects, not the target — until the construction is the canonical one there,
then transport the problem in. *Check:* the enlarged notion must reproduce the
classical cases as instances. Toposes clear this: each topological space and
each group determines one, with its usual cohomology ([McL]). *Cost, stated
honestly:* sixteen years and someone else's endgame.

### B4. Grothendieck–Riemann–Roch (1957)

**(a)** Hirzebruch's Riemann–Roch theorem, generalised. Not a problem of his own
choosing — he says Serre put him onto it ([ReS/McL], pp. 554–5).

**(b)** He states it was done by the rising sea, on someone else's problem
([ReS/McL], pp. 554–5). The generality is the mechanism: Bott's review notes the
theorem is "more generally applicable than Hirzebruch's version" *and* "depends
on a simpler and more natural proof" (Math. Reviews 22 #6817, via [McL]).

**(c)** Hirzebruch's theorem; sheaf cohomology; his own emerging functorial
machinery.

**(d)** Nothing computed. He did not write the proof — Borel and Serre published
it ([Borel & Serre 1958]). This is A5 in practice.

**(e)** Fast by his standards, and the reputational turning point: it "made me a
'big star' overnight", first dispelled Bourbaki's doubts, and left him
"somewhat feared" at the 1958 ICM ([ReS/McL], pp. P23, 705, 32).

**(f) MOVE — generalising can be the cheaper proof, not the dearer one.**
*Trigger:* a known theorem's proof is hard and case-bound. *Action:* look for
the more general statement, on the specific bet that the special structure the
old proof exploits is what makes it hard. *Check:* the new proof must be shorter
or more uniform than the old, not merely broader — Bott's review certifies both,
and only the conjunction distinguishes this move from generality for its own
sake.

---

## §C Against Tao

| Tao (`../tao/01`) | Grothendieck | Which, when |
|---|---|---|
| §1 turn off nine of ten difficulties and solve that | A1/A4: change the setting so the difficulties are not features of it | Tao's when the difficulties are independent and enumerable; Grothendieck's when they are all shadows of one bad encoding. `weakener` implements Tao's and nothing implements the other |
| §3 try anything, the stupider the better — the failure is informative | A1: the doomed attempt is the chisel. Value comes from waiting | Tao's on a cold start with no reading of the problem; Grothendieck's once a viewpoint exists and is unfinished. Note his is unaffordable under a fixed attempt cap |
| §5(a) special cases first, "start with modest assumptions" | A3: take the definition with no hypotheses at all | Directly opposed and both are sourced. The reconciliation is that Tao specialises the *problem* and Grothendieck generalises the *definitions* — but Serre, who is no less able, took Tao's side on Noetherian rings and was wrong |
| §19 a short proof of a famous problem is probably known | A9: rediscovering known work taught him what no master could | Not in conflict once separated: the check belongs on the reported answer, never on the derivation. This is how the branch built it |
| §20 numerics before theory | B1–B4: nothing computed in any of the four | The sharpest disagreement in the file. The runtime's method policy opens with a naive oracle and Grothendieck never once needed one. See `12-cross-cutting.md` |
| §35 one monotone, legible progress statistic | A1/A10: sixteen years in which the statistic does not move | Fatal to the rising sea as currently scheduled. A progress metric that reads the goal will kill this strategy in two attempts |

**The one-line version.** Everything the runtime does after an attempt asks
whether the *goal* moved. Grothendieck's entire method is a bet that the goal
should not move for a long time while the theory around it does, and the runtime
has no way to tell that state apart from being stuck.

---

## Sources

- Colin McLarty, *The Rising Sea: Grothendieck on Simplicity and Generality* —
  <https://www.landsburg.com/grothendieck/mclarty1.pdf> (fetched; the source of
  every [McL] and [ReS/McL] quotation here, including its own translations from
  the French pagination of *Récoltes et Semailles*)
- *Récoltes et Semailles*, English translation —
  <https://web.ma.utexas.edu/users/slaoui/notes/recoltes_et_semailles.pdf>
  (fetched; source of every [ReS] quotation, with that translation's section
  numbering)
- nLab, *The Rising Sea* — <https://ncatlab.org/nlab/show/The+Rising+Sea>
  (consulted for corroboration of the passage's location; not quoted)
- IHES, publication announcement for *Récoltes et semailles* —
  <https://www.ihes.fr/en/publication-recoltes-et-semailles-grothendieck/>
  (consulted for provenance of the text; not quoted)

Works cited *through* McLarty and not fetched directly — [Deligne 1998],
[Colmez & Serre 2001], [Grothendieck 1957], [Grothendieck 1958],
[Grothendieck & Dieudonné 1960/1971], [Borel & Serre 1958], [Serre 1963], and
Bott's *Mathematical Reviews* notice — are cited above as McLarty reports them
and were not independently verified.
