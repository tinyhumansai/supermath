# Trust: what a run must show before anyone believes it

The harness already gets the hardest part of this right. A run ends in one of
three verdicts, a missing or unknown verdict reads as not established, a run
that ran out of budget is never reported as a refutation, and the spec says
plainly that a `proved` verdict is the model's claim (see
[`../../specs/solving-harness.md`](../../specs/solving-harness.md),
"Non-goals"). Both the mathematician studies and the Jacobian notes
(`../rea-and-jacobian/implications-for-supermath.md`, "Three-valued results
from day one") recommend exactly this.

What the verdict does not yet say is how much a `proved` is worth. The
subjects disagree sharply about what counts as a result (see
[`06-settings.md`](06-settings.md), Split 2), but every position in that
disagreement accepts the strategies in this file. They are about making the
basis of a verdict visible, so that the reader can decide how far to trust it.

## 1. Put the price on the label

Zeilberger proposed that theorems carry a price, and that prices add up along
a deduction: if A costs p and B costs q, a C deduced from both costs p + q
(`mathematicians/06`§A1). His companion move separates "we know how to prove
this" from "we have proved this": when the procedure that would settle a step
is too expensive to run, record the obligation, the procedure, and the cost,
and carry on (`mathematicians/06`§B2).

> "It would be then OK to rely on such a priced theorem, provided that the
> price is stated explicitly." (`mathematicians/06`§A1)

**In supermath.** *Verdict*. The cheapest honest price is the list of things
the argument rests on that the run did not establish. A `proved` reply should
list its outstanding obligations (imported results, steps checked only
numerically, cited lemmas it did not prove), each on an `OBLIGATION:` line.
The harness stores them with the verdict. A `proved` with no obligations and a
`proved` that rests on three unchecked lemmas are different results, and
memory should not flatten them into one.

**Guard.** Zeilberger's own condition is the important part: everything that
depends on a priced statement inherits the price. When a later run recalls a
result with open obligations and builds on it, its own verdict carries those
obligations too. Without that rule, this is ordinary sloppiness with a label.

## 2. Check by random specialisation, and call it that

Zeilberger's cost trick: a symbolic identity that would be expensive to prove
can be checked at many specific parameter values, and solvability at enough of
them is very strong evidence. He also says when to pay full price.

> "It is a waste of money to get absolute certainty, unless the conjectured
> identity in question is known to imply the Riemann Hypothesis."
> (`mathematicians/06`§A2)

The source notes point out that the earlier runtime had every tool needed for
this and no word for the result. An obligation checked at forty random points
is neither unsupported nor proved, and there was no third status to write
down.

**In supermath.** *Verdict* and *tool*. An obligation discharged this way is
recorded as discharged by specialisation, with the number of points and how
they were chosen, not as proved. Once supermath has a sandboxed evaluator or
its own contract operations, the specialisation runs there and the harness
records the result itself (see section 8).

**Guard.** The points must be chosen independently of the argument. Values the
solver picked because they were convenient are examples, not specialisation.

## 3. Control the argument with examples all the way through

Arnold and Zeilberger disagree about the value of rigour more than any other
pair in the study, and they prescribe the same mechanism: check the argument
against concrete instances, continuously (`mathematicians/12`, Split 2, "The
unnoticed agreement"). Arnold's warning is that without that control, "after
some ten pages half of all the signs in formulae will be wrong"
(`mathematicians/09`§A4). The earlier runtime did this once, at the end.

**In supermath.** *Prompt* now, *tool* later. Every intermediate claim that can
be evaluated at a small case should be, at the point where it is made, and the
reply should show the check beside the claim. The deferred integer tools cover
some of this today. The harness can later count claims with and without an
attached check, which gives a crude but real measure of how much of an argument
was controlled.

**Guard.** A check at the case that inspired the claim is not a control. The
example must be one the claim was not built from.

## 4. Treat long chains as less reliable

Arnold's view is the reverse of the usual one: "The longer and fancier is the
chain of deductions ('proofs'), the less reliable is the final result"
(`mathematicians/09`§A3). Thurston, who rejects any weakening of the standard
of proof, makes a related point: formal definitions can hide the differences
between ways of understanding the same thing, and proofs should be written so a
reader can find the weak step (`mathematicians/05`§A3, §A4).

**In supermath.** *Verdict*. Record the derivation depth of a `proved` result,
meaning the longest chain of steps from the hypotheses to the conclusion, as
the solver reports it. It costs one field and it lets a reader see which proofs
are long enough to deserve a second look.

**Guard.** Depth is a reason to look harder, not a verdict. A long proof is not
wrong for being long.

## 5. Spend scrutiny where the most rests

The fourth agreement across all eleven subjects: scrutiny should scale with how
much depends on a claim, not with how surprising the claim is
(`mathematicians/12`, item 4). Scholze formalised the Liquid Tensor theorem
because it would "be used as a black box, a mistake in this proof could remain
uncaught", and he volunteers that a wrong proof of his once convinced top
mathematicians (`mathematicians/08`§A2). The earlier runtime had a formal
checker and no rule for what to check.

**In supermath.** *Role*, and later *tool*. We do not have formal checking, and
the spec rules it out for now. The rule still applies to the reviewer turn
proposed throughout these files: the results that get reviewed first are the
ones other notes in memory cite. Within one run, the lemma the most later steps
use gets the most careful check. When a Lean checker arrives (an open question
in the spec, and the Jacobian notes advise keeping it as a separate module),
the same rule decides what it is pointed at.

**Guard.** A surprising result still deserves a look. The rule sets priority,
and it should not be used as a reason to leave a strange claim unexamined.

## 6. Say what the result does not cover

The strongest convergence in the proofatlas notes is that a claim must state
its limits, and must name the specific neighbouring statements it could be
confused with, not a generic disclaimer (`proofatlas/00`, "What converged", 1).
On their natural-density result, five of six non-claims are about confusion
with sibling results in the same package. A claim that reads as more complete
than it is causes exactly the failure our three-valued verdict exists to
prevent, only one level down.

**In supermath.** *Verdict*. A `proved` or `refuted` reply lists what it does
not cover on `DOES-NOT-COVER:` lines: the converse, the boundary case, the
stronger form a reader might assume. These lines go to memory with the result,
and recall returns them together.

**Guard.** Boundary length should track how easy the result is to misread. A
list of every conceivable non-claim is as useless as an empty one.

## 7. Check that the statement proved is the statement posed

Proofatlas reviews statement alignment as its own gate, and it is one of the
two reasons it refuses to accept its own largest proof, Sendov, despite a
1,176-file Lean project with no `sorry` and no extra axioms: nobody has
certified that the 20-line theorem file states Sendov's conjecture
(`proofatlas/00`, "The one thing to read"). The gate has to be judged by
someone other than the author. The Erdős catalogue attempts show why: on
problem 728, both the model and its human operator misread the direction of a
quantifier in the statement, and the error was caught only by checking the
final Lean statement against the intended one (`mathematicians/02`§B1).

**In supermath.** *Role*. We have no Lean, but we have the same gap. The
solver's first step is to restate the problem in its own words, and every step
after that works from the restatement. A short reviewer turn that sees only
the problem file and the restatement, and answers whether they say the same
thing, catches the case where a run proves a nearby easier statement and
reports it as the original. It is cheap, and it is the check most likely to
catch the error an honest solver makes.

**Guard.** The reviewer must not see the proof. Once it does, it starts judging
whether the proof is good, and the question was whether the statement is the
right one.

## 8. Evidence the harness collected is not the same as evidence the model reported

The single most transferable point in the proofatlas notes: "A transcript you
were given is a claim. A transcript you collected is evidence."
(`proofatlas/00`). Their Sendov blocker is not mathematical. The retained audit
is a summary rather than a complete build transcript collected by their own
tooling.

**In supermath.** *Tool*. When a reply says "factor_integer returned 2 × 3 × 7",
the harness knows whether that call happened and what it returned, because the
harness ran it. Today that knowledge is not recorded. Each tool call made
during a run should be logged by the harness with its arguments and result,
and the stored verdict should say which of the reply's computational claims
match a collected call. A computation the model reports and the harness did
not see is a claim, and memory should label it as one.

**Guard.** Only the executing tool may mark a result as collected. Nothing the
model writes can set that mark, for the same reason the memory scope is never
built from model output.

## 9. Check novelty on the claim, not the derivation

Erdős problem records give a measured failure rate: of three solves attempted
against the Erdős catalogue by a capable, motivated person working with an LLM,
two duplicated existing literature, and the second came after the first had
already taught the lesson (`mathematicians/02`§B1). Tao's rule is that a short
proof by standard technique raises the chance that the result is already known
(`tao/01`§19). Grothendieck supplies the counterweight: rediscovering known work
internally may be the only real understanding a run has of it, so the novelty
check belongs on the reported answer and never on the derivation
(`mathematicians/01`§A9).

**In supermath.** *Role*, when a search tool exists. Before a run reports a
result as new, a literature check looks for the statement and separately for
the technique. Its finding is attached to the result. For problems in
`problems/`, which are mostly known results used to test the harness, this is
not a concern yet. It becomes one the moment the problem set includes open
problems.

**Guard.** The novelty check can add a note and cannot change a verdict. A
novelty check that can overturn a proof is a second judge, and the source notes
are right to forbid it.

## 10. Write the result so the next reader can use it

Thurston emptied his own field once, by writing his foliation results for
readers who already shared his intuition, and then corrected it with
geometrization by writing for everyone else (`mathematicians/05`§B1, §B2). His
test for a derivation is whether the next worker can pick it up. The source
notes observe that this is one of the few checks in the whole study a runtime
can actually run.

**In supermath.** *Role*. Hand the finished derivation to a turn that did not
write it, give it the problem and the derivation and nothing else, and ask it
to state the next step, or the weakest step. If it cannot, the derivation is
written for its author. This doubles as the reviewer in sections 5 and 7, so
it is one extra turn per run, not three.

**Guard.** The reader must have a different context from the writer. A turn
that inherits the writer's full transcript will understand the derivation for
the wrong reason.
