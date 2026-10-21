# Where the eleven disagree

The reason for studying ten at once rather than ten separately. Organised by
disagreement, not by person.

The rule this file applies: **where all eleven agree, the runtime has a
requirement; where they split, the runtime has a setting it did not know it had
chosen.** The second is the more valuable finding, because this repository's
recurring failure is a decision made by default, recorded nowhere, and defended
by a prompt.

Tao is the eleventh. See [`../tao/`](../tao/).

---

## The five splits

### Split 1 — Trick against framework

| Position | Subjects |
|---|---|
| Attack the problem where it stands | Tao, Erdős, Gowers, Wiles, Zeilberger |
| Enlarge the surroundings until the problem dissolves | Grothendieck, Scholze, Arnold (partly), Perelman (inherits one) |

Grothendieck states it as two named styles and does not rank them: the hammer
and chisel against the rising sea, with Serre named as the other pole and called
"the incarnation of elegance" (`01`§A1). Tao's §1 — enumerate the ten
difficulties, switch nine off, solve that — is the chisel with a procedure
attached.

**What the runtime chose, without recording it.** Every arm built on this branch
is chisel-side. `weakener` lowers the target, `refuter` attacks the statement,
`searcher` hits a score function repeatedly, `reducer` finds sufficient lemmas.
Four subjects independently name the same absence: nothing restates the goal in
a different ambient setting (`01`§A4, `06`§A4, `08`§A1, `09`§A6).

**Why it was chosen.** Not by argument. The rising sea is unaffordable under
`MAX_ATTEMPTS = 8` and `STUCK_THRESHOLD = 2`: Grothendieck's own case took
sixteen years and Deligne closed it (`01`§A10). So the runtime's thresholds
*decided the methodology*, and no document says so.

**The one part that is affordable.** Scholze supplies a firing rule Grothendieck
does not: a change of setting earns its place only if it covers the case where
the old setting was working well (`08`§A1). That is checkable — the new setting
must reproduce a result the run already established — and `research/CLAIMS.md`
can check it.

### Split 2 — What counts as a result

| Position | Subjects |
|---|---|
| The kernel, or nothing | Tao (§21–23), Scholze, and the branch's `lean_check` |
| The kernel is a model, and the model is not the thing | Arnold, Thurston |
| Certainty is a purchasable good with a price | Zeilberger |
| The statement, without the proof | Ramanujan, Grothendieck (§A5) |

This is the deepest split in the set and the runtime took one side silently.
`Status::Formalised` exists; nothing records that there was a side to take.

Four positions, and they are not on a line:

- **Arnold** (`09`§A3, §A5): "The longer and fancier is the chain of deductions
  ('proofs'), the less reliable is the final result." Formalisation is a
  modelling step, and the danger is a sound derivation from a statement that
  does not mean what you think.
- **Thurston** (`05`§A3, §A4): formal definitions evaporate the differences
  between the seven ways of understanding a derivative. He explicitly refuses
  the weak reading — "I am not advocating any weakening of our community
  standard of proof" — and asks instead for proofs written so a *reader* can
  find the weakness.
- **Zeilberger** (`06`§A1, §A2): prices, composing additively along deduction,
  and random specialisation buying almost-certainty for an epsilon of the cost.
- **Grothendieck and Ramanujan**: stop before the proof. Grothendieck because it
  is "a matter of 'trade', not to say routine"; Ramanujan because he apparently
  never considered writing one.

**And the strongest evidence runs the other way.** Scholze (`08`§A6) reports
that the Liquid Tensor Experiment taught him *why his own proof worked* — "When
I wrote the blog post half a year ago, I did not understand why the argument
worked" — after a year of obsession had not. That is a claim for formalisation
that Tao's filter argument does not make and that the runtime does not capture,
because `lean.rs` files a boolean.

**The unnoticed agreement.** Arnold and Zeilberger are the two subjects most
opposed on the value of rigour, and they prescribe the *same mechanism*:
continuous external control by instances. Arnold's "if one does not control
oneself (best of all by examples), then after some ten pages half of all the
signs in formulae will be wrong" and Zeilberger's random specialisation are the
same act with different justifications (`09`§A4, `06`§A2). The runtime does it
exactly once, at the end, as `SOLVED`'s program-on-disk requirement.

