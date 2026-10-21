# ProofAtlas' trust model, read as a grading machine

Source: proofatlas.ai as of 17 August 2026, fetched as raw HTML and JSON.
Pages read in full: `/`, `/advances/`, `/research/` (the Papers index),
`/formalizations/`, `/formalizations/sendov-conjecture/`, its evidence record
`/proofs/artifact.sendov.conjecture.lean-package.v001.html`, the accepted
counterpart `/proofs/artifact.known-nd-rhin-log-time.paper-package.v002.html`,
`/collatz-predecessor-090/`, `/research/berlekamp-domineering-temperature-counterexample/`,
`/collaboration/sendov-conjecture/`, `/collections/landmark-theorems-in-mathlib/`,
plus the machine-readable `evidence.json` and `data/releases/*.json` behind them.

[`01-sendov-bundle-anatomy.md`](01-sendov-bundle-anatomy.md) dissected the
*bundle*. This note dissects the *site's grading of it* — the question of who
decides what a reader may believe, and on what.

The single most useful thing on the site: a 1,176-file Lean project with zero
`sorry`, a printed axiom list of exactly `[propext, Classical.choice,
Quot.sound]`, and no `native_decide`, is **not accepted**. The stated reason is
not mathematical. It is that the platform holds a *summary* of the build rather
than the build.

## 1. The grades, verbatim

There is no `/trust/` page — it 404s. The nav item labelled **Trust** (and
"How evidence works" on the Mathlib collection) points at `/#trust-model`, a
four-item section on the homepage titled *"The theorem comes first; its status
stays precise"*:

> A ProofAtlas page separates four things that are easy to confuse: the
> mathematical claim, its exact Lean statement, the recorded check, and whether
> the result has been approved for public release.
>
> 01 Mathematical claim … 02 Exact Lean statement … 03 Checked evidence — Inspect
> the build, unfinished-step scan, axioms, complete source, and reproducibility
> information. 04 Publication status — **A checked proof remains separate from an
> accepted or publicly approved result.**

That last sentence is the whole model. Everything below is its mechanism.

**Release-level grades** (the five outcomes on `/advances/`, quoted exactly):

| # | Result | Badge |
| --- | --- | --- |
| 01 | Natural-density Collatz descent in logarithmic time | `Accepted in ProofAtlas · Lean checked` |
| 02 | Collatz predecessor lower bounds at exponent 0.90 | `Accepted in ProofAtlas · Lean checked` |
| 03 | Sendov's conjecture proof package | `Lean formalization checked · acceptance review open` |
| 04 | Jackson Hamilton-decomposition counterexample | `Accepted in ProofAtlas · Lean-checked counterexample` |
| 05 | Rectangle-reachable Berlekamp counterexample | `Unverified manuscript · adversarial audit attached` |

**Per-page status triple.** Every formalization page carries the same three
fields, and they are independent:

```
Lean checked            Recorded build passed        (Sendov)
Unfinished proof steps  None
Publication             Review pending
```

versus, on the accepted Collatz page, the identical first two lines and
`Publication  Accepted formal theorem`.

**Paper grades** (`/research/`, the Papers & manuscripts index): `Partially
formalized`; `Latest listed version · Formalization planned`; `Earlier listed
version · Formalization planned`. The index states its own rule: *"publication
or editorial review is not a substitute for a Lean-checked proof."*

**Mathlib landmarks** are a fourth lane: `Upstream indexed — Pinned source bytes
verified locally`, `Locally reproduced — Exact upstream declaration replayed`,
atlas status `Reviewed public index`, with the explicit note *"These are existing
upstream Mathlib declarations, not new ProofAtlas results."*

**Machine-readable grades.** `data/releases/<slug>.json` carries
`releaseKind: "accepted_result"` (Collatz) or `"paper"` (Sendov). The
`evidence.json` carries the decisive block:

```
publicationReview.status                  accepted_result_recorded
                                        / accountable_review_not_recorded
publicationReview.acceptedResultId        accepted.known-nd-rhin-log-time…  / null
publicationReview.statementAlignmentStatus accepted / not_reviewed
publicationReview.reviewIds               [4 ids] / []
```

