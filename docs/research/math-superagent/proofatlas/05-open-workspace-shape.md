# ProofAtlas' open workspaces, read as a queue

Source: the public `proofatlas.ai/collaboration/` tree, fetched 2026-08-17. Five
pages read in full as HTML rather than through a summariser — the four named
below plus the index. Every quotation is literal.

| Page | Retained lines | Statements | Routes | Obligations | Sources |
| --- | ---: | ---: | ---: | ---: | ---: |
| Hilbert 16 | 1,114 | 8 | 2 | 4 | 5 |
| Agoh–Giuga | 3,846 | 16 | 11 | 8 | 8 |
| Dürer edge-unfolding | 3,879 | 23 | 11 | 7 | 9 |
| Gallai path decomposition | 5,652 | 23 | 11 | 7 | 18 |
| Index (85 workspaces) | 271,452 | 1,239 | — | 309 | 920 |

Note 01 read a *finished* bundle. This note reads the other end: 85 problems
nobody has solved, in a runtime whose whole visible product is the queue. That
makes it the closer analogue of what `./conjecture <slug>` produces, and the
comparison is unflattering in three specific places.

One structural fact first, because it is the design: **the workspace page is the
index, and there is no body to click into.** Every `#`-anchor on those pages is
same-page; there is no link to a route record, an obligation detail, a note, or
a proof. 5,652 lines of Gallai investigation compress to about 40 kB of text
with 23 statements and 7 tasks, and the rest is not published at all. A run
joining that workspace reads one page. Our `read_ledger` fetches the rest; theirs
does not exist publicly. Both are the same bet — the index is the product — and
they push it further than we do.

## What a task obligation actually is

The DOM carries two id spaces. The anchor a human links to is
`task-obligation.<packet>.<slug>.<vNNN>`; the same entity appears in the
dependency graph as node `obligation.<packet>.<slug>.<vNNN>`. Four sibling
namespaces share the scheme:

```
conjecture                                     (one per workspace)
claim.preview-durer.strict-reciprocal-reduction.v001
route_record.preview-durer.reciprocal-gap.v001
obligation.preview-durer.strict-reciprocal-gap.v001
failed_route.preview-durer.supporting-shadow.v001
change.preview-durer.sixth-pass.v002
```

Every node carries `data-node-kind` ∈ {`conjecture`, `supporting_claim`,
`reduction`, `open_obligation`, `failed_route`} and `data-node-status` ∈
{`active`, `open`, `stopped`, `blocked`} — 56/23/14/3 respectively across the
four pages. Edges are `data-source`/`data-target` pairs, and the rendered
relation labels are `supports`, `equivalent to`, `specializes`, `challenges`,
each suffixed **`· reported by source`**.

The visible obligation fields:

