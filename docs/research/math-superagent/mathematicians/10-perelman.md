# Perelman: completing somebody else's programme

**The axis.** Every other subject started something. Perelman finished
something — Hamilton's Ricci flow programme, which had been running for twenty
years and was stuck on one identified obstruction — and supplied exactly what
was missing. He then posted the result as three arXiv preprints, submitted it to
no journal, and left the verification to other people, which took five years and
three independent teams.

He is in the set for two reasons. The runtime always starts cold, and
`../tao/04` R13's shared technique library — proposal #6, the highest-value
unbuilt item — is the same gap. And the runtime's terminal routes assume the run
that solves a problem is the run that reports it, which is not how this worked.

**Source keys.** **[E1]** = Perelman, *The entropy formula for the Ricci flow
and its geometric applications*, arXiv:math/0211159, 11 November 2002, 39 pages,
<https://arxiv.org/abs/math/0211159>; **[WP]** = the Wikipedia article
*Grigori Perelman*, <https://en.wikipedia.org/wiki/Grigori_Perelman>.

## Accuracy conventions

The most heavily mythologised subject in this set after Erdős, and the one with
the least first-person methodological writing of anyone here.

- **He wrote nothing about method.** §A is three entries, all `[INFERRED]` from
  the artefacts, and that is the honest size. Per
  [`00-conventions.md`](00-conventions.md), the alternative — inventing §A
  entries from the biography — would be manufacturing the finding.
- **[E1] is quoted from its arXiv abstract page**, which was fetched. The three
  papers themselves were not read; nothing here makes a claim about their
  contents beyond what the abstract states and what [WP] reports.
- **[WP] came through a summarising fetch.** Perelman's quoted refusals are
  reproduced as that fetch returned them, and every one of them originates in
  journalism — principally the 2006 *New Yorker* piece by Nasar and Gruber,
  which was not reached. They are the most-repeated Perelman quotations in
  existence and they are `[UNVERIFIED]` here.
- **The withdrawal-from-mathematics narrative is not used as evidence about
  method.** It is biography, it is contested, and no harness conclusion should
  rest on it.
- **The Cao–Zhu priority dispute is not covered.** It is a real part of the
  history and it bears on credit rather than on method.

---

## §A Method, read off the artefacts

### A1. Attack the named obstruction in somebody else's programme `[INFERRED]`

The structure of the whole solve, and it is unusual enough to state plainly.
Hamilton had built Ricci flow through the 1980s and by 1999 had reduced
geometrization to two things: a three-dimensional surgery technique, and a
conjecture on long-time behaviour ([WP]). The gap was identified, public, and
specific — Hamilton "could not achieve quantitative understanding of how
singularities occur in three-dimensional settings", though he had done it in
four ([WP]).

Perelman supplied exactly that: the noncollapsing theorem giving volume control,
and the canonical neighbourhoods theorem showing that microscopic singularities
look like cylinders or collapsing spheres ([WP]). With those, surgery in three
dimensions becomes constructible.

[E1] states the relationship in its own abstract: the paper verifies "several
assertions related to Richard Hamilton's program for the proof of Thurston
geometrization conjecture for closed three-manifolds" and gives "a sketch of an
eclectic proof of this conjecture" ([E1]).

**Agent:** the runtime's unit of work is a workspace containing one problem, and
a run starts from `problem.md` and `GOAL.md` with no inherited machinery —
`../tao/04` R13 and `docs/tao-proposals.md` #6. Perelman's shape is the argument
for the *other* half of that proposal, which #6 does not make: not only should
techniques be shared across problems, but a run should be able to take as its
goal **a named gap in another run's decomposition**. `research/BACKWARD.md`
already stores gaps with `id`, `lemma`, `status` and a first move — it is
already a publishable work item — and nothing outside its own workspace can
read it.

### A2. Import the tool from the far field, and say where it came from `[INFERRED]`

The entropy functional in [E1] is presented in physical terms: "a monotonic
expression for the Ricci flow, valid in all dimensions and without curvature
assumptions", interpreted "as an entropy for a canonical ensemble" ([E1]).