**Route grades**, inside a live workspace (`/collaboration/sendov-conjecture/`),
are a separate and finer vocabulary: `Active route`, `Narrowed route`, `Route
held in reserve`, `Useful but insufficient`, `Eliminated route`, `Not yet
justified`. Claims carry an *evidence posture* — `Reported result`, `Reported
reduction`, `Reported special case`, `Reported theorem candidate` — where
"reported" means the run said so and nothing checked it. Tasks carry `Ready to
work on`, `Prerequisites still open`, `Blocked by the current route`.

## 2. What moves a result up, and who decides

Four named review lanes, and they are the same four everywhere:

```
formal_evidence      statement_alignment      result_boundary      public_wording
```

The Sendov page lists them as open, with a fifth and sixth gate after:

> Review 1 Formal evidence — An independent reviewer must inspect the recorded
> build, declarations, unfinished-proof-step check, and disclosed foundations.
> Review 2 Statement alignment — The formal declaration must be reviewed against
> the theorem wording and its exact variant.
> Review 3 Result boundary — The limits must be checked so the page cannot imply
> a broader theorem.
> Review 4 Public wording — The explanation, infographic labels, and source
> presentation need independent review.
> Source — The permanent source link and downloadable files must be approved for
> public citation.
> Final record — Accepted result: After the four reviews and source route are
> ready, an accepted-result record must bind them to this exact formalization.

The accepted Collatz artifact has exactly four `reviewIds`, one per lane:
`review.accepted.formal_evidence.hc0dfbb66728d.v001`, and the
`statement_alignment`, `result_boundary`, `public_wording` siblings.

**Who decides is the surprise.** All four of those reviews were performed by
**Claude Fable 5**, with `independenceStatus: "not_asserted"`. Sendov's one
review was **OpenAI Codex**, also `not_asserted`. Every review names its own
limits in the record:

> The reviewer did not rerun Lean; it recomputed retained source, import-closure,
> and transcript bindings.

and, on Sendov:

> The lane did not establish formal-evidence, statement-alignment,
> result-boundary, novelty, specialist-review, or accepted-result status.

So the promotion from "Lean checked" to "Accepted in ProofAtlas" is decided by a
model reading digests, not by a mathematician reading mathematics. The site
never claims otherwise, but it never says it plainly either — the phrase used
throughout is *"An AI, human, or mixed reviewer"*. **The human in the loop is
Lech Mazur, and his recorded role is authorship and curation, not adjudication**:
*"sole named manuscript author, accountable curator, workflow designer, and
reconciler of model outputs."* Every release on the site has the same accountable
human. That is a real control — someone's name is attached — but it is a
*single* name across all five advances, which is not independence in any sense a
journal would recognise, and the site does not say so.

**Where the site did not say enough.** It never states who authors a review
prompt, what a review would have to find to *block* a promotion, or whether a
failed review is recorded anywhere. Every published review passed or is pending;
there is no rejected artifact on the site. A grading scheme whose lowest observed
outcome is "still pending" has not yet demonstrated it can say no.

## 3. Automatic versus human

Cleanly split, and the split is legible in one JSON object.

**Machine, per artifact** (`checkerEvidence`, Sendov shown):

```
allowedAxiomProfile          classical_mathlib_standard
axioms                       [Classical.choice, Quot.sound, propext]
buildStatus                  passed
noSorryStatus                passed
hasSorryAx                   false
hasUnexpectedAxioms          false
recordedBuildElapsedMs       0
buildTranscriptRecorded      false
collectionProvenanceRecorded false
```

The last three are the whole story, and section 4 is about them.

**Machine, per source** (`reproducibility` + `sourceFootprint`):
`leanToolchain: leanprover/lean4:v4.30.0-rc2`, `packageCommit`,
`dependencyLockHash`, `sourceArtifactHash`, and a per-file `sha256` +
`physicalLines` + `nonEmptyLines` + `bytes` for **all 1,160 files** of the
theorem's first-party import closure. `schemaVersion` on every record
(`public-site-source-evidence.v1`, `public-site-formalization-footprint.v1`,
`proofatlas.public-release-credit.v1`).

