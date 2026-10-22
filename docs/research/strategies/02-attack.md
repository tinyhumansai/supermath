# Attack: moves that make progress on the goal

These strategies cover the part of a run that does the actual work. The current
harness leaves all of it to one model call, which decides for itself every
turn what to try next and says nothing about why. Most of what follows is about
making those choices visible: a named move, a stated assumption, a decomposition
that had to pass a test. Visible choices can be checked, compared across runs
and stored in memory. Silent ones cannot.

Nearly everything here comes from the "attack the problem where it stands"
side of the study (Tao, Gowers, Wiles, Erdős, Zeilberger). The other side,
which enlarges the setting until the problem gets easy, appears in sections 9
and 10 with the conditions under which we would use it. Why the balance falls
this way is in [`06-settings.md`](06-settings.md), Split 1.

## 1. Cheat strategically

Tao's first move on an unfamiliar problem: build a deliberately falsified
version with all but one difficulty switched off, solve that, and then add the
difficulties back (`tao/01`§1). Mathematics allows this because the solver owns
the statement.

> "if there are 10 things that are making your life difficult, find a version
> of the problem that turns off nine of the difficulties, but only keeps one of
> them and solve that." (`tao/01`§1)

**In supermath.** *Prompt* first, *memory* next. Using the difficulty list from
intake ([`01-intake.md`](01-intake.md), section 2), the solver names each
variant by which difficulties it keeps ("finite field model, difficulty 3
only"). Each solved variant goes to memory keyed by the difficulties it
covers. Progress on the problem then has a shape a later run can read: which
subsets of difficulties are handled, and which combination is still open. Tao's
own measure of progress is the size of the largest solved subset, not
solved-or-unsolved, and that measure is useful to us mainly because the
`not-established` verdict currently carries nothing.

**Guard.** A variant can drop the difficulty that matters and keep only the
easy ones. Every variant should say which difficulty it keeps, and the run
should not report progress on a variant that keeps none of the hard ones.

## 2. Assume the blocker away, then count the debts

When one bad case blocks an argument, Tao grants it by fiat and pushes the rest
through, to learn whether that case is the only problem. One debt means fight
it. Several independent debts mean give up on the approach (`tao/01`§2).

> "You can just assume by fiat this bad case doesn't occur. So you do some
> magical thinking, but strategically okay for the point to see if the rest of
> the argument goes through. If there's multiple problems with your approach,
> then maybe you just give up. But if this is the only problem but everything
> else checks out, then it's still worth fighting." (`tao/01`§2)

**In supermath.** *Verdict*. An attempt should be able to carry explicit
assumptions, written as `ASSUME:` lines, and continue past them. At the end of
the run the debts are counted and reported. This also gives the stuck policy in
[`03-failure.md`](03-failure.md) something better than a fixed attempt count
to act on.

**Guard.** A debt that is never discharged must never turn into a proof. A reply
with any open `ASSUME:` line cannot have its verdict read as `proved`, whatever
its final line says. This check is mechanical and belongs in
`crates/supermath-agent/src/verdict/`, next to the existing rule that a missing
verdict line reads as not established.

## 3. Gate every decomposition

Tao names this as the check machines most lack: splitting a goal into parts is
only good if each part is strictly easier than the original and still plausibly
true. The base rate is against you. A random split usually makes the problem
worse (`tao/01`§9).

> "'Oh, this looks good. The two tasks look like they're simpler tasks than
> your main task and they've still got a good chance of being true. So this is
> good to try.' Or 'No, you've made the problem worse, because each of the two
> subproblems is actually harder than your original problem,' which is
> actually what normally happens..." (`tao/01`§9)

**In supermath.** *Prompt* now, *role* later. Each proposed split must answer
two questions in writing: is every part easier, and is every part still
plausibly true? The second question routes each part through a quick
counterexample search ([`01-intake.md`](01-intake.md), section 5) before any
effort goes into proving it. When a reviewer turn exists, its job is to reject
splits.

**Guard.** If almost every split passes, the gate is not working. The share of
proposed splits that pass should be logged, and a rate near 100% is a defect to
investigate, not a sign of good taste.

## 4. Test the strategy against the obstruction list before running it

Tao calls this the strongest pruning device on his list: keep a set of false
statements near the target, and throw away any strategy that would also "prove"
one of them (`tao/01`§16). A strategy that would prove a false neighbour has a
hole in it, and it is cheaper to find the hole before the work than after.

**In supermath.** *Memory*, then *tool*. The obstruction list is per problem and
grows across runs: every refuted neighbour, every variant that broke during
intake ([`01-intake.md`](01-intake.md), section 4), every repaired statement
whose original was false ([`03-failure.md`](03-failure.md)). Before the solver
commits to a strategy, it reads the list and says, for each entry, why the
strategy does not apply to it. The list belongs in its own memory note, so recall
returns it whole and not as fragments mixed into old replies.

**Guard.** "It doesn't apply" with no reason is the failure. The reason has to
point at a hypothesis or a step the false neighbour lacks.

## 5. Prefer safe moves, and know which moves cannot be undone

Gowers and Ganesalingam built a prover around one rule: at each point, rank the
available moves by how likely they are to be part of the final argument, and
take the safest one (`mathematicians/03`§A1). The order was fixed before the
test problems and did not need changing. Three of their more specific rules
are worth carrying over:

- If a hypothesis is essential and can be used in only one way, use it, even
  without seeing what good it will do (`mathematicians/03`§A8).
- Prefer forward reasoning, and switch direction reluctantly. Their own
  published failure case shows what happens with too much forward reasoning:
  the program derives irrelevant true facts. They suggest switching to backward
  reasoning when many forward moves look equally relevant
  (`mathematicians/03`§A9).
- Expanding a definition is expensive because it cannot be undone. It closes off
  arguments that could have worked at the higher level
  (`mathematicians/03`§A10).

> "expanding a definition is substantially less safe: sometimes it is possible
> to reason in a high-level way without expanding, and since we do not allow
> 'de-expansion' in this program … expanding a definition is closing off the
> option of such high-level arguments." (`mathematicians/03`§A10)

**In supermath.** *Prompt*. We are not building a Gowers-style prover. But the
idea that the order of moves is a policy, and that the policy can be written
down instead of re-decided silently each turn, transfers directly. A short
ordered list of move types goes in the solver prompt: use a forced hypothesis,
apply a named technique to the goal as stated, reason forward from the data,
then reduce backward, and only then expand definitions. The run then reports
which move types it used.

**Guard.** Gowers's own condition is that the order is fixed before trying it
on problems and survives them unchanged. If we keep editing the order to
rescue particular problems, we are tuning a prompt, not following a policy.
Changes to the order should go through the same review as a code change and be
tested against the problem set in `problems/`.

## 6. Use named techniques, and report them by name

Tao keeps a list of named tricks (the 245A list) so that an attempt can report
"applied an epsilon of room" instead of an unlabelled manipulation
(`tao/01`§15). Named attempts can be searched, counted and reused. Unnamed ones
cannot. The study supplies a starter set of techniques, each with a trigger:

| Technique | Trigger | Source |
| --- | --- | --- |
| Structure versus randomness | a quantity resists direct estimation; split it into a structured part and a part that is negligible | `tao/01`§11 |
| Amplification and the tensor power trick | a bound is off by a constant or a log; find a symmetry under which one side is invariant | `tao/01`§13 |
| No self-defeating object | the goal says no object of some kind exists; build one, feed it to itself, look for the contradiction | `tao/01`§14 |
| Counting bound instead of construction | the goal is existence and search is hopeless; bound the expected number of failures below one | `mathematicians/02`§B2 |
| A monotone quantity | the object evolves under a process; find something that only moves one way before trying the target | `mathematicians/10`§B1 |
| Auxiliary parameter on the bad case | the argument works except on a described subclass; rerun it at a second parameter that covers that subclass | `mathematicians/04`§B1 |

**In supermath.** *Prompt* now, *memory* later. The table goes into the solver
prompt as a menu with triggers, and the reply names the techniques it tried.
Once runs record technique names, memory can answer which techniques have been
tried on a problem and how each one failed. Across problems, that record is the
start of the technique library in [`05-memory.md`](05-memory.md).

**Guard.** Each technique has its own check, and the source notes state them.
Two matter most. A counting bound proves existence without exhibiting the
object, and the run must report the result as non-constructive. A monotone
quantity has to be monotone without the hypotheses that made the current case
easy, or it is a property of the case and not a handle on the process.

## 7. Carry two formulations

Arnold keeps both the concrete and the abstract definition of a manifold,
along with the theorem that says they agree, and picks whichever suits the
task: the concrete one for computation and counterexamples, the abstract one
for generality (`mathematicians/09`§B2). Tao does the same with quantitative
and qualitative versions of a goal: the explicit version is what a computation
can test, the limiting version is often shorter to prove (`tao/01`§12).

**In supermath.** *Prompt*. During intake, when a statement admits a concrete
and an abstract form, the solver writes both down and says which it will use
for what. The concrete one is what the integer tools and any later sandbox can
check.

**Guard.** The two forms have to be equivalent by a theorem the run can cite.
Without that, switching between them is a category error and not a choice.

## 8. Drop the hypothesis nothing uses

Grothendieck's move with schemes: if no step of the theory below a definition
uses one of its hypotheses, delete that hypothesis and accept the odd new
objects that appear (`mathematicians/01`§B2). The cross-cutting study ranks
this the cheapest of its proposals, because three subjects arrive at it
independently. Scholze reports that being forced to state which properties an
argument actually uses is what produced the simplification during the Liquid
Tensor work (`mathematicians/08`§A7), and Gowers ranks moves by what they close
off (`mathematicians/12`, "The three cheapest things").

**In supermath.** *Role*. After a `proved` verdict, a reviewer turn reads the
proof and lists which hypotheses of the statement each step uses. A hypothesis
that no step uses is either redundant (the run proved something stronger,
worth recording) or a sign the proof skipped something (worth checking). Either
finding goes to memory.

**Guard.** Deleting the hypothesis must keep the objects closed under the
operations the argument needs. If deleting it breaks that, the hypothesis was
doing work that the proof does not show, which is exactly the second case above.

## 9. Change the setting, but only when the new one earns it

This is the other side of the study. Grothendieck refused to attack a problem
where it stood, and instead enlarged the theory around it until the theorem
fell out (`mathematicians/01`§A1, §A4). Scholze does the same thing
foundationally, replacing the ambient category (`mathematicians/08`§A1). Four
subjects independently name the absence of any arm that restates the goal in
a different setting as the gap in the earlier runtime (`mathematicians/12`,
Split 1).

We are not adopting the rising sea as a general method. It is not affordable
inside a run: Grothendieck's own example took sixteen years and someone else
finished it. What we adopt is Scholze's firing rule, which makes a change of
setting checkable.

> "I want to make the strong claim that in the foundations of mathematics, one
> should replace topological spaces with condensed sets...this claim is only
> tenable if condensed sets can also serve their purpose within real
> functional analysis." (`mathematicians/08`§A1)

**In supermath.** *Prompt*, with the rule attached. The solver may propose
restating the problem in another setting. The proposal counts only if it also
recovers a result the run has already established in the old setting. That
condition makes the proposal testable within the same run, which is the only
form of this move we can afford.

**Guard.** The new setting must reproduce the known cases, and the author's
confidence in it is not evidence. Grothendieck's general rule that the
generalisation must be shorter or more uniform than the old proof, and not merely
broader (`mathematicians/01`§B4), applies as well.

## 10. Build the bridge an analogy is missing

When two settings are known to be analogous and nothing explains why, Scholze
builds the object in which the analogy becomes a theorem, and then transports
results across (`mathematicians/08`§B1). It is a narrower form of section 9,
and the one most likely to arise inside a problem rather than across a career.

**In supermath.** *Prompt*. If intake finds that the problem resembles a solved
problem in another area, the solver should say what would make the
resemblance exact, and whether that is cheaper than solving the problem
directly. Usually it is not, and saying so is a fine outcome.

**Guard.** The bridge must recover known results in both settings. An analogy
that only works in the direction the run needs is not yet a bridge.
