# Scholze: find the category, then verify what you cannot check

**The axis.** Grothendieck's rising sea (`01`) with two things attached that
Grothendieck did not have: a formalisation practice, and a published, dated,
first-person account of what formalisation *did to the mathematician's own
understanding*. Thurston (`05`) argues that formal proof evaporates the
differences between understandings; Zeilberger (`06`) argues that certainty is
overpriced; Scholze ran the experiment and reported the opposite of both, in his
own words, about his own theorem.

That report is the most valuable thing in this file, because it is the only
direct evidence in the whole directory bearing on whether `lean_check`'s
premise — that kernel verification is worth what it costs — holds for a working
mathematician on a hard result.

**Source keys.** **[LTE]** = Peter Scholze, *Liquid tensor experiment*, guest
post on the Xena Project blog, 5 December 2020,
<https://xenaproject.wordpress.com/2020/12/05/liquid-tensor-experiment/>;
**[LTE6]** = *Half a year of the Liquid Tensor Experiment: Amazing
developments*, 5 June 2021,
<https://xenaproject.wordpress.com/2021/06/05/half-a-year-of-the-liquid-tensor-experiment-amazing-developments/>;
**[LTEdone]** = *Completion of the Liquid Tensor Experiment*, Lean community
blog, <https://leanprover-community.github.io/blog/posts/lte-final/>.

## Accuracy conventions

- **[LTE] is a guest post written by Scholze**, hosted by Kevin Buzzard, per the
  blog's own header. Quotations below are Scholze's own words except where
  attributed otherwise. [LTE6] is an interview format: Buzzard asks, Scholze
  answers, and the attribution is stated per quotation.
- **All three sources came through summarising fetches.** The wording is
  reported as those fetches returned it. Treat it as reported verbatim rather
  than independently confirmed against the page HTML — which matters more here
  than usual, because §A2 and §B2 turn on the precise wording of Scholze's
  self-assessment.
- **[LTEdone] does not answer the question most worth asking.** The fetch found
  no statement in it about whether the formalisation uncovered an error in the
  original proof, and no closing statement from Scholze. The absence is reported
  rather than filled in: **this file does not claim that the formalisation found
  a gap, and does not claim that it found none.**
- **The mathematics is not the point and is not vouched for here.** Condensed
  sets, `Ext` vanishing for `M_{p'}(S)` against a `p`-Banach space, and the ring
  `Z((T))_{>r}` appear because Scholze's methodological claims are about them.

---

## §A Stated method

### A1. Replace the ambient category, at the foundations `[STATED]`

The claim in its strongest form, stated by him as a claim about foundations
rather than a convenience.

> "I want to make the strong claim that in the foundations of mathematics, one
> should replace topological spaces with condensed sets...this claim is only
> tenable if condensed sets can also serve their purpose within real functional
> analysis." — Scholze, [LTE]

**Agent:** this is `01`§A4 — find the world the problem is native to — with the
test condition attached, and the test condition is the transferable half.
Grothendieck's move has no firing rule; Scholze supplies one. *A proposed change
of ambient category earns its place only if it covers the case where the old one
was working well.* Real functional analysis is where topological spaces are
comfortable, so that is where the replacement has to be shown, and that is the
theorem he set out to prove.

For the runtime: there is still no arm that proposes a change of setting
(`01`§A4, `06`§A4). If one is built, this is its acceptance criterion, and it is
implementable — the new setting must reproduce a result the run has *already
established* in the old one, which `research/CLAIMS.md` can check.

### A2. Formalise what will be used as a black box `[STATED]`

His stated reason, and it is a risk argument rather than a purity argument.

> "I spent much of 2019 obsessed with the proof of this theorem, almost getting
> crazy over it...I still have some small lingering doubts." … "nobody else has
> dared to look at the details." — Scholze, [LTE]

> "As it will be used as a black box, a mistake in this proof could remain
> uncaught." — Scholze, [LTE]

And the calibration evidence he volunteers against himself:

> "I have occasionally been able to be very persuasive even with wrong
> arguments...I once had a full proof of the weight-monodromy conjecture that
> passed judgment of top mathematicians, but then it turned out to contain a
> fatal mistake." — Scholze, [LTE]