| Field | Example (Agoh–Giuga #03) |
| --- | --- |
| id | `task-obligation.preview-agoh-giuga.replay-enumerations.v001` |
| title | "Replay the large A = 71 enumerations" |
| statement | "Reproduce the 10⁹, 2×10⁹, and 4×10⁹ candidate counts and upward-rounded reciprocal checksums from the exact common predicate." |
| suggested move | "Run the retained segmented implementation with the stated exact factorization range and directed rounding, then independently check all three rows." |
| status | `Ready to work on` \| `Prerequisites still open` \| `Blocked by the current route` |
| priority | `critical priority` (only on the one featured task) |
| acceptance | "What would count as progress" — 2–3 bullets, **only rendered for the featured task** |
| parent route | via graph edge `route_record.… → obligation.…` |

Two of those are things our `tasks` ledger has no field for. The **suggested
move** is a method hint separate from the goal — "Treat the active saturation
quadratics as norm or resultant objects", "do not require a basis exchange",
"Study actual prime divisors of 71P²−71P+1 rather than fixed CRT-compatible
local shadows". The **acceptance criteria** are stated before the work, as
artifacts:

> Every extracted cell has an unambiguous edge set and does not double-count edges.
> Each cell has an explicit ideal cost, protected endpoint set, and allowed defect locations.
> A proved zero-overhead or bounded-defect theorem covers every cell produced by the extraction.

and for Hilbert 16, tellingly:

> Supply a complete argument with every imported premise identified.
> Survive an independent attempt to falsify the proposed step.

**Versioning.** `.v001` is the birth version. A bump means the *mathematics*
changed under a stable identity: Dürer carries `claim.…four-interior-eight-harmless.v002`
and `obligation.…eight-four-interior-types.v002` beside eleven `.v001` siblings,
and the reason is a separate record, `change.preview-durer.sixth-pass.v002` —
"Sixth-pass corrections and four-interior frontier … closes the octahedral
layer, preserves four scoped retractions or refutations, and reports a
five-graph, sixteen-type four-interior census with eight harmless and eight
unresolved types." The page states the exclusion rule explicitly: "Uploads,
model runs, and presentation changes do not count as mathematical updates."

**Who may close one, and on what evidence: the pages do not say.** The console
reads "Read-only beta · actions unavailable" and lists a return channel — "A
proof attempt, partial advance, counterexample, useful failure, or corrected
dependency can all move the shared frontier forward" — with three submission
routes (work it yourself, hosted agent, connect your own agent) and an identity
line ("Name, organization, agent ownership, and previous contributions stay
attached to the work"). Nothing states an adjudicator, a merge policy, or what
happens when two contributors return work on one obligation. Given that
`/research/` credits Lech Mazur by name on all three papers, adjudication looks
editorial, but that is inference, not something the site says.

## How a workspace records what has been ruled out

This is the part worth stealing wholesale. Dead work is not one flag; it is a
**seven-value route disposition** plus a **separate anti-pattern entity**.

| Disposition | Meaning as used | Instance |
| --- | --- | --- |
| `Active route` | live | "Exact reciprocal strict-gap search" |
| `Narrowed route` | survives in a restricted form, with the restriction named | "Degree-three reciprocal recursion — the local affine-boundary star discharge survives only as leaf refinement. Arbitrary-depth recursive deletion remains unavailable until external-child margins are controlled." |
| `Eliminated route` | closed for a stated structural reason | "Fixed finite local accumulation — Adding finitely many terminal faces, 2-adic digits, F/W checks, or terminal-overlap identities cannot close A=71." |
| `Refuted route` | killed by an exhibited counterexample | "Convex supporting-edge shadow — The convex-block shadow theorem is refuted. Only the stronger sliced dual-shadow condition or the affine-boundary patch theorem remains viable." |
| `Useful but insufficient` | true, does not conclude | "Standalone rank-minor contradiction — Vanishing 4×4 minors and the rank-three kernel describe genuine terminal structure; without global aggregation they do not contradict it." |
| `Route held in reserve` | parked with a revival condition | "Conditional dense alternate routes … should be pursued only if the universal defect route stalls." |
| `Not yet justified` | eliminated on one instance, general case open | "Pointwise failure to common mass obstruction — eliminated on the 26-vertex minimal conflict because adaptive networks expose the quantifier gap; future negative work must search the actual fixed-mass forest-cone union directly." |

Two of these have no analogue anywhere in our runtime. `Narrowed` records the
*surviving fragment* of a dead route; `held in reserve` records the *condition
under which to revive it*. Our `approaches` stances (`proposed`, `grounded`,
`refuted`, `adopted`, `spent`) collapse both into `refuted`, which loses the
salvage and guarantees the salvage is re-derived.

Beside the routes sits a second entity: the **useful failure**, one imperative
sentence describing a *move an agent would make*, not an idea it would have. 37
across the four pages. Verbatim:

```
Infer closure from checking finitely many terminal coefficients A.
Treat derivative completion as full completion without checking denominator overlap.
Use vanishing rank-three minors as a standalone contradiction.
Use enormous support or largest-prime lower bounds as a contradiction.
Use only a qualitative O(x/(log x)^(3/2)) estimate.
Repair any legal packet return by one cycle exchange.
Purify packet evacuation by always moving toward increasing lifting height.
Optimize raw hinge-scaled forest margins without strict hinge controls.
Retain floating MILP infeasibility or an unchecked floating forest cover as exact evidence.
```

They carry their own posture, `reported failure` (37) or `witness reproduced`
(2), and are attached to clusters rather than to one route, so the same failure
appears under every cluster it threatens — "Repair any legal packet return by
one cycle exchange" is listed twice on the Dürer page, once under the positive
program and once under the retracted-routes cluster. Deliberate redundancy: the
agent reading a cluster sees the failure that cluster invites.

And the **Challenge** record, which is a live objection against a step rather
than against a route, typed by failure mode:

```
unsupported step · open            (4)
unsupported step · reported resolved
counterexample · reported resolved (3)
quantifier error · reported resolved
```

> Pointwise failure has quantifiers ∀F∃(v,u), while a common obstruction needs
> ∃α∀F. Two adaptive networks cover all ratios on the minimal four-source
> conflict. — *quantifier error · reported resolved*

A named quantifier-order error, kept with the computation that exposed it, after
it was fixed. Our runtime has no place to put that; the closest is a `refuted`
approach, which does not say *which kind of wrong* it was.

## The state-of-play card, and the honest denominator

Every workspace opens with the same fixed-arity card. It is the answer to "how
does a fresh agent join without re-reading everything":

| Slot | Hilbert 16's value |
| --- | --- |
| Strongest result so far | "Two distinct classical parts" · *Reported lemma* |
| Leading route / Main reduction | "A(n) ≤ H(n) ≤ n²A(n)" · *Reported reduction* |
| Useful failure | "Unrestricted inverse-flow gauge flattening" · *Eliminated route* |
| Evidence foothold | *(absent on this page)* |
| Completed special case | *(absent on this page)* |
| Current obstacle | "Independently review the source-reported fixed-degree Part-I algorithm…" · *Ready to work on* |

Slots are omitted when empty rather than filled with prose — Dürer and Gallai
show `Completed special case` and `Evidence foothold`, Hilbert 16 shows neither,
and the absence reads correctly as "this workspace has banked nothing yet."

Then the denominator, which is the honest part:

```
3.8k retained lines of mathematical investigation
  Argument development        3,308 · 86%
  Explored or eliminated routes 144 ·  4%
  Computational analysis        133 ·  3%
  Open obligations              134 ·  3%
  Definitions and setup         127 ·  3%
```

with, immediately below: "This measures retained mathematical investigation, not
proximity to a proof. Code, data, logs, repeated text, operational instructions,
and generated presentation copy are excluded." The site never converts activity
into confidence, and says so at every point it renders a number — under the
route map ("more nodes or edges do not establish correctness or completion"),
under the statement census ("These counts describe the work's structure; they do
not estimate distance to a proof"), and on the index ("A view across selected
research maps, not a proof-completion score"). Four disclaimers of the same shape
in one page is a policy, not a lawyer.

## The reproduction ladder

Computations carry a three-rung posture, and this is the sharpest single control
on the site:

| Rung | Count | Meaning |
| --- | ---: | --- |
| `reported unreproduced` | 4 | the source says so; nobody reran it |
| `reproduced same implementation` | 4 | the retained code was rerun and agrees |
| `independently reimplemented` | 2 | a second, differently-written program agrees |

> The v11 path-mask exact-cover implementation and v12 rollback-DSU
> edge-coloring implementation agree that the defect is feasible exactly at x and
> the fourteen rung vertices, and infeasible at c,a,b,v,s.
> — *independently reimplemented*

> The current work reports counts 4,564,232; 8,657,199; and 16,453,222 and
> corresponding scaled reciprocal upper sums, but none of these large runs was
> replayed during assembly or source material. — *reported unreproduced*

Two consequences the site actually enforces. First, an unreproduced number
**poisons everything downstream**: the Agoh–Giuga derivation that uses it is
marked `challenged`, and a Challenge record says so — "so their counts and
reciprocal checksums remain reported inputs rather than reproduced evidence."
Derivation status is its own four-value field: `active reported` (8),
`challenged` (2), `invalidated` (1), `proposed` (1). Second, **replay is a
first-class obligation**: `task-obligation.preview-agoh-giuga.replay-enumerations.v001`
exists for no other purpose than to move three numbers up one rung, and the
index promotes it as one of three recommended starting points, at `critical
priority`. A run whose next best move is "rerun your own arithmetic" is allowed
to say so.

Dürer #05 is the same instinct aimed forward — "Build an independently checkable
reciprocal engine … Implement an exact forest enumerator and independent verifier
before trusting any new cover or strict-gap result" — and Gallai #04 asks for "an
implementation-independent compact certificate" before the census is believed.
This is the certificate/checker separation from note 01, arriving at the queue
level as a schedulable task.

## Provenance, and what the evidence does not cover

Every statement is stamped with who asserted it: `Reported result`,
`Reported lemma`, `Reported reduction`, `Reported computational claim`,
`Reported negative result`, `Reported theorem candidate`, against
`Manually checked` (5 on Agoh–Giuga, 16 on Gallai) and `Computation reproduced`.
Prose keeps the same discipline — "The source constructs the transported
connection", "The current work reports and reproduces exactly 64 reduced residue
classes". Nothing is ever stated in the platform's own voice without a posture.

And each page closes with **Important qualifications**, which is note 01's
missing `does-not-cover` field, at workspace scope:

> This was a scoped authoritative-source search, not a systematic review of every
> low-degree curve classification or limit-cycle lower bound.
> No public proof-assistant formalization … was verified. The empty formalization
> list means none was found in scope, not that none exists.
> The current work's fixed-degree algorithm, same-core reduction,
> relative-Schwarzian identities, and work orders were not independently proved,
> executed, or reproduced during metadata collection.

An empty list stating it is an empty *search*, not an empty *world*. We have
nothing that does this, and the failure it stops is a run reading its own
`FRONTIER.md` as a complete map of the literature.

## Partially resolved, open, published

The index bar reads **59 Open · 11 Recent claim monitored · 14 Partially
resolved · 1 Finite check remains** (85 total; the brief's "15" is off by one,
and the categories are as of the 2026-08-13 external status check). The load-
bearing sentence is right above it:

> This chart tracks sourced external status. Agent-developed routes and updates
> appear separately inside each workspace.

So "partially resolved" is a fact about the **literature**, not about ProofAtlas'
progress — Sendov sits under `Partially resolved` while its own workspace card
still reads "Current obstacle · next task: Audit the exact latest-route core",
and the Sendov proof candidate lives in a *different tree* entirely. Two status
axes, never merged: what the world knows, and what this run has done. Our
`CLAIMS.md` mixes them — a cited literature result and a run-established result
are both rows with an evidence word.

Publication is a third tree, `/research/`, with three items and its own
vocabulary: `Partially formalized`, `Formalization planned`, and version rows
(`Latest listed version` = Sendov Revision 16, `Earlier listed version` =
Revision 14, both retained). Its header states the boundary: "Each page keeps its
formalization status explicit: publication or editorial review is not a
substitute for a Lean-checked proof." The promotion workspace → paper is a hard
edge, and a superseded revision is not deleted.

## Parallelism: what the site shows, and what it does not

Direct answer: **there is no visible locking, no route assignment, and no
concurrency control.** The only mechanisms observable are:

1. **The obligation set is pre-partitioned by route.** Each `route_record` owns
   its obligations through graph edges, and the routes are chosen to be
   different attacks — Agoh–Giuga's seven active routes range over a scalar
   matching equation, a sieve, a factorization argument, a determinant argument
   and a replay. Two agents on two obligations are on two routes by construction.
2. **Status gates how many are workable at once.** Of 26 obligations across the
   four pages, `Ready to work on` is the majority but `Prerequisites still open`
   and `Blocked by the current route` hold back a fifth of them. Gallai's
   matching-deficiency task is blocked with the dependency named: "After
   canonical cell extraction is fixed, translate arm incompatibilities into
   directed double-shadow dependencies."
3. **One task is featured at `critical priority`** per workspace, and the index
   recommends exactly three across 85 workspaces. Diversity is not enforced;
   convergence is *encouraged*, which is the opposite of our schools design.

Nothing on any page says what happens when two returns arrive for one
obligation, whether an obligation can be claimed, or whether a hosted agent and
an outside agent can hold the same task. The console is read-only in beta, so
that machinery may simply not be public yet. **This is the largest gap in what I
could read**, and the one place our runtime is plainly ahead: `worklock.rs`, the
per-candidate worktrees, and `adopt_attempt` are answers to a question this site
does not visibly ask.

Two further silences worth recording. There is **no cost, wall-clock, model, or
token figure anywhere** — nothing says what 271,452 lines took. And there is **no
Lean artifact in any collaboration workspace**; Hilbert 16 lists three
"Formalization targets" as aspirations, and the qualifications say no
formalization was found in scope. The formalisation ambition is stated and
unmet, which is exactly the gap our `lean_prover`/`lean_scribe` pair exists to
close.

## What this would change in this repository

Ordered by how much it changes what a run leaves behind.

**1. `disposition` on the approaches ledger — seven values, not two.**
*Failure stopped:* a route that was 80% right is filed `refuted`, the surviving
fragment is lost, and a later attempt re-derives it. Also: a route parked for a
good reason is never revived, because nothing recorded the condition.
*Where:* `src/orchestrator/approaches.rs` — add `narrowed`, `insufficient` and
`reserved` to the stance life cycle, with a required `survives-as` field on
`narrowed` and a required `revive-when` on `reserved` (same shape as `thesis`'s
mandatory `refuted-by`). `APPROACHES.md` grows a "Narrowed — what survives"
section beside the existing dead-route section.
*Cost:* one enum, two required fields, one derived section, and the ceiling test.

**2. A `failure` ledger: anti-patterns as imperatives, not as ideas.**
*Failure stopped:* our dead-route records name *ideas* ("the generating-function
reformulation"), so a role about to make the same *move* in a new dress does not
recognise it. ProofAtlas' 37 useful failures are all of the form *verb the
object without the guard*, which is recognisable at the point of temptation.
*Where:* a new registry entry beside `reductions`, one line per failure, with
`posture` ∈ {`reported`, `witness-reproduced`} and a `cluster` field so the same
failure can index under several threads. It reaches a prompt as index lines, and
`witness-reproduced` requires a `code/out/` artifact.
*Cost:* one `LedgerSpec`; the routing surface does not change.

**3. A `reproduction` posture on every computational claim.**
*Failure stopped:* `CLAIMS.md`'s `checked` does not distinguish "we reran our own
script" from "a second implementation agrees", so a run's own enumeration bug is
invisible and its consequences are filed as established. This is note 01's
mutation-control gap, generalised and cheaper.
*Where:* `src/orchestrator/claims.rs` — a `reproduction:` line taking
`reported-unreproduced` / `same-implementation` / `independent-reimplementation`,
required whenever the note lives under `code/out/`. Any claim resting on a
`reported-unreproduced` input inherits a `challenged` marker in `ENTAILMENT.md`,
which is the propagation ProofAtlas does by hand.
*Cost:* one field, one propagation pass over the existing entailment graph.

**4. Replay as a schedulable task, not a virtue.**
*Failure stopped:* a run never proposes "rerun the thing you already ran" because
no verdict rewards it, so unreproduced numbers accumulate and then carry a
conclusion. `replay-enumerations.v001` shows the fix: make it an obligation, and
let it be the *top-priority* one.
*Where:* `goals.rs` emits a `tasks` row automatically whenever a claim with
`reproduction: reported-unreproduced` is cited by another claim. The rule is
mechanical, so it is a control rather than an instruction.
*Cost:* one derivation rule; no new ledger.

**5. `suggested-move` and `done-when` on the tasks ledger.**
*Failure stopped:* our `TASKS.md` rows state a goal and nothing about method or
acceptance, so a role reads a task, does something adjacent, and the judge has no
stated test to score against. ProofAtlas states both before work starts, and its
Hilbert 16 acceptance bullets — "every imported premise identified", "survive an
independent attempt to falsify" — are exactly the two things our attempts most
often skip.
*Where:* `registry.rs`, `tasks` spec: two optional fields, `done-when` required
when a task is created by the judge.
*Cost:* two fields; index lines grow by one clause.

**6. A `superseded` record with a reason, and version-bumped ids.**
*Failure stopped:* our ledgers merge by id, so a corrected claim silently
overwrites the wrong one and the run loses the fact that it *was* wrong — which
is precisely the information that stops the next attempt walking back into it.
Dürer's stage list is half retractions: "Distinct 26-vertex cap interpretation
withdrawn", "Convex supporting-edge shadow theorem retracted", "Leaf refinement
replaces unrestricted degree-three deletion".
*Where:* claims and approaches take an optional `supersedes:` id plus a required
`why:`; the derived file keeps the superseded row with a strikethrough and the
reason. Do **not** copy their `.vNNN` suffix — our ids are already stable and a
suffix would break `read_ledger { id: … }`.
*Cost:* one field pair, one rendering change.

**7. A fixed-arity state-of-play card at the head of every derived tree.**
*Failure stopped:* a fresh role — or a fresh school — reads `TASKS.md`,
`CLAIMS.md` and `FRONTIER.md` and still cannot say in one sentence what the
strongest thing established is or what the single obstacle is. Six slots, each
either filled with one line plus a posture or *omitted*, and omission is the
signal.
*Where:* a new `derived/STANDING.md` derived from the existing ledgers — nothing
new is written, it is a projection. Slots: strongest result, main reduction,
leading approach, useful failure, evidence foothold, current obstacle.
*Cost:* one derivation module reading four existing ledgers; no new writer.

**8. A `scope-not-covered` block on the library.**
*Failure stopped:* `FRONTIER.md` reads as a complete map of the literature when
it is a map of what the run happened to download. ProofAtlas states this in the
same breath as its results.
*Where:* `LIBRARY-STATUS.md` gains a required "what this search did not cover"
section, derived from the search queries actually issued versus the subject areas
named in `problem.md`.
*Cost:* small; the queries are already logged.

Three of their controls I would **not** copy. The percentage breakdown of
retained lines is a vanity metric with a disclaimer attached, and we would end up
optimising it. Featuring one `critical priority` task per workspace pushes every
contributor onto one route, which is the opposite of the schools bet and would
undo `MATH_AGENT_SCHOOLS`. And the `.vNNN` id suffix costs id stability for
information a `supersedes` field carries better.

---

# Hilbert 16: intelligence for the live run

`workspace/conjectures/hilbert-16` is attacking **Part II** through the
Roussarie reduction and the DRR 121-graphics program. ProofAtlas' Hilbert 16
workspace is the *youngest* of the 85 I sampled — 1,114 retained lines, 8
statements, 2 routes, 4 obligations, 5 sources, no `route_record` nodes and no
`change` stages at all, status checked 2026-08-13 — and it attacks something
else. Read it as one independent route plus two named negative results, not as
competition.

**It never mentions DRR, Roussarie, Écalle, Ilyashenko, Dulac, Bautin ideals,
Abelian integrals, or finite cyclicity.** Our library is far deeper on Part II's
actual state of the art. Their five sources are Hilbert 1902, Encyclopedia of
Mathematics, Viro 1986, Gasull–Santana 2024, Gasull 2024.

### 1. Their main reduction, which we do not have

> For each field X, let A(X) be its largest same-core stack height, and define
> A(n)=sup_{deg X≤n} A(X). The source reports **A(n) ≤ H(n) ≤ n²A(n)**, so
> finiteness of the Hilbert number is equivalent to a uniform degree-only bound
> on same-core stacks. The supremum A(n) is not asserted to be attained.
> — *Evidence posture · Reported reduction*

Marked source-reported and **unaudited**; their own obligation #02 is to audit
it — "Reconstruct the residual-core ownership argument, test all edge conventions
such as empty cores and singularities at infinity, and verify the inequality
A(n) ≤ H(n) ≤ n²A(n) from the exact definitions."

I searched our workspace: **"same-core" appears nowhere in
`workspace/conjectures/hilbert-16/research/`** (one incidental hit for
"Schwarzian" in the Llibre survey, an unrelated nonsmooth-systems paper title).
This is either a real reduction our librarian missed or a term the source
invented. Either answer is worth one librarian pass, and the shape is exactly
what `derived/REDUCTIONS.md` was built for: one named scalar `A(n)`, a lower
bound and an upper bound as separate fields, gap `n²`.

### 2. Their strongest negative result is our METHOD's stated failure mode, quantified

> **Omitting finite-cover multiplicity from a local-to-global estimate.** For a
> finite subcover W₁,…,W_ν, the source obtains a sum over all neighborhoods. With
> local bounds b(n) and B(n)+3, this gives H(n) ≤ ν(n)·b(n)·(B(n)+3);
> **compactness alone does not prove ν(n)=1.** Retain the finite-cover
> multiplicity ν(n), or prove a genuinely global single-atlas theorem by a
> separate argument. — *Narrowed route*

`METHOD.md` already warns: "Any step that quantifies over a compact set of
parameters and concludes a uniform bound from pointwise finiteness is the error
to watch for." Their ν(n) is the concrete arithmetic form of that warning, and it
sharpens our own frame: **the DRR list of 121 graphics is precisely an explicit
finite atlas for n = 2** — DRR is what "prove ν(2) is finite and enumerate it"
looks like when it is done properly. That reframing is worth filing, because it
says what a general-n attack would owe: not a compactness argument, but a
degree-uniform bound on the atlas size.

File as an approach with disposition `narrowed`, `survives-as: "H(n) ≤ ν(n)·b(n)·(B(n)+3) with ν(n) retained"`.

### 3. Their other negative result: gauge flattening trivialises

> **Unrestricted inverse-flow gauge flattening.** The source constructs the
> transported connection explicitly and obtains **φ(t,y)=φ₀(Φₜ⁻¹(y))**, so every
> trajectory becomes constant in the transformed coordinate and the equation is
> **u′=0**.

The gauge that flattens the flow proves nothing, because it removed the content.
They keep the route alive under four explicit conditions — "provided it is
noncircular, survives parameter degeneration, closes around the full return
loop, and is combined with the separate nonsingular localization gate" — which is
a textbook `reserved` / `revive-when` entry.

Directly relevant to us: any normalisation our `inventor` proposes for the return
map is exposed to the same collapse. The transferable test is a *noncircularity
check on a normalisation* — after the change of coordinates, does the
displacement function still have the zeros we are trying to count? Worth adding
as a `failure` row the moment control 2 above exists.

### 4. Their open target, stated precisely

> **Controlled closing gauge.** Construct a normalized noncircular admissible
> gauge for a closed return loop and prove a uniform zero bound for its exact
> **closed relative Schwarzian**, beginning with one saddle-node passage and
> regular connector.

and the gating obligation:

> Prove the one-corner saddle-node closed-gauge zero bound and, **independently**,
> classify nonsingular Hausdorff limits into a finite return atlas or a
> finite-cyclicity period annulus.

Two independent gates, both required before any conditional architecture implies
a uniform bound. The second gate — classifying nonsingular Hausdorff limits into
a finite return atlas — is the *same object* as the DRR limit-periodic-set
classification, approached without the DRR vocabulary. If the run wants a
cross-check on its own framing, this is it: an independent source arriving at
"you need a finite atlas of limit objects" from a different direction.

### 5. A source we do not hold

**Gasull & Santana, "A note on Hilbert 16th problem", Proc. Amer. Math. Soc.
(2024), doi 10.1090/proc/17116, arXiv 2407.13465** — "proved that H(n), whenever
finite, is realized by a structurally stable vector field with hyperbolic cycles
and is strictly increasing." *Peer reviewed.*

`grep -ril "Santana\|2407.13465"` over our `research/` returns nothing. Two
reasons this matters: the realisation statement reduces the extremal problem to
counting hyperbolic cycles of structurally stable fields, which is a much better
behaved class; and strict monotonicity of H(n) is a constraint any lower-bound
row in our library must respect. Also missing: **Gasull, "From Abel's
differential equations to Hilbert's 16th problem", São Paulo J. Math. Sci.
(2024), doi 10.1007/s40863-024-00471-2** — a 2024 survey.

Recommended: `download_document` both, then let `FRONTIER.md` re-derive.

### 6. Where their page is weaker than our workspace, and it matters

Their "Current status" prose asserts flatly that "each individual polynomial
vector field has finitely many limit cycles." Our `derived/CLAIMS.md` carries
`h16-dulac-proof-contested`, `h16-dulac-reopened-community-view` and
`h16-gap-claims-2024` — Yeung, "Dulac's Theorem Revisited", *Qual. Theory Dyn.
Syst.* 24 (2025) Art. 57, doi 10.1007/s12346-025-01220-2, is a **peer-reviewed**
gap claim against Ilyashenko's argument. ProofAtlas' context collection missed
it, and their own qualifications block predicts exactly this: "This was a scoped
authoritative-source search, not a systematic review."

The practical read for the live run: on Part II's literature we are ahead and
should not treat their page as a check on ours. Take from it the reduction in §1,
the two negative results in §2–3, and the two missing sources in §5.

### 7. What their page does not say

No Lean, no code, no data, no certificates — the workspace lists three
"Formalization targets" as aspirations and its qualifications record that no
formalization was verified in scope. No numbers for any computation. No account
of how many agents worked it or for how long. Nothing on Part I beyond
"discriminant-chamber enumeration", which their obligation #01 explicitly refuses
to upgrade — "Independently review the source-reported fixed-degree Part-I
algorithm **without** upgrading it to a transparent all-degree classification."
That refusal is itself a good habit and matches our `problem.md` decision to hold
Part I out of scope in this pass.
