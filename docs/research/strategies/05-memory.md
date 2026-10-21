# Memory: what a run leaves behind

The third agreement across the study is that the artifact worth optimising is
the one a later worker can pick up (`mathematicians/12`, item 3). For
supermath the later worker is usually the next run on the same problem, and the
channel between them is CortexDB. The harness already has the two rules that
matter most here. The memory scope is built from configuration and the problem
slug, never from model output. And a write counts as stored only when CortexDB
reports it indexed.

What memory holds today is one kind of note: the verdict and a truncated copy
of the reply. The strategies below are about writing several kinds of note,
each with a different standing, so that recall can return the right thing and
cannot confuse a lead with a result.

## 1. Keep three tiers apart

Tao's collaborative projects separate the working thread (high volume,
disposable), the control channel (summaries and steering), and the settled
store (curated, promoted only after checking). Long derivations go to the
workspace and are referred to by a one-line summary (`tao/01`§33).

**In supermath.** *Memory*. The run's transcript is the working thread and
stays in OpenHuman's workspace. It is not memory. The settled store is the set
of notes a run writes after its verdict, and the spec already requires that
writing happens only once a run has said what it established. What is missing
is the split inside that store. We propose these note kinds, all inside the
problem's existing scope:

| Kind | Holds | Standing on recall |
| --- | --- | --- |
| `result` | a verdict with its obligations and non-claims ([`04-trust.md`](04-trust.md)) | as strong as its verdict and obligations |
| `intake` | goal type, difficulty list, load-bearing hypotheses ([`01-intake.md`](01-intake.md)) | a description of the problem, to be corrected |
| `dead-end` | failed approaches with coordinates ([`03-failure.md`](03-failure.md)) | true about past attempts, not about the problem |
| `bound` | searched ranges with no counterexample | true up to its stated bound |
| `lead` | repaired conjectures, half-formed ideas, unbounded patterns | unverified, never cited as established |

The turn message then presents each kind in its own section, so the solver
reads "these approaches failed" separately from "this was established".

**Guard.** A note's kind is set by the harness from the structured part of the
reply, never by the solver choosing a label. A `lead` must not become a
`result` because a later run quotes it confidently.

## 2. Write in small units

Polymath's house rule caps each contribution at one claim, one obstruction or
one improved bound. A contribution is written when another participant could
act on it now, not when the author has finished (`tao/01`§31, with the source
caveat that the rules are unsigned Polymath doctrine Tao enforced rather than
wrote).

**In supermath.** *Memory*. One note per finding, not one note per run. The
current 6,000-character note holds a whole reply, so recall returns large
blocks in which the useful sentence may be anywhere. Splitting the write into
one note per entry in the structured block (one per dead end, one per bound,
one per obligation) makes recall return the finding itself.

**Guard.** More, smaller notes make duplicates more likely. A second run that
finds the same dead end should update the existing note, and say it found it
again, rather than add a copy.

## 3. Share the half-formed idea, behind a gate

Gowers designed Polymath around "just give quick reactions", because a
contribution that has to be well-formed is a contribution that never gets
posted (`mathematicians/03`§A11, §B2). The cross-cutting study points out that
the earlier runtime had deliberately put scratch notes out of reach of durable
recall, so no role could see another's provisional thought at all, and that the
fix is to share scratch between roles, not to relax what counts as a claim.

**In supermath.** *Memory*. The `lead` kind is this channel. Today a run has
one role, so leads matter between runs: an idea a run had and did not pursue
should reach the next run. When the harness gains separate roles (refuter,
reviewer, solver), leads are how they talk.

**Guard.** Nothing that derives a `result` may read a `lead` as support. A lead
can suggest what to try. It cannot be cited.

## 4. Separate generating conjectures from proving them

Ramanujan's notebooks record statements and almost no derivations. Much of
their value came from others proving them later, and the source notes read his
method as volume with selection: generate cheaply, record the statement and
the evidence, and leave justification to a separate process
(`mathematicians/07`§A1, §A2, §B1, all `[INFERRED]`). Erdős worked the same
way at the level of problems, generating the neighbourhood of each solved one
(`mathematicians/02`§A5, `[INFERRED]`).