**Agent:** this is a *targeting* rule for an expensive control and the runtime
has no targeting rule at all — `lean_check` is available to `lean_prover` and
what gets formalised is whatever that role decides to formalise. Scholze's
criterion is precise and mechanical: formalise the results that *many other
results will rest on and that nobody will re-derive*. `blueprint.rs` computes
exactly this. It derives a node per goal, lemma and claim, with standing as the
minimum over what a node rests on — so **in-degree in the blueprint graph is a
ready-made priority order for kernel verification**, and nothing reads it that
way. This is the cheapest proposal in this directory: a derived "verify these
first" list off a graph the branch already builds.

The weight-monodromy anecdote is the other half. A proof that "passed judgment
of top mathematicians" and was fatally wrong is Thurston's social-validity
mechanism (`05`§A4) failing, reported by someone it failed for. Two subjects,
opposite conclusions, same mechanism.

### A3. Know which quantifier structure you are in `[STATED]`

An unusually operational piece of self-assessment.

> "a statement of the form ∀∃∀∃∀∃, and there's no messing around with the order
> of the quantifiers. It may well be the most logically involved statement I
> have ever proved." — Scholze, [LTE]

He characterises the proof itself as "very much of *arithmetic* nature...
nontrivial argument fighting with estimates against homological algebra"
([LTE]).

**Agent:** quantifier alternation depth is computable from a statement and is a
genuine difficulty signal — it is what makes a statement hard to check by
reading and easy to get wrong by paraphrase. `02`§B1 records the Erdős-728
episode where exactly this went wrong: "it was noted that `C` was meant to be
taken arbitrarily large, and I myself misread the problem". Both the model and
the human misread a quantifier. Nothing in the runtime records a goal's
quantifier structure, and `research/CLAIMS.md`'s `hypotheses` field is the place
it would live.

### A4. State the two obstructions by name `[STATED]`

Before the proof, he names what is in the way, in a form that reads like a
difficulty list.

> "The real numbers are not locally profinite...there's a mismatch between the
> objects M_p'(S), V and the category Cond(Ab)." — Scholze, [LTE], problem (a)

> "Putting bounds on the real numbers leads to subsets that are not stable under
> addition anymore...very bad news for a person that has spent all of their life
> in the p-adic world." — Scholze, [LTE], problem (b)

**Agent:** this is `../tao/01`§1's enumeration of difficulties, done by a
theory-builder rather than a trick-user, which is evidence that the enumeration
step is common ground across the split this directory is organised around.
`weakener` on this branch names the difficulties and builds a ladder that turns
them off. Scholze names them and does not turn any off — he changes the ring.
Same first step, divergent second.

### A5. Take the detour that looks absurd `[STATED]`

The move, and he flags its strangeness himself.

> "To prove a theorem about real vector spaces (in fact, to set up real
> functional analysis) we have to work with the arithmetic ring Z((T))_{>r}!" —
> Scholze, [LTE]

**Agent:** `research/APPROACHES.md` has a `precedent` field filled in by the
`research` role, and an approach with no precedent is weaker for it. Scholze's
detour has no precedent by construction — that is what makes it the move. The
schema is right to want precedent and should not treat its absence as
disqualifying, and today nothing distinguishes "no precedent because nobody
tried" from "no precedent because it is a bad idea".

### A6. The formalisation is what taught him why his proof worked `[STATED]`

The finding, and it is worth stating before quoting: **Scholze reports that he
did not understand why his own argument worked until other people formalised
it.**

Asked whether he learned any mathematics during the formalisation:

> "What actually makes the proof work! When I wrote the blog post half a year
> ago, I did not understand why the argument worked." — Scholze, [LTE6]

The specific thing he learned, via the formalisation team's work on convex
geometry for Gordan's lemma:

> "the key thing happening is a reduction from a non-convex problem over the
> reals to a convex problem over the integers" — Scholze, [LTE6]

That is the answer to A5's own question — why the arithmetic detour is
necessary — and it arrived eighteen months late, from the verification effort.

