# Settings: where the mathematicians disagree, and which side we take

The cross-cutting study (`mathematicians/12`) is built on one rule. Where all
eleven subjects agree, a solver has a requirement. Where they split, a solver
has a setting, and it has chosen a value for that setting whether or not anyone
wrote the choice down. The study's main finding about the earlier runtime was
that it had picked a side on all five splits by default, and in one case the
choice was made by two constants in a configuration file.

This file writes our choices down. Each section states the split, the side
supermath takes for now, the reason, and what would make us revisit it. The
choices are proposals until a spec adopts them, like everything else in this
folder, but a spec that touches any of these should say which side it is on.

## Split 1: attack the problem, or enlarge the setting

| Position | Subjects |
| --- | --- |
| Attack the problem where it stands | Tao, Erdős, Gowers, Wiles, Zeilberger |
| Enlarge the surroundings until the problem dissolves | Grothendieck, Scholze, Arnold (partly), Perelman (inherits one) |

Grothendieck states the two styles himself, as the hammer and chisel against
the rising sea, and does not rank them (`mathematicians/01`§A1).

**Our side: attack where it stands, with one door left open.** A run is one
turn today and a handful of turns soon. The rising sea needs years. The study
is blunt that the earlier runtime's attempt limits ruled it out without anyone
deciding to, and we would rather decide it on purpose. What we keep from the
other side is Scholze's firing rule: a change of setting is allowed when it
also recovers a result the run has already established in the old setting
([`02-attack.md`](02-attack.md), section 9). We also keep the inherited
hypothesis check (section 8 of the same file), which three subjects from both
sides arrive at.

**Revisit when** supermath runs campaigns across many runs on one problem,
with a technique library ([`05-memory.md`](05-memory.md), section 5) that can
hold machinery built ahead of a theorem. That is the setting in which Thurston
and Grothendieck's way of working becomes affordable.

## Split 2: what counts as a result

| Position | Subjects |
| --- | --- |
| The formal kernel, or nothing | Tao, Scholze |
| The formal model is not the thing | Arnold, Thurston |
| Certainty has a price, and the price goes on the label | Zeilberger |
| The statement, without the proof | Ramanujan, Grothendieck |

The study calls this the deepest split in the set, and the four positions do
not sit on a line. Arnold thinks long formal chains are less reliable, not more
(`mathematicians/09`§A3). Thurston refuses any weakening of the standard of
proof but wants proofs a reader can check (`mathematicians/05`§A4). Scholze
reports that formalising the Liquid Tensor theorem taught him why his own proof
worked, after a year of trying (`mathematicians/08`§A6).

**Our side: priced claims, with the price visible.** We have no formal checker
and the spec says a `proved` verdict is the model's claim. Of the four
positions, Zeilberger's is the only one that is honest about that without
giving up: a result is worth what it rests on, and the reader should be able
to see what it rests on. That is the whole of [`04-trust.md`](04-trust.md):
obligations, specialisation, non-claims, collected evidence, statement
alignment.

Grothendieck's position gets a narrow, explicit form and no more. His habit of
declining to write a proof once the statement was understood
(`mathematicians/01`§A5) is backed by a record no run has, and an LLM saying
"the rest is routine" is the exact failure the verdict parser exists to stop.
What survives is the distinction the source notes draw: "asserted because
nobody checked" is different from "asserted because the remaining step is
named and mechanical". The second is an `OBLIGATION:` line with the procedure
named, not a `proved`.

**Revisit when** a formal checker exists. At that point Scholze's targeting rule
decides what it checks ([`04-trust.md`](04-trust.md), section 5), and his
experience argues for recording what the checker explains about a proof, not
only whether it passed.

## Split 3: one mind, or many

| Position | Subjects |
| --- | --- |
| Undivided concentration, and tell nobody | Wiles, Perelman |
| Make each contribution as small as possible | Erdős, Gowers, Tao |
| Publish the infrastructure so others can work | Thurston, Grothendieck |

Wiles bought his concentration with secrecy (`mathematicians/04`§A2). Gowers
designed the opposite, aiming for a solution in which no single participant
had to think very hard (`mathematicians/03`§A11). The study's most useful
observation is that both solitary cases needed others at the end. Wiles could
not close his gap without Taylor, and Perelman's proof took three teams and
five years to verify (`mathematicians/10`§A3).

**Our side: one solver, with separate turns only where independence is the
point.** The current harness is a single solver turn, which is the Wiles shape.
We add other turns only when they buy something the solver cannot provide for
itself, and the strategies in these files name three such cases: a refutation
search that does not share the solver's bias ([`01-intake.md`](01-intake.md),
section 5), a statement-alignment check that does not see the proof
([`04-trust.md`](04-trust.md), section 7), and a reader that did not write the
derivation (the same file, section 10). Each extra turn has a cost, and Wiles is
the argument for measuring it rather than assuming that more roles are better.
The earlier runtime grew to twenty-two roles, and its own notes say that four of
its five standing teams duplicated a loop arm without anyone choosing that.