**What follows.** Not a change of side. A record that a side exists — and three
cheap additions that every position above would accept: derivation *depth* in
the ledger (Arnold), a *targeting rule* for which claims deserve the kernel
(Scholze, off `blueprint.rs`'s in-degree), and an *explanation* output from
`lean.rs` beside its verdict (Scholze).

### Split 3 — Solitary against collective

| Position | Subjects |
|---|---|
| Undivided concentration; tell nobody | Wiles, Perelman |
| The unit of contribution should be as small as possible | Erdős, Gowers, Tao (§31–36) |
| Publish the infrastructure so others can work | Thurston, Grothendieck (§A2) |

Wiles is explicit that concentration was *bought*: "You can't really focus
yourself for years unless you have undivided concentration, which too many
spectators would have destroyed" (`04`§A2). Gowers designed the opposite:
"The ideal outcome would be a solution of the problem with no single individual
having to think all that hard" (`03`§A11).

**Both solitary cases needed others at the end.** Wiles could not repair his own
gap without Taylor; Perelman's proof took three independent teams and five years
to establish (`10`§A3). That is the honest form of the finding, and it argues
for the runtime's structure rather than against it.

**What the runtime chose.** Collective, by construction — twenty-two
tool-boundaried roles, which `../tao/02` F2 identifies as its real strength.
Two things it does not have:

- **The half-baked contribution.** Every ledger demands a well-formed block, and
  `note_scratch` is deliberately unreachable from durable recall, so a
  provisional thought cannot be seen by another role at all (`03`§A11).
  Gowers's rule is "just give quick reactions".
- **A measurement of what observation costs.** `docs/solution-loop.md` already
  flags that four of five standing teams duplicate a loop arm and says the
  duplication was never chosen. Wiles is the argument for resolving it by
  measurement.

### Split 4 — Where computation belongs

| Position | Subjects |
|---|---|
| Compute continuously; it is the only reliable control | Arnold |
| Compute to *discover*, with no goal in hand | Ramanujan, Zeilberger, Erdős (§A4 inverted) |
| Compute first, as reconnaissance on the goal | Tao (§20), and the runtime's `method_policy.md` |
| Deliberately refuse the machine's speed | Gowers |
| Never computed at all | Grothendieck, Wiles, Thurston (for the Haken proof), Scholze, Perelman |

The last row is the finding. **Five of the ten subjects' central results
involved no computation at any stage** — Grothendieck's four programmes, Fermat
over eight years, the geometrization theorem for Haken manifolds, the Liquid
Tensor theorem, and the Ricci flow papers. `../tao/02` independently reports
that computation was decisive in one of eleven Tao programmes and decorative in
most of the rest.

`method_policy.md` opens by requiring computation before prose. That is the
right default for Project Euler, whose problems have a number as an answer, and
it is a policy the whole conjecture side of the runtime inherits without anyone
choosing it.

**The distinction nobody in the runtime draws.** Tao's computation is
*goal-directed reconnaissance*: build the naive oracle, check the statement.
Ramanujan's and Arnold's is *undirected exploration*: compute in the
neighbourhood and see what falls out. `pattern_finder` has the tooling for the
second — `analyze_sequence`, `find_linear_recurrence`, `oeis_lookup` — and is
pointed at the first.

**And the missing guard.** Arnold's `1, 2, 4, 8, 16, 29` (`09`§B1): five terms
of a doubling sequence are consistent with several closed forms and the sixth is
where they part. `pattern_finder` emits patterns with no refutation step, so
every pattern it produces enters the ledger untested. Gowers's rule is the
generalisation — "a sufficiently simple general statement that is not obviously
true is almost certainly false" (`03`§A7) — and it is a *gate before proving*,
where the runtime's refuter is a cadence *after* committing.

### Split 5 — Whether a run should start cold

| Position | Subjects |
|---|---|
| Take a stalled programme and supply the missing piece | Perelman, Wiles |
| Publish the infrastructure so someone else can | Thurston, Grothendieck |
| Distribute a priced, statused catalogue | Erdős |
| Rediscovering known work is not wasted | Grothendieck (§A9) |

No disagreement here — this is the closest the set comes to unanimity, and it is
`../tao/04` R13 and `docs/tao-proposals.md` #6, the highest-value unbuilt item,
arriving from four directions.

**What each adds that #6 does not say:**

- **Perelman**: the transferable unit is not a technique but a *named gap*.
  Hamilton had stated publicly what was missing; Perelman took that and adopted
  the machinery wholesale rather than auditing it. `research/BACKWARD.md`
  already stores gaps in publishable form — `id`, `lemma`, `status`, first move
  — inside a directory nothing outside the workspace reads (`10`§A1).
- **Thurston**: publish the infrastructure *ahead of* the theorem, in terms that
  do not mention the motivating problem, and the test of success is whether
  someone else can use it (`05`§B2).
- **Erdős**: the catalogue needs *status and price*, and it needs a curator's
  warning attached — "Do not assume that an 'unsolved' problem is in fact
  unsolved" (`02`§A6).
- **Grothendieck**: the counterweight. Duplicating the literature *internally*
  may be the run's only real understanding of it, so the novelty check belongs
  on the reported answer and never on the derivation (`01`§A9) — which is how
  the branch built it.

---

## Where all eleven agree

Four things. These are requirements, not settings.

**1. Enumerate the difficulties by name, before choosing a method.** Tao §1 does
it to switch them off; Scholze does it and changes the ring instead (`08`§A4);
Gowers's move generator is the mechanised form (`03`§A1); Arnold's scheme starts
with observation of special cases (`09`§A2). Nobody in the set starts by
attempting the full statement. **The runtime does** — `attempt_step` goes at the
goal, and `weakener` runs on a cadence beside it rather than before it.

**2. A failed attempt carries information that must be extracted.** Tao §3 (the
way it fails is instructive); Gowers §A6 ("understand why it is unhelpful, and
use that understanding"); Wiles's abandoned Iwasawa approach becoming the repair
three years later (`04`§B1); Ramanujan's two wrong identities generating a paper
ninety-six years on (`07`§B2). **The runtime discards it**: `killed-by` may be
empty, and `refuted`/`spent` are absorbing.

**3. The artefact worth optimising is the one a later worker can pick up.**
Thurston `05`§B1 with the negative result attached — he emptied his own field by
writing for readers who shared his background. Grothendieck's `01`§A11 complaint
about the "conspiracy of silence" around the generative phase. Gowers's whole
human-style-output premise. Erdős's catalogue. **The runtime keeps the record
and reads none of it**: `reflections/` is loop-owned and indexed, `trace.jsonl`
is gitignored, and no derived ledger reads the abandoned attempts.

**4. Scrutiny should scale with what rests on a claim, not with how surprising
it is.** Scholze formalises because the result "will be used as a black box"
(`08`§A2), and volunteers that a wrong proof of weight-monodromy once "passed
judgment of top mathematicians". Thurston's social-validity mechanism (`05`§A4)
is the same mechanism *failing*, described approvingly. **The runtime has no
targeting rule at all** — and `blueprint.rs` computes the in-degree that would
supply one.

---

## The three cheapest things this reading produces

Ranked by evidence weight against implementation cost. The full ranking is
[`docs/methods-proposals.md`](../../docs/methods-proposals.md); these are the
ones where more than two subjects converge on a change that is a graph walk over
a ledger the runtime already derives.

1. **The inherited-hypothesis check.** A hypothesis that no step below cites is
   detectable from `research/BACKWARD.md`'s `rests-on` edges. Grothendieck
   (`01`§A3) says drop it; Gowers (`03`§A10) says rank moves by what they close
   off; Scholze (`08`§A7) reports that being forced to state which properties an
   argument actually uses is what produced the LTE simplification — which is the
   mechanism `docs/tao-proposals.md` #11 says it lacks. **Three subjects, one
   graph walk.**

2. **Formalisation targeting off the blueprint's in-degree.** `blueprint.rs`
   already builds the statement graph. Scholze's criterion — verify what will be
   used as a black box by people who will not re-derive it — is exactly
   in-degree. Nothing reads it that way.

3. **A refutation that produces a repaired statement.** `07`§B2 is the measured
   case: two false claims yielded corrected series, a generalised Voronoi
   summation formula, and a new class of transforms. The refuter arm already
   runs; asking it for the nearest true statement before it files the verdict is
   a prompt-and-schema change, and the guard is stated — the repair must still
   entail something the run wanted.