**Agent:** the strongest single piece of evidence in this directory for the
branch's `lean_check`, and it is evidence for something the proposal did not
claim. `../tao/02`§6 and `docs/tao-gap-analysis.md` justify the kernel as a
*filter*: it catches the plausible-but-wrong argument. Scholze's report is that
it also acted as an *explainer*, and that the explanation was not available by
any cheaper route — he had spent a year on the proof and it did not come.

The runtime cannot capture this today, because `lean.rs` files a verdict —
compiled, `sorry`, axioms — and a verdict is a boolean about correctness. What
Scholze got out was a sentence about *which step is load-bearing*. Nothing in
the workspace has a place for that sentence; `research/ROOT.md`, "what the
library means", is the nearest and is not connected to the Lean verdict at all.

### A7. Formalisation produces simplifications, from the formaliser `[STATED]`

The second-order effect, and Scholze credits it to Commelin rather than to
himself.

Commelin found that the Breen–Deligne resolution could be axiomatised to capture
only the structural properties actually used, and:

> "one can actually give a nice and completely explicit object satisfying those
> axioms, and this is good enough for all the intended applications" — Scholze,
> [LTE6]

The consequence, per [LTE6]: stable homotopy theory was no longer needed, and
the rest of the proof became "considerably more explicit and more elementary".

**Agent:** this is `../tao/02`'s sunflower cascade — simplification as a
research mode, listed as proposal #11 and *not built*, with the note that
"unclear, and that is the finding". Scholze supplies the missing mechanism.
The simplification did not come from someone deciding to simplify; it came from
someone being **forced to state exactly which properties the argument uses**,
which is what formalisation is. So the buildable version of proposal #11 is not
a `simplifier` role. It is a step that asks, of an established claim, *which of
its hypotheses does the proof actually consume* — which is `03`§A3's inherited-
hypothesis check (`01`§A3) arriving for the third time in this directory, and
which `research/BACKWARD.md`'s `rests-on` edges make computable.

Buzzard's characterisation of the whole experience, reported in [LTE6], is worth
keeping as the human framing: it resembled working "with a very careful
colleague".

---

## §B Anatomy

### B1. Perfectoid spaces (2011–)

**(a)** Long-standing problems in `p`-adic geometry and arithmetic, including
cases of the weight-monodromy conjecture.

**(b)** Build the category. Perfectoid spaces make precise a passage between
characteristic 0 and characteristic `p`, so that a problem in one becomes a
problem in the other.

**(c)** Not detailed here. See the accuracy conventions — the mathematics is
outside what was verified for this file.

**(d)** Nothing computational.

**(e)** The relevant ladder item is a *failure*, and it is one Scholze
volunteers about himself: a full proof of weight-monodromy that "passed judgment
of top mathematicians, but then it turned out to contain a fatal mistake"
([LTE]). He raises it as evidence about his own persuasiveness, not about the
theory.

**(f) MOVE — build the category that makes the transfer legal.** *Trigger:* two
settings are known to be analogous and the analogy has no mechanism. *Action:*
construct the object in which the analogy is a theorem, then transport.
*Check:* the construction must recover known results in both settings, and its
author's confidence is not evidence — see (e).

### B2. The Liquid Tensor Experiment (December 2020 – July 2022)

The entry that matters, because it is a controlled experiment on formalisation
with a first-person report of the outcome.

**(a)** Theorem 1.1 (Clausen–Scholze): for `0 < p' < p ≤ 1`, `S` profinite, `V`
a `p`-Banach space, `Ext^i_{Cond(Ab)}(M_{p'}(S), V) = 0` for `i ≥ 1` ([LTE]).
Posed publicly, as a challenge, with a stated proof already in hand.

**(b)** The reframing is the one Scholze only understood later: reduce a
non-convex problem over the reals to a convex problem over the integers, which
is what the detour through `Z((T))_{>r}` accomplishes ([LTE6]).

**(c)** Condensed mathematics; homological algebra with estimates; the
Breen–Deligne resolution, which Commelin then axiomatised away ([LTE6]); Lean 4
and Mathlib; Gordan's lemma and convex geometry, contributed by the formalisers.

