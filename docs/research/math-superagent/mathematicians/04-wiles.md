# Wiles: the long siege, and the abandoned branch that saved it

**The axis.** Seven years on one problem, in secret, with no intermediate
publication and no collaborator until the repair. Everything in this runtime is
built for a bounded run — `MAX_ATTEMPTS = 8`, a wall-clock ceiling,
`STUCK_THRESHOLD = 2` — and Wiles is the case that says what a run looks like
when none of those exist. He is also the cleanest documented instance of a
*failed approach turning out to be the material for the repair*, which is a
finding the runtime's ledgers currently make unrepresentable.

**Source keys.** **[NOVA]** = Wiles interviewed for NOVA, *The Proof*, PBS,
<https://www.pbs.org/wgbh/nova/proof/wiles.html>; **[Horizon]** = the BBC
Horizon documentary *Fermat's Last Theorem* (Singh & Lynch, 1996), quoted at
second hand — see conventions; **[WP]** = the Wikipedia article *Wiles's proof
of Fermat's Last Theorem*,
<https://en.wikipedia.org/wiki/Wiles%27s_proof_of_Fermat%27s_Last_Theorem>,
used for the technical timeline.

## Accuracy conventions

- **The "dark mansion" passage is quoted at two removes and is flagged as
  such.** The wording used below was reproduced from
  <https://micromath.wordpress.com/2011/11/06/andrew-wiles-on-doing-mathematics/>,
  which attributes it to Frans Oort quoting the 1996 BBC Horizon documentary.
  Neither the documentary nor Oort's paper was reached. It is the most-quoted
  Wiles line in existence and it is *not* independently verified here.