A separate reviewer turn is still a model reviewing a model. Proofatlas records
`independenceStatus: not_asserted` on every one of its review lanes for exactly
this reason (`proofatlas/00`, "What was deliberately not proposed"). Our
reviewer turns check legibility and alignment, which a fresh context is good
at. They are not evidence that a proof is correct, and the verdict must not be
presented as if they were.

**Revisit when** runs have several turns and we can measure, on the bench in
[`05-memory.md`](05-memory.md), section 8, whether each added turn changes
outcomes.

## Split 4: where computation belongs

| Position | Subjects |
| --- | --- |
| Compute continuously, as the only reliable control | Arnold |
| Compute to discover, with no goal in hand | Ramanujan, Zeilberger, Erdős |
| Compute first, as reconnaissance on the goal | Tao |
| Deliberately refuse the machine's speed | Gowers |
| Never computed at all | Grothendieck, Wiles, Thurston, Scholze, Perelman |

The last row is the surprise. Five of the ten non-Tao subjects' central results
involved no computation at any stage, and the Tao study found computation
decisive in one of eleven of his programmes (`mathematicians/12`, Split 4).
Gowers goes further and deliberately withholds the machine's speed, so that his
prover is forced to develop technique (`mathematicians/03`§A3).

**Our side: compute when the problem has something to compute, and say which
kind of computation it is.** For the problems supermath has now, which have
numerical content, Tao's reconnaissance and Arnold's control are the right
default and the toolkit should grow in that direction. Two qualifications
follow from the split. First, the prompt should not require computation before
prose for every problem, because that would push a structural problem into
fake numerics. Second, reconnaissance on the goal and open exploration near it
are different activities, and their findings have different standing: the
first produces checks, the second produces `lead` notes
([`05-memory.md`](05-memory.md), section 4). Every pattern found either way is
bounded before it is used ([`01-intake.md`](01-intake.md), section 3).

We do not adopt Gowers's restriction. It is a research choice for building a
human-style prover, and supermath is not building one. We do take his warning:
a run that keeps scaling one computation is showing a symptom, and the stuck
policy in [`03-failure.md`](03-failure.md) should notice.

**Revisit when** the problem set includes problems without numerical content.
Then the bench will show whether the solver forces computation where it does
not belong.

## Split 5: whether a run starts cold

| Position | Subjects |
| --- | --- |
| Take a stalled programme and supply the missing piece | Perelman, Wiles |
| Publish the infrastructure so someone else can | Thurston, Grothendieck |
| Distribute a priced, statused catalogue | Erdős |
| Rediscovering known work is not wasted | Grothendieck |

This is not really a split. It is the closest the set comes to agreement, from
four directions.

**Our side: never start cold.** The harness already recalls per-problem memory
before every turn. The strategies in these files extend that in three steps.
Within a problem, recall returns typed notes rather than old replies
([`05-memory.md`](05-memory.md), section 1). Before a problem, intake asks
whether the problem is a named gap in someone's programme
([`01-intake.md`](01-intake.md), section 7). Across problems, a library scope
holds machinery written without reference to the problem that produced it
([`05-memory.md`](05-memory.md), section 5). Grothendieck's counterweight sets
where the novelty check goes: on the reported answer, never on the derivation
([`04-trust.md`](04-trust.md), section 9).

**Revisit when** the library exists and we can see whether runs that recall
from it do better on the bench than runs that do not.

## What we are deliberately not adopting

Some strategies in the source notes are good mathematics and wrong for this
harness, at least for now. Writing them down saves the next reader from
proposing them again without the reasons.

- **The rising sea as a general method.** See Split 1. It needs a timescale we
  do not have, and the part we can afford is already in
  [`02-attack.md`](02-attack.md).
- **Statements without proofs as results.** Ramanujan's notebooks were valuable
  because other people spent a century proving them (`mathematicians/07`§A5).
  A run's unproved statements are `lead` notes, and they stay leads until
  something proves them.
- **Secrecy.** Wiles hid his work to protect his concentration. A run has no
  spectators to hide from, and memory that a later run cannot read defeats
  Split 5.
- **Refusing the machine's speed.** See Split 4.
- **Review lanes as independent evidence.** See Split 3. Several model turns
  agreeing is not the same as a check.
- **Restating an open problem as a competition exercise to get the model to try
  it.** The Erdős catalogue attempts found that the model refused open research
  problems until the author recast them as competition problems, which the
  author called "gaslighting the model" (`mathematicians/02`§B1). It worked,
  and it is worth knowing about. But it builds a false statement into the
  prompt, and supermath's prompt is the one place we promise the model the
  truth about what it is doing. If our solver turns out to refuse open
  problems, that needs a spec and an argument, not a quiet prompt change.

## Where the choices go next

Each "our side" above should become a sentence in the spec that implements the
strategies that depend on it. The cheapest of those, and the ones with the most
support in the source notes, are the verdict extensions: open debts
(`ASSUME:`), obligations (`OBLIGATION:`), non-claims (`DOES-NOT-COVER:`) and a
reason on `not-established`. They change the reply format, which is cheapest
to change before anything consumes it, and they give every other strategy in
these files something to read and store.