**Model, disclosed**: the four review lanes above.

**Human**: naming, curating, authorising release, and — implicitly — writing the
schema that decides everything else.

Notably the site does *not* rerun the big builds itself and says so:
*"ProofAtlas has not rerun the substantial Lean build for this page update or
reproduced the result in an unrelated implementation."* (Berlekamp). For the
Mathlib landmarks it does — *"Locally reproduced — Exact upstream declaration
replayed"* — which is the tell: they replay what is cheap, and grade the rest on
supplied evidence with the gap written down.

## 4. Why Sendov is not believed

The actual stated reason, from the evidence record page, under a heading the
site wrote for exactly this question:

> **Why ProofAtlas acceptance is still pending:** the supplied release audit
> records a successful one-worker rehash build, no unfinished proof commands, and
> the reported axiom closure. It does not retain the complete command/output
> transcript and clean-source collection record required for accepted-result
> status.

And on the theorem page, in the result boundary:

> The current ProofAtlas evidence record retains a source-supplied successful
> build and axiom audit but **not the complete command/output transcript**
> required for accepted-result status.
>
> ProofAtlas accepted-result status remains separate because **the retained audit
> is a summary rather than the complete build transcript** required by the current
> evidence contract.

In the JSON this is two booleans and a zero:

```
buildTranscriptRecorded       false
collectionProvenanceRecorded  false
recordedBuildElapsedMs        0        ("Recorded build time — Not retained in
                                         the supplied release audit")
reproducibility.collectedAt   null
reproducibility.collectorCommit null
```

against the accepted Collatz artifact's `true`, `true`, `2278`,
`"2026-07-17T04:05:14.062Z"`, `"887cc16a…"`.

Read that carefully, because it is not what a mathematician would guess. The
objection is **not** that the proof might be wrong, that the certificate is too
large to audit, or that Sendov is too famous to accept on one reading. It is a
**provenance** objection: the audit was produced by the *submitter's* machine and
handed over as a summary; ProofAtlas has no record of *its own* collector running
the build, so it cannot say which bytes were compiled by what, when. `LEAN_AUDIT.txt`
in the bundle (which does quote the literal `#print axioms` output, the 4,524-job
build line, and the `prlimit`/`timeout` settings) is exactly the artifact the
site is calling insufficient — because a transcript you were given is a claim,
and a transcript you collected is evidence.

Stacked on top, and separately recorded:

- `statementAlignmentStatus: not_reviewed` — nobody has yet checked that
  `theorem sendov_conjecture : SendovConjecture` unfolds to Sendov's conjecture.
  The whole proof rests on a 20-line `Theorem.lean` whose SHA-256 is published;
  the site declines to certify that those 20 lines say what the title says.
- *"The Lean development is not a line-by-line formalization of the manuscript."*
- *"Lean does not verify the manuscript's supplementary Python programs."*
- *"Specialist novelty review and independent statement-alignment review remain
  unrecorded."* and *"No independent external priority anchor is recorded."*
- *"The disclosed Lean foundations are standard logical foundations, not
  assumptions of Sendov's conjecture."* — a disclaimer against the *reader's*
  likely misreading of the axiom list, which is a nice touch.

The site's own one-line summary on `/advances/`: *"presented together without
treating submission as acceptance."*

**The lesson.** A kernel check is a statement about a *file*. Belief is a
statement about a *pipeline*: which file, checked by which toolchain, collected
by whom, at what time, and does the top-level statement mean the theorem. Sendov
maxes out the first and fails the rest, and the site scores those separately
rather than averaging them into a verdict.

## 5. `native_decide` is allowed — and priced

Contrast worth having, because this repository refuses it by test.

The accepted Collatz predecessor result uses `native_decide` twice and is still
graded `Accepted in ProofAtlas · Lean checked`. The site does not hide it; it
*counts* it:

