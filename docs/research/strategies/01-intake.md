# Intake: what a run does before it attacks

Every subject in the study does work on a problem before attacking it, and
none of them starts by attempting the full statement. The current solver
prompt asks for some of this ("restate the problem precisely", "try small
cases and special cases first"), but it asks for it in one breath with the
proof attempt, so nothing distinguishes a run that did the reconnaissance from
one that skipped it.

The strategies here share one adoption idea: intake produces an artifact. A
run that has done intake can show what it found, in a form the harness can
read and memory can store, before any proof attempt starts.

## 1. Classify the goal before choosing a method

Tao sorts problems into three kinds by their shape: "show that" or "evaluate",
"find a" or "find all", and "is there a". The kind decides the method. A
"show that" problem has a visible target and can be approached directly. A
"find all" problem needs a guess and then a proof that nothing was missed. An
"is there" problem has two live answers and needs both a construction attempt
and a refutation attempt.

> "'Show that . . .' or 'Evaluate . . .' problems start with given data and the
> objective is to deduce some statement or find the value of an expression;
> this type of problem is generally easier than the other two types because
> there is a clearly visible objective, one that can be deliberately
> approached." (`tao/01`§4, from *Solving Mathematical Problems*)

**In supermath.** *Prompt* first: ask the solver to state the goal type on its
own line before anything else. *Verdict* next: make the goal type part of the
reply the harness parses, because it changes what a verdict means. A `proved`
on a "find all" problem is only honest if the run also proved exhaustiveness,
and the harness cannot check that without knowing the problem was a "find
all". Problem files could eventually declare the type themselves, so the run
can be checked against the author's reading rather than the model's.

**Guard.** Misclassification is the failure, and it is quiet: "find the
largest prime factor of N" read as "evaluate" lets a run skip proving the
factor is prime. The goal type should be recorded with the run, so a wrong
classification shows up in memory rather than disappearing into the reply.

## 2. Name the difficulties

This is the first of the four agreements across all eleven subjects
(`mathematicians/12`, "Where all eleven agree"). Tao lists what makes the
problem hard so that he can switch most of it off (`tao/01`§1). Scholze states
the obstructions by name, and then changes the setting so they go away
(`mathematicians/08`§A4). Gowers's move generator is a mechanised version of
the same list (`mathematicians/03`§A1). Arnold's research cycle starts with
observation of special cases, which is how the difficulties get found
(`mathematicians/09`§A2).

**In supermath.** *Prompt*: ask for a numbered list of difficulties during
intake, each phrased as something specific ("the error term is not uniform in
k", "the bound fails for n below 100"), not as a feeling ("the problem is
hard"). *Tool*: the list is the key that strategies in
[`02-attack.md`](02-attack.md) and [`03-failure.md`](03-failure.md) refer to.
Cheating strategically switches difficulties off by name, and a failed attempt
records which difficulty stopped it. So the list should be written to memory as
its own note, not buried in the run's reply, and a later run on the same
problem should start from it.

**Guard.** A list that never changes is not doing anything. When an attempt
fails on something that is not on the list, the list was wrong, and the run
should add the new difficulty before trying again.

## 3. Compute first, as reconnaissance, and bound every pattern

For a problem with a computable side, Tao builds the naive check and runs it
before theory: small-case enumeration, plots, pattern hunting
(`tao/01`§20). Arnold goes further and treats computation as the only reliable
way to keep a long argument honest (`mathematicians/09`§A1, §A4). But the
cross-cutting study is careful here. Five of the ten other subjects' central
results involved no computation at any stage (Grothendieck, Wiles on Fermat,
Thurston on Haken manifolds, Scholze on liquid tensors, Perelman), so "compute
first" is a default for problems that have something to compute, not a law
(`mathematicians/12`, Split 4).

The guard comes from Arnold. His example sequence `1, 2, 4, 8, 16, 29` is
consistent with doubling for five terms, and the sixth is where the candidate
closed forms part company (`mathematicians/09`§B1). Gowers states the general
rule: "a sufficiently simple general statement that is not obviously true is
almost certainly false" (`mathematicians/03`§A7).

**In supermath.** *Prompt* now, *tool* soon. The deferred integer tools are the
start of this. When the toolkit grows sequence tools, a pattern found by them
should carry an obligation: before the run states the pattern as a conjecture
or uses it to steer, it extends the computation past where the pattern was
noticed, into a regime that is genuinely new (larger magnitude, a different
residue class, a different parity), and reports that it did. "Checked for
n ≤ 20, pattern noticed at n ≤ 6" is the kind of line a run should be able to
write.

**Guard.** More of the same is not an extension. Checking 1,000 more even
values of n says nothing about odd n. The extension must name what is new
about it.

## 4. Break the statement on purpose

Tao's aggressive move: remove a hypothesis, swap data and objective, or negate
the goal, and see where the problem breaks. The breakdown shows which
hypotheses carry the weight (`tao/01`§6). His smaller version applies the same
probe to every lemma a run depends on: drop each hypothesis, try the converse,
test a degenerate parameter (`tao/01`§7).

> "Basically, we try to push the problem until it breaks, and then try to
> identify where the breakdown occurred; this identifies what the key
> components of the data are, as well as where the main difficulty will lie."
> (`tao/01`§6)

**In supermath.** *Prompt*: during intake, for each hypothesis in the
statement, ask whether the conclusion survives without it, and say whether
that was checked by an example or argued. The hypotheses whose removal breaks
the statement are the ones the eventual proof has to use. Write them to memory
next to the difficulty list. *Verdict*: a `proved` verdict whose proof never
uses one of those load-bearing hypotheses is suspicious, and a later reviewer
role (see [`04-trust.md`](04-trust.md)) can check exactly that.

**Guard.** A hypothesis can look idle on small cases and be essential in the
limit. Record whether a "this hypothesis is not needed" finding came from an
example or from an argument, and treat the example kind as a lead.

## 5. Look for the counterexample first

Counterexample search is usually cheaper than proof, and either outcome helps.
Tao's version is to run both at once on one budget and let whichever finishes
first end the other (`tao/01`§10). Gowers's rule that simple, non-obvious
general statements are usually false (`mathematicians/03`§A7) makes this a gate
before proving rather than a check afterwards. The cross-cutting study points
out that the earlier runtime had it backwards: its refuter ran on a schedule
after the run had committed to a direction.

> "If you are trying to prove some ambitious claim, you might try to first look
> for a counterexample; either you find one, which saves you a lot of time and
> may well be publishable in its own right, or else you encounter some
> obstruction." (`tao/01`§10)

**In supermath.** The current prompt says to look for a counterexample "as
seriously as for a proof". That is the right instruction in the unenforced
form. The next step is *role*: a separate refutation turn, with its own
context, run before or alongside the proof turn, whose result is fed to the
proof turn. Two turns cost more than one, but they cannot share a bias the way
one turn arguing both sides does. Whatever the refutation turn searched and
failed to find should be stored as a bound ("no counterexample with n below
10^6"), so the next run starts above it.

**Guard.** A refutation search that comes back empty is not evidence of truth
beyond the bound it searched. The stored note must carry the bound, and the
verdict parser already prevents an empty search from becoming `refuted`.

## 6. Ask whether the problem belongs to a decidable class

Zeilberger's move: before asking whether this instance is provable, ask
whether the whole family it belongs to is decidable. If it is, run the
decision procedure and keep the certificate (`mathematicians/06`§B1). The WZ
method for hypergeometric identities is the standard case, and the check that
makes it honest is that family membership is syntactic ("proper hypergeometric
term"), not a judgement call.

**In supermath.** *Tool*. This is where supermath's own contract crate
eventually earns its place. The Jacobian notes
(`../rea-and-jacobian/principles.md`) describe operations admitted only when
they return a checkable answer for a stated class of input. Intake should ask
whether the problem falls in a class one of those operations decides, and if
so, the answer comes from the operation, not the model. Until those operations
exist this is a prompt line: "say whether this problem is an instance of a
class with a known decision procedure, and name it".

**Guard.** A guess about class membership is not membership. Only route to a
decider when the membership test itself is mechanical. Where the decider is too
expensive, fall back to random specialisation (see
[`04-trust.md`](04-trust.md)) and say so.

## 7. Ask whether someone has already named the gap

Perelman did not take on the Poincaré conjecture cold. He took Hamilton's
programme, whose remaining gap Hamilton had stated in print, and supplied the
missing piece, adopting the machinery wholesale rather than re-deriving it
(`mathematicians/10`§A1, §B1). Wiles similarly worked inside a stalled
programme. The cross-cutting study calls this the closest thing in the set to
unanimity (`mathematicians/12`, Split 5). Erdős adds a warning that belongs
next to it: "Do not assume that an 'unsolved' problem is in fact unsolved"
(`mathematicians/02`§A6).

**In supermath.** *Prompt* and *memory*. Intake asks two questions: is this
problem a special case or a step of a known programme, and if so, what does
that programme say is missing? The answer, with its source, goes to memory.
Once supermath has a shared library across problems (see
[`05-memory.md`](05-memory.md)), it is also where the run looks first.

**Guard.** Imported machinery has the standing of its source, not of the run.
Perelman did not audit Ricci flow. A run that does the same must mark the
imported part as catalogued, not established, and its verdict inherits that
mark.

## 8. Decide whether the problem is in reach

Tao picks problems in the band where known techniques "can do 90% of the job"
(`tao/01`§8). His matching rule for walking away, on the Riemann hypothesis,
is that even with every cheat turned on there is no path. He also says when a
large run is the wrong tool: with no partial route to adapt and no legible
measure of progress, the honest goal is to explain why the known methods fail
(`tao/01`§36). Before each expensive attempt, he projects forward and asks
whether success would actually close the goal (`tao/01`§28).

> "What's really interesting are the problems just on the boundary between
> what we can do rather easily and what are hopeless, but what are problems
> where existing techniques can do 90% of the job and then you just need that
> remaining 10%." (`tao/01`§8)

**In supermath.** *Verdict*. `not-established` currently covers both "ran out
of budget on a tractable problem" and "this is out of reach with known
methods". Those are different findings and memory should keep them apart. We
want a reason on `not-established`, something like `budget`, `stuck` or
`out-of-reach`, and when the reason is `out-of-reach`, the deliverable is the
explanation of why the known methods fail. Tao counts that explanation as a
result, and so should we.

**Guard.** "Out of reach" is a cheap thing to say and an expensive thing to
show. It should require the fully cheated version of the problem (every
difficulty from step 2 switched off but one) to have been attempted and to
have failed, with the failure recorded.