- **[NOVA] quotations came through a summarising fetch** of the PBS interview
  page and are reported as that fetch returned them. Treat them as reported
  verbatim, not independently confirmed. Note in particular that the fetch
  reported one line ("after a year of work, and after inviting the Cambridge
  mathematician Richard Taylor…") that reads as the *interviewer's* question
  rather than Wiles's answer; it is not used as a Wiles quotation below.
- **The September 1994 quotation is Singh's, reported via [WP].** The wording
  "I realised that, the Kolyvagin–Flach method wasn't working, but it was all I
  needed to make my original Iwasawa theory work from three years earlier" is
  attributed by [WP] to Wiles as quoted by Simon Singh. Singh's book was not
  reached.
- **The technical account in §B1 rests on [WP], not on Wiles's papers.** The
  1995 *Annals* papers were not fetched. Where the mechanism matters to a
  harness conclusion, the conclusion is stated so that it survives minor errors
  in the mathematics.
- **Wiles wrote very little about method.** §A is short for that reason and the
  weight is in §B, per the rule in [`00-conventions.md`](00-conventions.md).

---

## §A Stated method

### A1. The dark mansion — orientation precedes progress `[STATED]`

His account of what the first months of a problem are, and the reason it is
quoted so often is that it describes a phase in which nothing measurable
happens.

> "Perhaps I could best describe my experience of doing mathematics in terms of
> entering a dark mansion. One goes into the first room, and it's dark,
> completely dark. One stumbles around bumping into the furniture, and gradually,
> you learn where each piece of furniture is, and finally, after six months or
> so, you find the light switch. You turn it on, and suddenly, it's all
> illuminated. You can see exactly where you were." — [Horizon], at two removes

**Agent:** the runtime cannot represent this phase. `STUCK_THRESHOLD = 2` sends
a run to `diversify` after two unproductive attempts, and "learning where the
furniture is" produces no claim, discharges no gap, and reads as unproductive by
every derived ledger. The proposal this suggests is not to raise the threshold —
that just wastes budget — but to make *orientation* a scoreable outcome:
an attempt that added definitions, worked examples, and a map of what the
objects are has done the thing Wiles is describing, and `research/THREADS.md`
is the ledger where it would show.

### A2. Undivided concentration, bought with secrecy `[STATED]`

> "I realized that anything to do with Fermat's Last Theorem generates too much
> interest. You can't really focus yourself for years unless you have undivided
> concentration, which too many spectators would have destroyed." — [NOVA]

**Agent:** the runtime is the opposite by construction — nineteen roles, a
standing review team, a judge scoring conduct on a cadence, and a director that
can inject a human directive at any boundary. Wiles's claim is that observation
has a *cost*, and the runtime has never measured its own. The concrete question
this raises is answerable: `docs/solution-loop.md` already flags that four of
five standing teams duplicate a loop arm and says the duplication was never
chosen. Wiles is an argument for resolving that by measurement sooner rather
than later.

### A3. Work the problem that matters most to you `[STATED]`

> "Always try the problem that matters most to you." — [NOVA]

Paired with the fact that he waited: he took the problem up only when Ribet's
1986 theorem made it, in his words as reported by [WP], "professionally
justifiable".

**Agent:** the Erdős point (`02`§A1) from the other side. Erdős says choose the
problem that isolates a difficulty; Wiles says choose the one you will still be
working on in year six, and *wait for the reduction that makes it tractable*.
The runtime is handed its goal and has no intake step at all, so neither applies
today — but the second is the one with a mechanical form. Ribet's theorem is a
reduction from the outside that changed the problem's status, and
`research/FRONTIER.md` is where such a thing would arrive if anything watched
for it.

### A4. Walk when stuck; the subconscious is doing work `[STATED]`

> "When I got stuck and I didn't know what to do next, I would go out for a
> walk. I'd often walk down by the lake." … "Walking has a very good effect in
> that you're in this state of relaxation, but at the same time you're allowing
> the sub-conscious to work on you." — [NOVA]

**Agent:** the honest reading is that this has no machine analogue and should
not be given a fake one. The temptation is to map it onto "spawn a divergent
arm", which is not what he describes — he describes *stopping*. The one real
transfer is negative: a runtime that always has an arm running has no state
corresponding to this, and should not claim to.

---

## §B Anatomy

### B1. Fermat's Last Theorem (1986–1994)

**(a)** As posed: `xⁿ + yⁿ = zⁿ` has no positive integer solutions for `n > 2`.
Considered inaccessible. Wiles did not attack it.

**(b)** The reframing was done by others and he waited for it. Frey (1982–85)
attached to any counterexample an elliptic curve that could not be modular;
Ribet's 1986 theorem made this a proof: modularity for semistable elliptic
curves implies Fermat. Wiles then worked exclusively on modularity and never on
Fermat again ([WP]).

This is the whole shape of the solve, and it is worth stating baldly: **the
famous problem was never attacked.** A different, harder, more general statement
was, because a reduction existed that nobody was going to find by staring at
Fermat.

**(c)** Imported: Frey's curve and Ribet's theorem, from arithmetic geometry;
Langlands–Tunnell, which supplies the base case by making irreducible mod-3
representations modular; horizontal Iwasawa theory; the Kolyvagin–Flach Euler
system, learned in summer 1991 and described as "tailor made" for the induction
([WP]).

**(d)** Nothing computed. There is no numerical component to this proof at any
stage — a striking fact for a runtime whose method policy opens with a naive
oracle.

**(e)** The ladder is the point of the entry.

| Date | State |
|---|---|
| 1986 | Ribet's theorem lands. Wiles begins, in secret |
| 1991 | Horizontal Iwasawa theory attempted, fails to yield the class number formula, **abandoned** |
| summer 1991 | Kolyvagin–Flach adopted instead |
| spring 1993 | All but a few families covered |
| 21–23 June 1993 | Announced at Cambridge |
| ~Aug 1993 | Katz's referee questions expose the gap: the Euler system extension is incomplete, so a bound on a group's order is unproved. No proof of Fermat exists |
| late 1993 | Rumours the proof has failed. Richard Taylor joins |
| 19 Sept 1994 | The repair |
| 24 Oct 1994 | Two manuscripts submitted; published May 1995 as the whole issue |

The repair is the finding:

> "I realised that, the Kolyvagin–Flach method wasn't working, but it was all I
> needed to make my original Iwasawa theory work from three years earlier." —
> Wiles, quoted by Singh, via [WP]

The approach abandoned in 1991 was completed by what was learned from the
approach that replaced it and then failed. Neither alone sufficed. He called it
"the most important moment of my working life" ([WP]).

A second, smaller move deserves recording because it is exactly the kind a
runtime could make: the **3–5 switch**. Modularity is proved at `p = 3`, where
Langlands–Tunnell gives the base case, except when the mod-3 representation is
reducible — there one switches to `p = 5`; and when both are reducible, one
introduces an auxiliary curve `F` with irreducible mod-3 and isomorphic mod-5
representations, and routes through it. Wiles noticed the switch in a 1993 Mazur
paper ([WP]).

**(f) MOVE — a killed approach is inventory, not a dead end.** *Trigger:* the
current approach fails against a specific obstruction. *Action:* before
discarding it, check every *previously abandoned* approach against what the
current failure taught. The pairing is the unit, not either approach.
*Check:* the revival must name which specific piece of the failed approach
supplies the missing piece of the revived one. "Retry the old idea" without that
is a loop, and it is what an unconstrained version of this move degenerates to.

**(f′) MOVE — carry an auxiliary parameter and switch it on the bad case.**
*Trigger:* the argument works except on a characterised subclass. *Action:* look
for a second instantiation of the same argument, at a different parameter, that
covers exactly the bad subclass, and an auxiliary object that bridges the
overlap. *Check:* the union of the cases must be exhaustive and the bridge must
be constructible in the doubly-bad case — Wiles's `F` is that, and it is the
only hard part.

---

## §C Against Tao

| Tao (`../tao/01`) | Wiles | Which, when |
|---|---|---|
| §2 count your debts; several independent problems means abandon the approach | B1: the abandoned approach was the answer, three years later | Not contradictory, but Tao's rule is incomplete. Abandoning was correct in 1991; *deleting* would have been fatal. The runtime deletes — `research/APPROACHES.md` has `refuted` and `spent` and no revivable state |
| §16 record which techniques are known not to apply | B1: and record what the *later* failure teaches about the earlier one | A cross-product the runtime does not compute. Immunity is recorded per-approach and never re-examined when a different approach fails |
| §31–36 collaboration, records, scale | A2: undivided concentration, bought by telling nobody for seven years | Directly opposed, and Wiles's is the minority position that produced the result. See `12-cross-cutting.md`; note Taylor's arrival was necessary for the repair, so the pure form failed at the end |
| §20 numerics before theory | B1: nothing was computed, at any point, over eight years | The third of four subjects so far with no numerical component. `12-cross-cutting.md` |
| §35 one monotone, legible progress statistic | A1: six months of bumping into furniture before the light switch | Wiles is the strongest case against a progress statistic that reads the goal, and the strongest case *for* one that reads orientation |

**The one-line version.** The runtime treats a failed approach as terminal state
and a stuck run as a signal to diversify. Wiles's proof exists because a failed
approach was kept for three years and a stuck period lasted six months. The
gap is not the thresholds; it is that `refuted` and `spent` are absorbing states
in a ledger nothing ever revisits.

---

## Sources

Fetched for this file:

- NOVA, *The Proof* — Andrew Wiles interview,
  <https://www.pbs.org/wgbh/nova/proof/wiles.html> (summarising fetch)
- *Wiles's proof of Fermat's Last Theorem*,
  <https://en.wikipedia.org/wiki/Wiles%27s_proof_of_Fermat%27s_Last_Theorem>
  (the timeline, the Iwasawa/Kolyvagin–Flach mechanism, the 3–5 switch, and the
  Singh quotation)
- *Andrew Wiles on doing mathematics*,
  <https://micromath.wordpress.com/2011/11/06/andrew-wiles-on-doing-mathematics/>
  (the dark-mansion wording and its attribution chain)

Not reached, and therefore `[UNVERIFIED]` as sources: Singh & Lynch,
*Fermat's Last Theorem*, BBC Horizon 1996; Simon Singh, *Fermat's Last Theorem*
(book); Frans Oort, *Did earlier thoughts inspire Grothendieck?*; Wiles,
*Modular elliptic curves and Fermat's Last Theorem*, Annals of Mathematics 141
(1995); Taylor & Wiles, *Ring-theoretic properties of certain Hecke algebras*,
same issue.