```
Trust dependencies      3 standard foundations · 2 native checks
Disclosed Lean native checks   2
Finite LP rows checked         129,140,163
Adaptive-potential inequalities checked  215,233,605
```

and prints the axiom names in full:

```
Erdos1135.KrasikovLagarias.k18AdaptiveForcedPotentialCertificate_check._native.native_decide.ax_1_1
Erdos1135.KrasikovLagarias.k18Gamma901EncodedCertificate_check._native.native_decide.ax_1_1
```

with *"The native-assisted certificate checks are not described as kernel-clean"*
under **What is not claimed**, *"Independent Python and C++ verifiers replay the
relevant calculations. They reduce the native-evaluation trust boundary without
replacing its disclosure"*, and an **open route** to remove it: *"Reduce the
computation trust boundary — Replace or complement the native finite checks with
smaller kernel-checkable certificates, independently replayable proof objects, or
reusable verifier lemmas."*

Meanwhile the Berlekamp artifact advertises the opposite: *"The development
contains no `sorry`, custom mathematical axiom, or native-decision shortcut.
Python output and file hashes are not theorem evidence."* — 995,069 states pushed
through *"a proved checker and ordinary kernel reduction"*.

So their `allowedAxiomProfile` is a **named profile with named exceptions and a
standing debt**, where ours is a binary refusal. Ours is stricter and cheaper to
enforce; theirs can publish a result ours would have to discard, at the cost of
carrying the debt in public. Both are defensible. The part worth stealing is that
the exception is *counted, named at the axiom level, and paired with the work
item that retires it*.

## 6. Volume as evidence

The headline numbers are `271,452 unique investigation lines`, `85 workspaces`,
`312 ready tasks`, `70 open conjectures`, `15 partially resolved or scoped
variants`, and at the foot: `146 theorem families · 176 recorded Lean
declarations · 2,299 first-party Lean files · 447,316 Lean source lines`.

Volume *is* the front-page framing. But the guards are unusually good, and each
is placed on the element it guards rather than in a footer:

- Above the bar chart: *"Text volume on one scale; **this is not a measure of
  closeness to a proof**"*.
- Beside it: *"**How this is measured** This measures mathematical investigation,
  not proximity to a proof. Code, data, logs, repeated text, operational
  instructions, and generated presentation copy are excluded. Exact duplicates
  count once across the Atlas; every bar uses the same linear scale."*
- Under the route map: *"Working overview, not proof. The map shows selected
  recorded relationships; **more nodes or edges do not establish correctness or
  completion**."*
- Under every line count: *"Line counts exclude blank lines; comments and
  documentation count."* and *"This is source footprint, **not a proof-quality or
  difficulty score**."*
- The composition breakdown is itself a guard: Sendov's 3,605 lines are
  `Argument development 88% · Explored or eliminated routes 2% · Computational
  analysis 2% · Open obligations 4% · Definitions and setup 5%`. A workspace that
  is 88% prose says so on its own page.

**What would make it misleading anyway.** Three things the guards do not reach.

1. **Dedup is exact-match.** *"Exact duplicates count once"* — near-duplicate
   restatement, the characteristic failure of a long agent run, counts every
   time. 13,298 lines on the Quillen conjecture is the top bar; nothing on the
   page distinguishes thirteen thousand lines of new argument from thirteen
   thousand lines of the same argument rephrased.
2. **The scale is monotone.** A line is only ever added. An eliminated route is
   filed under `Explored or eliminated routes` (2–4% everywhere) and the total
   still grows. There is no way for a workspace to get *smaller* by learning that
   it was wrong.
3. **Volume and grade are on different pages.** `/collaboration/sendov-conjecture/`
   reports research stage 12, nine routes, 20 mapped statements, and *"No complete
   proof or counterexample is claimed"* — while `/formalizations/sendov-conjecture/`
   carries a complete Lean proof of the same conjecture. Those are two independent
   lanes on one problem and neither page tells the reader the other exists in that
   state. A reader landing on either gets a coherent and incomplete picture.