Note what the abstract claims about generality: *all dimensions*, *no curvature
assumptions*. That is Grothendieck's A3 (`01`) — take the statement with no
inherited hypotheses — done by someone attacking a specific three-dimensional
obstruction.

**Agent:** monotonicity is the operational content and it is the thing the
runtime most lacks. `../tao/01`§35 asks for one monotone, legible statistic;
`../tao/02` F3 says a long programme needs one; the judge's 1–5 score is written
to `state.scores` and read by no code. Perelman's entropy is what a real one
looks like: not a score on the run's conduct, but a quantity *about the
mathematical object* that provably moves one way. A runtime cannot invent such a
thing, but it can notice when its own work has produced one — a claim asserting
that some quantity is monotone under the process being studied is a distinguished
kind of claim, and `research/CLAIMS.md` has no way to mark it.

### A3. Publish the argument; decline to publish the paper `[INFERRED]`

Three preprints — November 2002, March 2003, July 2003 — posted to arXiv and
never submitted to a journal ([WP]). Verification was left to others: three
independent expositions between 2003 and 2008, by Kleiner–Lott, Cao–Zhu, and
Morgan–Tian ([WP]).

His stated reasons for refusing the Fields Medal and the Millennium Prize are
about the community rather than about the mathematics — "the prize was
completely irrelevant for me" and, on the Clay prize, "the main reason is my
disagreement with the organized mathematical community. I don't like their
decisions, I consider them unjust", with the specific objection that Hamilton's
credit was not shared ([WP], and see the accuracy conventions — these are
`[UNVERIFIED]`).

**Agent:** the runtime's `Solved` and `Reported` routes both assume the solving
run writes the final account. The five-year, three-team verification is the
actual cost of the alternative, and it is the strongest available argument
*against* letting a run stop at "the argument is on disk". It is also the
argument for the branch's `lean_check`: a kernel-checked artefact is the one form
in which "here is the argument, verify it yourselves" costs the reader hours
instead of years. Perelman is the case that makes `../tao/02`§6's three-week PFR
formalisation look like a different century of mathematics rather than a
different decade.

---

## §B Anatomy

### B1. Geometrization and Poincaré (2002–2003, verified to 2008)

**(a)** Thurston's geometrization conjecture — see `05`§B2, where Thurston poses
it and proves the Haken case, and writes in 1994 that "the full geometrization
conjecture is still a conjecture" and "I am convinced that the general proof will
be discovered". Poincaré follows as a corollary. Thurston's own line about the
two worst outcomes he avoided names the first as keeping a discovery to yourself
"perhaps with the hope of proving the Poincaré conjecture" ([TH] §6, quoted in
`05`).

**(b)** No reframing by Perelman. The reframing was Hamilton's — study the
manifold by flowing its metric — and had been in place for twenty years. What
Perelman changed was the *analysis of the flow's singularities*, which is the
one place it was stuck.

**(c)** Hamilton's Ricci flow and surgery programme, entire; a monotone entropy
functional presented in the language of statistical mechanics ([E1]); Thurston's
conjecture as the target and Thurston's Haken-case techniques as prior art.

**(d)** Nothing computational, in a proof about the singularity formation of a
geometric flow. That is now five of ten subjects in this directory with no
numerical component to the central result.

**(e)** The ladder, and it is a ladder of other people's work:

| Period | State |
|---|---|
| 1980s | Hamilton develops Ricci flow |
| 1982 | Thurston's geometrization conjecture; Haken case proved |
| by 1999 | Hamilton reduces geometrization to 3-D surgery plus a long-time behaviour conjecture ([WP]) |
| — | Hamilton stuck: no quantitative understanding of 3-D singularity formation, though the 4-D case was done ([WP]) |
| Nov 2002 | [E1], 39 pages. Entropy formula; noncollapsing |
| Mar 2003 | *Ricci flow with surgery on three-manifolds* |
| Jul 2003 | *Finite extinction time…* |
| 2003–2008 | Kleiner–Lott, Cao–Zhu, Morgan–Tian: three independent expositions ([WP]) |
| 2006 | Fields Medal declined |
| 2010 | Millennium Prize declined |