> The move as the source states it: "separate generation from justification,
> and resource them differently." (`mathematicians/07`§B1)

**In supermath.** *Memory*, later *role*. Conjectures a run forms along the way
(from patterns, from repaired refutations, from variants in
[`02-attack.md`](02-attack.md)) are stored as `lead` notes tagged with what
produced them and what evidence supports them. A later run, or a dedicated
generator role, can then be pointed at them.

**Guard.** The source notes give three mandatory checks, and all three carry
over. The store must be unreachable from anything that derives an established
result. The rate at which each generator's conjectures are later proved must be
recorded from the start, or nobody can tell a good generator from a noisy one.
And something must actually attack the store, or it is a landfill.

## 5. Build a technique library across problems

Thurston's correction after foliations was to publish the infrastructure ahead
of the theorem, written in terms that do not mention the problem that
motivated it, and judged by whether someone else can use it
(`mathematicians/05`§B2). The source notes identify a shared technique library
as the highest-value item the earlier runtime never built, and four
subjects arrive at it independently (`mathematicians/12`, Split 5).

**In supermath.** *Memory*, with a spec of its own. This is the one strategy in
these files that needs memory outside a single problem's scope, so it touches
an invariant. A library scope (for example `{CORTEX_SCOPE_ROOT}/library`) would
be written only by the harness, only from a reviewed result, and only with the
problem's name removed from the entry. The scope is still built from
configuration, never from model output, so the existing guarantee holds.

**Guard.** Thurston's test applies literally. An entry is admitted only if it
is usable by a run that does not know the original goal. A reviewer turn that
sees only the entry, and not the problem it came from, can check that.

## 6. Make the problem set a catalogue with status

Erdős priced his problems and distributed them as a catalogue others could
work from, and the prices worked as a priority signal (`mathematicians/02`§A3,
§A6). His curator's warning belongs on any such list: "Do not assume that an
'unsolved' problem is in fact unsolved" (`mathematicians/02`§A6).

**In supermath.** *Problem files*. `problems/` is already a catalogue of sorts:
one statement per file, with the slug naming its memory. A small header on each
problem (status, source, whether it is a known result, the best verdict any run
has reached, and a link to that run's notes) would make it the run's starting
point and the place a human looks first. The smoke-test problems are a good
model: `sum-of-odd-numbers.md` is labelled as a statement every capable model
should prove, so a failure points at the harness.

**Guard.** Status in a problem file is written by people or by the harness from
a stored result, never copied from a reply.

## 7. Track one number that only improves

For problems whose progress is a bound, Tao's factory-line advice is to cache
the best known value of each intermediate quantity in a shared store, let
anyone improve it, log each improvement with its cause, and watch for the
point where improvements shrink toward nothing. At that point the search space
should change, not the effort (`tao/01`§35).

> "A run posting 0.3% improvements is stalled, not progress." (`tao/01`§35,
> the source notes' own summary)

**In supermath.** *Memory*. The `bound` note kind holds this. A run that
improves a bound writes the new value, the old value, and what produced the
change. A run that does not improve it reads the history first and sees
whether recent gains have been shrinking.

**Guard.** Only a checked improvement updates the number. Proofatlas's own
blind spot is that its counts only ever grow and nothing shrinks when a
result is refuted (`proofatlas/00`, "What converged", 5). A refuted
improvement must roll the value back and say so.

## 8. Keep a calibration bench

Tao recommends reproving known results with a restricted toolkit as a
self-calibration exercise, and splitting the budget across difficulty tiers
rather than spending it all on the headline problem (`tao/01`§29). Proofatlas
does the same thing at scale, with a bench of known theorems whose formal
versions are compared with the familiar statements
(`proofatlas/04-known-theorem-bench.md`).

**In supermath.** *Problem files* and CI. `problems/` already has two
calibration problems. The bench should grow by tier (things every model should
prove, things that need a tool, things that need a correct refutation, things
whose honest answer is `not-established`) and be run whenever the solver
prompt or the verdict format changes. A prompt change that improves one
problem and breaks two others is visible only if the bench exists.

**Guard.** The bench must include problems whose correct answer is `refuted`
and problems whose correct answer is `not-established`. A bench of provable
statements only rewards a solver that says `proved` more often.