**(d)** All of it. That is the experiment.

**(e)** The timeline:

| Date | Event |
|---|---|
| 2019 | Scholze proves it; "almost getting crazy over it" ([LTE]) |
| 5 Dec 2020 | Challenge posted. "I still have some small lingering doubts"; "nobody else has dared to look at the details" ([LTE]) |
| 5 Jun 2021 | Half-year report. Scholze: "I did not understand why the argument worked" ([LTE6]). Commelin's axiomatisation removes stable homotopy theory |
| 14 Jul 2022, 15:46:13 EST | Completed, "A year and a half after the challenge was posed" ([LTEdone]); around 27 named contributors |

Compare `../tao/02`§6: PFR formalised in Lean in three weeks by ~25 strangers
with Tao writing ~5% of it. Same collaborative shape, eighteen months instead of
three weeks — which is the honest scale factor between formalising a clean new
result and formalising one described by its author as "the most logically
involved statement I have ever proved".

**(f) MOVE — formalise the load-bearing black box, and read the *explanation*
out, not only the verdict.** *Trigger:* a result will be used without being
re-derived, by consumers who will not check it. *Action:* verify it formally,
and require the verification to state which step the argument turns on.
*Check:* the value claimed here is Scholze's own report and it is a single case
— the branch's `lean_check` should not be re-justified on it, but a *targeting*
rule (A2) and an *explanation* output (A6) are both cheap and both currently
absent.

---

## §C Against Tao

| Tao (`../tao/01`) | Scholze | Which, when |
|---|---|---|
| §21–23 a proof is what the kernel accepted | A2/A6: yes, and the kernel also explains. But note *he had the proof first* — the kernel checked and taught, it did not find | Agreement, with a stronger claim than Tao makes. The runtime implements the filter and not the explanation |
| §1 turn off nine of the ten difficulties | A4/A5: name the difficulties, turn off none, change the ring instead | `weakener` versus the unbuilt reframing arm, stated by someone who does both halves of the first step |
| §19 a short proof of a famous problem is probably known | A2: a *long* proof by a famous mathematician, that top mathematicians accepted, was fatally wrong | The complementary prior, and the runtime has no version of it. Scrutiny should scale with how much rests on a claim, not with how surprising it is |
| §35 one monotone, legible progress statistic | B2: eighteen months of formalisation during which the theorem did not change | Same problem as `01`§A1 and `04`§A1. Verification work is invisible to a goal-reading statistic |
| §—, simplification as a mode (`../tao/02`§8, proposal #11) | A7: simplification came from being forced to state which properties are used | The mechanism proposal #11 says it lacks. This is the single most actionable line in the file |

**The one-line version.** The branch built `lean_check` to stop a
plausible-but-wrong argument. Scholze's report is that formalisation did
something else as well — it told him which step his proof turned on, after a
year in which he could not tell — and the runtime throws that output away
because `lean.rs` files a boolean.

---

## Sources

Fetched for this file (all summarising fetches; see conventions):

- Peter Scholze, *Liquid tensor experiment* (guest post), Xena Project,
  5 December 2020 —
  <https://xenaproject.wordpress.com/2020/12/05/liquid-tensor-experiment/>
- *Half a year of the Liquid Tensor Experiment: Amazing developments*, Xena
  Project, 5 June 2021 —
  <https://xenaproject.wordpress.com/2021/06/05/half-a-year-of-the-liquid-tensor-experiment-amazing-developments/>
- *Completion of the Liquid Tensor Experiment*, Lean community blog —
  <https://leanprover-community.github.io/blog/posts/lte-final/>

Consulted and not quoted: the n-Category Café and Silicon Reckoner discussions
of LTE. Not reached — `[UNVERIFIED]` as sources: Clausen–Scholze's condensed
mathematics lecture notes; Scholze's perfectoid spaces papers; any statement by
Scholze after July 2022 assessing the completed formalisation, which is the one
source that would settle whether the Lean effort found an error and which this
file explicitly does not claim to know.