Eight months of preprints closing a twenty-year programme, then five years of
other people establishing that it closed.

**(f) MOVE — take the published obstruction, not the published problem.**
*Trigger:* a named programme exists whose remaining gap is stated in the
literature. *Action:* make that gap the goal, adopting the programme's machinery
wholesale rather than re-deriving or replacing it. *Check:* the gap must be
*stated as a gap by its own author* — Hamilton's was — and the programme's
machinery must be adopted, not audited. Perelman did not verify Ricci flow; he
used it. A runtime attempting this must therefore mark the imported programme
as `catalogued` rather than `established`, and accept that its result inherits
that standing.

**(f′) MOVE — look for a monotone quantity before looking for a proof.**
*Trigger:* the object under study evolves under a process, and the difficulty is
controlling what the process does. *Action:* search for a functional that
provably moves one way under it, before attempting the target. *Check:* the
monotonicity must hold "in all dimensions and without curvature assumptions"
([E1]) — that is, without the hypotheses that make the current case tractable —
or it is an artefact of the case rather than a handle on the process.

---

## §C Against Tao

| Tao (`../tao/01`) | Perelman | Which, when |
|---|---|---|
| §35 one monotone, legible progress statistic | A2/B1(f′): and the statistic should be about the mathematics, not the run | The runtime's only candidate is the judge's conduct score, which nothing reads. Perelman is what the requirement actually asks for |
| §27 reuse what is already proved; never re-derive | A1: and go further — take the *gap* someone else published, along with their machinery | The strongest form of `../tao/04` R13 in the directory. `research/BACKWARD.md` gaps are already publishable work items and are workspace-local |
| §31–36 collaboration, records, scale | A3: three preprints, no journal, no collaborators, five years of other people's verification | With `04` (Wiles), the second solitary case in the set. Note both needed others at the end — Taylor for Wiles, three teams for Perelman |
| §19 a short proof of a famous problem raises the prior it is known | B1: eight months of preprints closing a twenty-year programme, and it held | The prior is about *proofs that are short relative to the problem*, not about programmes that finish fast once the obstruction falls. The runtime's novelty check cannot tell those apart |
| §21–23 a proof is what the kernel accepted | A3: five years and three teams, because there was no kernel | The cleanest cost estimate available for the absence of formal verification, and the best argument for `lean_check` in this directory that is not `08`§A6 |

**The one-line version.** Perelman's result exists because a twenty-year
programme's remaining gap was stated publicly, in a form somebody else could
take up. The runtime writes exactly that artefact — `research/BACKWARD.md`, a
skeleton with gaps, each carrying a first move — into a directory nothing outside
the workspace ever reads.

---

## Sources

Fetched for this file:

- Grisha Perelman, *The entropy formula for the Ricci flow and its geometric
  applications*, arXiv:math/0211159 — <https://arxiv.org/abs/math/0211159>
  (abstract page; 39 pages, 11 November 2002)
- *Grigori Perelman* — <https://en.wikipedia.org/wiki/Grigori_Perelman>
  (summarising fetch; the timeline, Hamilton's gap, the verification effort,
  and the quoted refusals)

Not reached — `[UNVERIFIED]` as sources: the second and third preprints
(arXiv:math/0303109 and math/0307245); Sylvia Nasar and David Gruber,
*Manifold Destiny*, The New Yorker, 28 August 2006, which is the origin of the
refusal quotations; Kleiner–Lott's *Notes on Perelman's papers*; Cao–Zhu;
Morgan–Tian's *Ricci Flow and the Poincaré Conjecture*; Hamilton's papers on
Ricci flow and on the 4-dimensional surgery case.

`[TH]` in §B1(a) refers to Thurston's *On proof and progress in mathematics*,
read in full for [`05-thurston.md`](05-thurston.md).