Credit where due: item 3 is the only place I found the site's own separations
working against it, and items 1–2 are failures nobody has solved.

## 7. Reproducibility apparatus

What is actually published, per result:

- **Per-file digests.** `sourceFootprint.files` — 1,160 entries for Sendov, each
  `{path, bytes, physicalLines, nonEmptyLines, sha256}`. Not a bundle hash: a
  hash per file, so a diff localises.
- **Pins.** `leanToolchain` (`leanprover/lean4:v4.30.0-rc2` on every artifact
  read), `packageCommit` (full 40 hex), `dependencyLockHash`,
  `sourceArtifactHash`, and separately `licenseCommit`.
- **Collector provenance.** `collectedAt`, `collectorCommit` — *the code that ran
  the check is itself version-pinned*. This is the field Sendov lacks.
- **Asset digests** in `data/releases/*.json`: every PDF, ZIP and PNG with
  `byteLength` + `sha256`.
- **Actor digests.** Reviewers, authors and organisations are records with their
  own `sha256`, and `identityMode: "real_name" | "pseudonym"` — a model is a
  pseudonymous actor with a hashed record, so a review binds to a specific
  reviewer *definition*.
- **Review bindings.** Each review has `sha256`, a `subjectBindings` list of
  `{id, sha256}`, a `scope.statement`, and a `scope.limitations` array. "Download
  exact review bindings" is a real link.
- **Replay commands, printed.** Berlekamp: `sha256sum -c MANIFEST.sha256`,
  `./RUN_ALL.sh`, three named Python audits, and `lake env lean
  Domineering/ReachableCounterexampleValue.lean` — with the honest caveat
  *"compilation is a substantial resource job; build products are deliberately
  omitted."*
- **Bundle contents, enumerated**: *"the checked first-party Lean import closure,
  exact statements and boundaries, license, notice, evidence, source-footprint
  manifest, and continuation data. Mathlib and other third-party dependencies are
  not bundled."*

**Gaps.** No published build-resource limits on the site (the bundle's 32 GiB /
1200 s / one worker appear only in `LEAN_AUDIT.txt`, and note that the one
resource fact the site *does* surface is `recordedBuildElapsedMs`). No
Mathlib commit in the site's `reproducibility` block — only a
`dependencyLockHash`, so you need the ZIP to recover it. No container or OS
identity. No re-verification date: `collectedAt` is when it was first collected,
and nothing says the check has been repeated since.

