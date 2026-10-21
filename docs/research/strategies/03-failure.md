# Failure: what a failed attempt is worth

All eleven subjects agree that a failed attempt carries information, and that
the solver has to extract it on purpose (`mathematicians/12`, "Where all eleven
agree", item 2). The earlier runtime is the cautionary case. It recorded its
failures and then read none of them: abandoned attempts sat in files no later
step consulted, and a refuted claim was simply closed.

supermath is at risk of the same thing in a smaller form. Today a run that ends
`not-established` writes its verdict and up to 6,000 characters of its reply to
memory (`memory_note` in `crates/supermath-agent/src/harness/prompt.rs`). The
next run recalls that text as a lead. Whether the useful part of the failure
survives depends on where it happened to fall in the reply, and whether recall
happens to surface it. The strategies here make the useful part explicit.

## 1. Record where each attempt failed

Tao's version: partial progress is the unit of work, and every attempt should
leave behind its failure coordinate, meaning which step broke, on which
hypothesis, over which range of parameters. The next attempt is then chosen to
cover what the last one did not (`tao/01`§25).

**In supermath.** *Verdict*. A `not-established` reply should end with a short
structured block before the verdict line, one entry per attempt, naming the
approach, the step it failed at, and the difficulty from the intake list
([`01-intake.md`](01-intake.md), section 2) that stopped it. The harness stores
that block as its own memory note, separate from the prose. A recall for the
next run then returns "approach X failed at step 3 on difficulty 2 for n > 40"
rather than a fragment of a long reply.

**Guard.** "It didn't work" is not a coordinate. An entry with no step and no
difficulty should be treated as missing.

## 2. Never stop at "no idea": try the stupid thing

Tao's cure for a run with no plan is to try something almost certain to fail,
because the way it fails points at the hypothesis that matters (`tao/01`§3).

> "the next step then is to try anything no matter how stupid and in fact
> almost the stupider, the better, which technically is almost guaranteed to
> fail, but the way it fails is going to be instructive. It fails 'cause you
> are not at all taking into account this hypothesis. Oh, this hypothesis must
> be useful. That's a clue." (`tao/01`§3)

**In supermath.** *Prompt*. The solver is not allowed to end a run with "no
applicable technique". If it has no plan, it makes a cheap doomed attempt and
records which hypotheses that attempt ignored. Those hypotheses become the
focus of the next attempt, and they go to memory as the run's finding.

**Guard.** The doomed attempt must be cheap. Its purpose is to name an unused
hypothesis. If it turns into a long derivation, it has stopped being
reconnaissance.

## 3. Repair a refuted statement before filing it

Two identities Ramanujan wrote down around 1920 turned out to be false, each
spoiled by divergent series. In 2016 a paper on them did not discard the
claims. It repaired one, and the repair produced corrected convergent series,
new identities, a generalisation of Voronoi summation, and a new class of
integral transforms (`mathematicians/07`§B2). Tao gives the same instruction
for numerics: a conjecture refuted by computation should be edited into a
survivor rather than dropped (`tao/01`§20). The cross-cutting study lists this
among its three cheapest proposals, because the refuter already runs and the
only change is to ask it one more question.

**In supermath.** *Prompt* and *verdict*. When a run reaches `refuted` (on the
problem, or on a conjecture it formed along the way), it asks what the nearest
true statement is: which hypothesis, quantifier, range or convergence condition
would have to change. It writes that statement down as a new conjecture with
the counterexample that motivated it. The repaired statement goes to memory as
an open lead. It is not established, and should never be recalled as if it
were.

**Guard.** Repair without limit weakens any statement until it says nothing.
The repaired statement must still imply something the original problem cared
about, and the counterexample that killed the original must not also kill it.
For a problem in `problems/`, the original verdict stays `refuted`. The repair
is an addition, not a substitute.

## 4. Treat an abandoned approach as inventory

In 1991 Wiles abandoned horizontal Iwasawa theory and switched to the
Kolyvagin-Flach method. In 1993 the gap found in his announced proof was in
the Kolyvagin-Flach part. The 1994 repair was that the failed method supplied
what the abandoned one had been missing, and neither worked alone
(`mathematicians/04`§B1).

> "I realised that, the Kolyvagin-Flach method wasn't working, but it was all I
> needed to make my original Iwasawa theory work from three years earlier."
> (Wiles, quoted by Singh, via `mathematicians/04`§B1)

**In supermath.** *Memory* and *prompt*. When an approach fails on a specific
obstruction, the run checks the earlier failed approaches on this problem
(recalled from memory, using the coordinates from section 1) against what the
new failure taught. The unit is the pair: does what A lacked match what B
provides? This is cheap once failures are stored with coordinates, and nearly
impossible when they are stored as prose.

**Guard.** Without a named piece, this degenerates into "retry the old idea",
which is a loop. A revival has to say which specific part of the newly failed
approach supplies which specific missing part of the old one.

## 5. Decide when to stop by debts, not by a fixed count

The earlier runtime stopped after a fixed number of attempts and a fixed number
of unproductive ones. The cross-cutting study's sharpest observation about it
is that those two constants decided its methodology without anyone noticing:
under a short budget, every strategy that needs patience is ruled out, and no
document recorded that choice (`mathematicians/12`, Split 1). Tao's alternative
is to drive the stop decision by the debt count from
[`02-attack.md`](02-attack.md), section 2, and to let a near miss raise the
budget rather than end the run (`tao/01`§26). On a near miss, he looks for a
strategy that never produces the problem term instead of patching the one that
does.

**In supermath.** *Tool*, once the harness runs more than one turn. Today a run
is one turn, so the budget is whatever OpenHuman allows. When the harness gains
a multi-turn loop, the continue-or-stop rule should read the reply's open
debts and failure coordinates. One open debt with everything else checked is a
reason to continue. Several independent debts is a reason to stop and record
why. Whatever the constants are, they go in configuration with a comment
saying what they rule out, so the choice is at least recorded.

**Guard.** A near miss is easy to claim. "One debt left" should mean one
`ASSUME:` line in a reply that otherwise checks, not a model's estimate of how
close it got.

## 6. Archive everything, and read the archive

Tao uses the wastebasket freely but keeps everything, and the point of keeping
it is that abandoned branches, with their reasons, are fed into later attempts
(`tao/01`§27). The third agreement across the study is the same idea seen from
the reader's side: the earlier runtime kept a full record and no step read it
(`mathematicians/12`, item 3).

**In supermath.** *Memory*. CortexDB memory per problem already gives us the
archive. What it does not yet give us is a guarantee that the dead ends are
read. Two changes do that. First, dead-end notes are written as their own kind
of note (see [`05-memory.md`](05-memory.md)), so they are recalled as a group.
Second, the turn message presents them in a section of their own, "approaches
already tried and why they failed", before the solver starts, rather than
mixed into a general "memory recalled these notes" block.

**Guard.** An archive that only grows becomes noise. Notes from one problem
should be merged when they describe the same dead end, and recall should favour
the failure coordinate over the full reply that contained it.

## 7. Count a clean negative result as a result

Tao insists that quality has many dimensions and that a run with no answer can
still have produced something worth having: a clean obstruction, a reusable
lemma, or a clear explanation (`tao/01`§30). A failure verdict with its reason
attached is a first-class artifact, not a placeholder (`tao/01`§32). And when
the known methods cannot reach a problem, explaining why they fail is the
deliverable (`tao/01`§36).

**In supermath.** *Verdict*. The three-valued verdict is right and should stay
three-valued. What `not-established` needs is the reason and the findings to
travel with it: debts left open, failure coordinates, repaired conjectures,
obstructions found, bounds searched. The verdict line stays exactly as it is;
the structured block above it carries the rest. Then a `not-established` run
that found an obstruction can be told apart, in memory and in any later
report, from one that found nothing.

**Guard.** None of these findings may change the verdict. An obstruction is not
a refutation, and a repaired conjecture is not a proof of anything. The parser
already refuses to infer a positive verdict, and it should refuse in the same
way to let a finding upgrade one.