Also worth noting the counts drift between site and bundle: the site records
**1,160 files / 92,816 non-blank lines** (the theorem's import closure), the
bundle ships **1,176 files / 120,999 lines** (the repository). Both are correct;
they answer different questions, and the site says which one it is answering.

## What this would change in this repository

Five proposals. Each names the failure it stops, where it goes, and what it
costs. Ordered by how much it would change what a run produces.

### A. Split `verified` into `checked` and `accepted`

**Failure stopped.** Today a claim reaching `verified` in `derived/CLAIMS.md`
means "the Lean compiled". It does not mean anyone confirmed the Lean statement
is the mathematics we set out to prove — and the Sendov page is a live
demonstration that those come apart at the top-level theorem, which is where it
matters most. Our verdict is read off the kernel, which is right, and then read
as belief, which is not.

**Where.** `src/orchestrator/lean.rs` (verdict), `ledger/registry.rs` (the claim
row), `derived/CLAIMS.md` (rendering). Add a second axis alongside the existing
verdict: `alignment: not_reviewed | accepted | disputed`, defaulting to
`not_reviewed` and settable only by a role that is *not* the one that wrote the
Lean. `lean_prover` writes the proof; the reviewing role reads the statement
against the goal and nothing else.

**Cost.** One enum, one ledger field, one prompt for the reviewing role, and a
rendering change that shows both. Moderate: the loop's routing already reads a
verdict, so `blocked`/`pass` semantics must not shift — alignment is reported,
never routed on, at least at first.

### B. `collector` provenance on every recorded check

**Failure stopped.** Sendov is refused acceptance on exactly this, and we have
the same hole in a worse form: `derived/CLAIMS.md` records that a claim is
verified but not by which Lean, against which Mathlib, over which bytes, run by
which build of the harness. (Noted as gap **c** in `01-…-anatomy.md`; the site
gives the precise field list to build it from.)

**Where.** `src/orchestrator/lean.rs`, where `lean_check` already parses build
output. Record, per checked file, in the claim row: `lean_toolchain`,
`mathlib_commit`, `source_sha256` (per file, not per run), `elapsed_ms`,
`collected_at`, `collector_commit` (this repository's HEAD), and — the field that
makes it a control rather than metadata — **`transcript_recorded: bool`**, false
whenever the record was reconstructed rather than captured from the executed
command.

**Cost.** Low. Everything except `mathlib_commit` is already in reach at the call
site. The rendering must cap, per the ledger rules.

### C. A `provenance` gate on the claim ledger, mirroring their two booleans

**Failure stopped.** The general form of B: our harness treats an agent's report
of a successful check the same as a check the harness ran. A role that says
"`lake build` passed" and a `lean_check` that returned success are currently
indistinguishable downstream.

**Where.** `ledger/registry.rs`. Every evidence-bearing row carries
`evidence_origin: collected | supplied`, where `collected` is settable **only by
the tool that executed the command**, never by a model writing an event.
`derived/CLAIMS.md` renders `supplied` evidence in its own section with a count,
and the `verify` ranking prefers a `supplied` row for re-collection.

**Cost.** Low-to-moderate, and it is the highest-leverage item here because it is
a boundary the write path can enforce rather than a field a model fills in. It
generalises the existing rule that derived files refuse hand edits.

### D. Name the trust debt, count it, and file the work item that retires it

**Failure stopped.** We refuse `native_decide` by test, which is correct and
which also means a run that could only close a gap that way must throw the result
away silently. ProofAtlas publishes such a result with two named axioms, two
independent replays, a "not described as kernel-clean" line, and an open route to
remove it — the debt is visible and someone can pay it.

**Where.** Keep the refusal. Add: when `lean_check` sees a non-standard axiom in
`#print axioms`, it does not merely fail — it files a `reductions`-style row
naming the axiom, the declaration that depends on it, and the replacement
required (`kernel-checkable certificate | independent replay | verifier lemma`).
`derived/LEMMAS.md` grows a **disclosed trust dependencies** section that is
empty in the healthy case. Location: `lemmas.rs`, `lean.rs`.

**Cost.** Low. It is a new row on an existing failure path, and it converts a
dead end into a queued task, which is what our loop is short of.

### E. A composition guard on every ledger count we render

**Failure stopped.** Our `FRONTIER.md`, `BACKWARD.md` and attempt counts are read
as progress by the judge and by us. ProofAtlas prints, next to every number, what
the number is not — *"this is not a measure of closeness to a proof"*, *"more
nodes or edges do not establish correctness or completion"*, *"source footprint,
not a proof-quality or difficulty score"* — and breaks its volume down by kind so
an 88%-prose workspace is visibly 88% prose.

**Where.** `ledger/` rendering. Each capped section already says what it left
out; add what the count *means*, and where a ledger is a mixture, its composition
(established / reported / open / eliminated). Cheapest real version: report
`identity` and `reductions` rows split by whether anything downstream reads them,
the way `LEMMAS.md` already reports generated data no checker consumes.

**Cost.** Low, and mostly copy. The honest caveat: this is a rendering change,
which is nearer a prompt instruction than a control — its value is that the
*judge* reads the rendering, so a number it cannot mistake for progress is a
number it cannot be fooled by.

**Deliberately not proposed.** An AI review lane in the shape of their four
questions. Our `judge` already scores an attempt, and adding a second model
opinion between "the Lean compiled" and "we believe it" would buy the appearance
of independence without the substance — theirs is `independenceStatus:
not_asserted` for every review on the site, and they were right to record that.
Proposal A gets the useful half (statement-versus-goal is a real question a
separate role can answer) without pretending the answer is independent.
