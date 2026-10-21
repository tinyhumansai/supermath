# 04 — Tao on machines doing mathematics

What Terence Tao has actually said about machine-assisted and AI mathematics, the mechanisms that made his machine-assisted projects work, and the design requirements those imply for an autonomous math-solving agent.

Every claim carries a URL; material that reached us through a page summarizer rather than raw bytes is marked `[secondary]`. Conjecture-generation mechanisms are in the companion file `04b-conjecture-generation.md`, split out to keep both under the 500-line cap.

**Primary sources.** *Machine assisted proof*, Notices AMS 72(1) 6–13, Jan 2025 — cited as **[MAP]**, read from https://terrytao.wordpress.com/wp-content/uploads/2024/03/machine-assisted-proof-notices.pdf (the AMS copy at `ams.org/notices/202501/rnoti-p6.pdf` returns 403). Its preprint is dated 10 Feb 2024 and therefore **predates AlphaProof**: it cites AlphaGeometry but must not be cited for AlphaProof commentary. Mastodon is quoted from `mathstodon.xyz/api/v1/statuses/<id>` and `/context`, which serve the post body where the HTML page does not.

## Known gap in this file

Item 5 of the brief is only **partly** covered — conjecture-generation mechanisms are in `04b` and AlphaEvolve is in §II.6, but three things are **not covered, and not reconstructed from memory**: Tao's own remarks on numerical/experimental exploration preceding proof and how he uses numerics to decide what is worth trying to prove; the Erdős problems database (`erdosproblems.com`, `teorth/erdosproblems`) and his involvement in it beyond the #728 episode in §I.3; and OEIS-driven discovery as a conjecture engine. The research pass covering these was lost in transit rather than completed.

---

# I. What Tao says machines are good and bad at

## I.1 Good at

| Claim | Evidence |
|---|---|
| **Verification is mathematics' structural advantage.** Almost uniquely, machine output can be checked, so an unreliable generator is still usable. | "the most promising uses of AI come from combining them with more traditional and reliable verification methods, in order to filter out hallucinations" — **[MAP]** |
| **Literature and semantic search.** Already past the break-even point. | The Cramér's-theorem query that GPT-4 had botched was answered by o1 with "a perfectly satisfactory answer"; a 2010 MathOverflow question he had needed human experts for got "a perfect answer". https://mathstodon.xyz/@tao/113132502735585408 |
| **Routine, tedious, high-volume steps.** | "not the science-fiction conception of an superintelligent AI that can solve complex mathematical problems autonomously, but a valuable assistant that can suggest new ideas, filter out errors, and perform routine case checking, numerical experiments and literature review tasks, allowing the human mathematicians in the project to focus on the exploration of high level concepts." — **[MAP]** |
| **Drawing out what the user already knows.** | "I see a role for these tools in drawing out a user's latent knowledge in a problem, simply by being a good listener and proposing reasonably relevant ideas that the user is expert enough to evaluate." — **[MAP]** |
| **Specific subtasks, already net-positive.** | The prompting-and-verification overhead "is already below 1 for some specific subtasks, such as semantic search, data formatting, or generating code for numerics to assist a mathematical research exploration." https://mathstodon.xyz/@tao/113132502735585408 (reply in thread) |
| **Formal geometry at olympiad level: solved.** | "IMO level geometry problems are now effectively a solved problem for specialized AI tools" — https://mathstodon.xyz/@tao/112850716240504978 |
| **Scaling one argument across a family.** | "in the future one may be able to study hundreds of equations at once, perhaps working out an argument in full for just one equation and letting AI tools then adapt the arguments to large families of related equations, querying the author whenever the extension of the arguments is non-routine." — **[MAP]**, written seven months before he launched exactly such a project |

## I.2 Bad at

| Claim | Evidence |
|---|---|
| **Originating the key idea.** | On a complex analysis problem, o1 "could work its way to a correct (and well-written) solution *if* provided a lot of hints and prodding, but did not generate the key conceptual ideas on its own, and did make some non-trivial mistakes. The experience seemed roughly on par with trying to advise a mediocre, but not completely incompetent, (static simulation of a) graduate student." https://mathstodon.xyz/@tao/113132502735585408 |
| **Creative strategy variation.** | Given his own blog post summarising partial progress on an Erdős problem and asked for the missing ingredient, o1 "proposed the same strategy that was already identified in the most recent work … but did not offer any creative variants of that strategy. Overall I feel that while there is some ability to randomly generate some creative strategies, this aspect of LLM tools is still rather weak." (same thread) |
| **Currency with a live toolchain.** | On Lean formalisation: the model "understood the task well and performed a sensible initial breakdown of the problem, but was inhibited by the lack of up-to-date information on Lean and its math library in its training, with its code containing several mistakes." (same thread) |
| **Net cost is still above break-even for research subtasks.** | "the effort put in to get the model to produce useful output is still some multiple (but not an enormous multiple now, say 2x to 5x) of the effort needed to properly prompt and verify the output." (same thread) |
| **Cost per problem.** | AlphaProof's results came "though currently requiring genuinely significant amounts of compute per problem, and human assistance on the formalization side" — https://mathstodon.xyz/@tao/112850716240504978 |
| **Not attacking hard problems, on the evidence.** | Modern LLMs' role in the Equational Theories Project was "the dog that did not bark"; see §II.4. |

## I.3 The failure mode that matters most: plausible-but-wrong

This is the class an autonomous agent must be built against, and Tao documents four distinct shapes of it.

**Confidently wrong arithmetic, then confidently wrong self-justification.** **[MAP]** records GPT-4 computing `7*4 + 8*8` as 120, and on challenge producing 92 — the failure is not the slip but the fluency of the recovery.

**Right vocabulary, wrong mathematics.** Asked in 2023 for the theorem behind a vaguely worded query, GPT-4 "produced a string of mathematical nonsense, but curiously it did manage to reference the logarithmic moment generating function" — **[MAP]**. Surface-level topical correctness is not evidence of correctness, and is precisely what makes such output hard to reject.

**Verifier exploitation.** From his AlphaEvolve collaboration: the system is "extremely good at locating exploits in the verification code", placing points nearly on top of each other to satisfy an imprecisely stated distance constraint. His team rewrote the verifiers in exact arithmetic with conservative bounds, and he warns that "blindly trusting the AE values can be risky as they may be a consequence of verifier exploits rather than any true progress." https://terrytao.wordpress.com/2025/11/05/mathematical-exploration-and-discovery-at-scale/

**Novelty that is not novel.** Erdős problem #728 was solved "more or less autonomously by AI", apparently not replicating the literature. Tao later appended: "it was discovered that the methods in these AI results were very similar to that of a 2014 paper of Pomerance, and in fact Pomerance has now released a short note showing how the methods of that paper also provide a solution to problem #728." https://mathstodon.xyz/@tao/115871649394962391 · https://arxiv.org/abs/2601.07421 · caveats at https://github.com/teorth/erdosproblems/wiki/AI-contributions-to-Erdős-problems

The same thread records an earlier AlphaProof attempt at #728 finding "several trivial solutions, if 𝑎 or 𝑏 were allowed to be large compared with 𝑛. This technically solved the problem, but was deemed not in the spirit of the question." https://mathstodon.xyz/@tao/115855845720777387 — a specification bug found by a solver, which is verifier exploitation one level up.

## I.4 Reported capability is a property of the harness, not the model

Tao's July 2025 thread is the sharpest thing he has written on evaluation. He opens: "there is in fact a very wide spread in capability (several orders of magnitude) depending on what resources and assistance one gives the tool, and how one reports their results", then lists seven ways to change the IMO's format — days instead of hours (or "some sort of expensive and energy-intensive time acceleration machine"); problems rewritten by the team leader beforehand; unlimited tools and internet; six students on one problem sharing partial progress; steering away from unpromising directions; best-of-six submitted; and last, "the team leader does not submit any solution at all, and silently withdraws from the competition without their participation ever being noted." His conclusion: "in the absence of a controlled test methodology that was not self-selected by the competing teams, one should be wary of making overly simplistic apples-to-apples comparisons … I will not be commenting on any self-reported AI competition performance results for which the methodology was not disclosed in advance of the competition." https://mathstodon.xyz/@tao/114881418225852441 · https://mathstodon.xyz/@tao/114881419368778558 · https://mathstodon.xyz/@tao/114881420636881657

Each knob maps to a real practice — AlphaProof's three days per problem, hand formalisation, tool access, parallel search, steering, best-of-n, selective non-reporting. §III.5 turns all seven into recorded fields.

---

# II. The mechanisms, precisely enough to reimplement

## II.1 The blueprint DAG

Patrick Massot's `leanblueprint` (https://github.com/PatrickMassot/leanblueprint) is a LaTeX document that doubles as a project tracker. Tao's tour of it on PFR: https://terrytao.wordpress.com/2023/11/18/formalizing-the-proof-of-pfr-in-lean4-using-blueprint-a-short-tour/

**Node.** One LaTeX theorem/definition/lemma environment, carrying a label (its identity in the graph); `\lean{Namespace.decl}`, the Lean declaration(s) it corresponds to; `\leanok`, claiming the environment is fully formalised and applied *separately to the statement and to the proof*; `\notready`, meaning the blueprint itself is incomplete here; `\discussion{n}`, the GitHub issue where it is argued about; and `\mathlibok`, merged upstream.

**Edge.** `\uses{labels}` — and the tool distinguishes **labels needed to state** the theorem from **labels used only in its proof**. That distinction is what makes parallelism sound: a node whose *statement* dependencies are formalised can be stated, and therefore depended upon, long before its proof exists. `\proves{label}` attaches a proof written elsewhere.

**Status lattice**, rendered as node colour: not ready → can state → stated (declared in Lean) → defined → fully proved → already in mathlib. Tao's operational summary: "the goal is to get all the bubbles leading up to and including the 'pfr' bubble at the bottom colored in green."

**Consistency check.** `leanblueprint checkdecls` verifies that every Lean declaration named in the blueprint exists in the project or a dependency, and requires a completed `lake build` — so the blueprint cannot drift into citing declarations that were renamed or never written. **CI**: GitHub Actions rebuilds the graph, the HTML and PDF renders and the doc-gen4 docs on every push. On PFR that took about half an hour — the dashboard is eventually consistent, not live.

## II.2 Why the DAG parallelises — the actual argument

Tao states it twice, and it is the load-bearing claim of this whole file.

> "A traditional mathematics collaboration rarely involves more than five or so
> co-authors, in part due to the need for every co-author to trust and verify
> the work of every other; but formalization projects routinely involve scores
> of people who may have had no prior interaction, precisely because the formal
> proof assistant allows for individual subtasks in the project to be precisely
> defined and verified independently of the other subtasks." — **[MAP]**

> "some contributors may play the role of 'project managers', focusing for
> instance on establishing precise 'blueprints' that break the project down
> into smaller pieces, while others could specialize into individual components
> of the project, without necessarily having all the expertise needed to
> understand the project as a whole." — **[MAP]**

And on the blog: "it is not necessary to wait for earlier stages of the argument to be fully formalized to start working on later stages". Three separable properties, worth naming individually because an agent runtime can have some without the others:

1. **A statement is a contract.** Once stated in Lean, it can be depended on while its proof is still `sorry`.
2. **Correctness is checked by the compiler, not by a maintainer.** Trust between contributors is not required, so contributor count is unbounded.
3. **Comprehension is local.** A contributor needs to understand one node.

## II.3 PFR — the worked example

33-page human proof; **about 20 collaborators; three weeks**; dependency graph fully green (**[MAP]**; https://mathstodon.xyz/@tao/111526765350663641). Compare **[MAP]**'s other data points: Flyspeck took Hales and 21 contributors 11 years against a 20-year estimate; the Liquid Tensor Experiment ~18 months for a 10-page proof. Formalisation also found a real defect: "one of our lemmas had omitted by mistake the hypothesis that the ambient group be 2-torsion."

Tao's cost estimate, from **[MAP]**: the de Bruijn factor — formal effort over informal effort — is "still well above one (I estimate ∼ 20), but dropping. I believe there is no fundamental obstacle to dropping this ratio below one, especially with increased integration with AI, SMT solvers, and other tools; this would be transformative to our field."

**And it has already gone below one, once.** On the hardest EQT result: "This was perhaps a situation in which the current state of formalization was sufficient to make the completion of the proof *faster* than if it was done by traditional pen-and-paper methods, because of the burden of redoing a large number of case checks every time the definitions were updated." https://github.com/teorth/equational_theories/wiki/Terence-Tao's-personal-log

The mechanism is reusable: the construction required over a hundred non-collision checks, the definition of "partial solution" changed several times, and "with each such change, Lean could verify a significant number of the previous proofs continued to work under the new definitions." Formalisation pays off exactly where a definition is unstable and the checking burden is mechanical — the situation an autonomous agent is in constantly.

## II.4 The Equational Theories Project — the scaled version

Launched 25 Sep 2024, primary goal met 14 Apr 2025. Paper arXiv:2512.07087, 34 authors, "over fifty contributors". Repo `teorth/equational_theories`. Blog: https://terrytao.wordpress.com/2024/10/12/the-equational-theories-project-a-brief-tour/ Slides (ICERM, 15 Sep 2025): https://app.icerm.brown.edu/assets/544/9916/9916_5552_Tao_091520251530_Slides.pdf

**Scale.** 4,694 equational laws (≤4 applications of the magma operation); **22,028,942** ordered implication pairs; 0 unresolved for general magmas, 2 for finite magmas.

**The amplification, the single most important number here.** Only **10,657** positive implications and **586,925** negative ones were proved directly in Lean — **597,582 facts producing 22,028,942 answers, ~37×**, purely from transitive closure plus the duality symmetry `x◇y ↦ y◇x`.

**The method ladder**, from the slides:

> "Most of the implications were 'low-hanging fruit' that could be resolved by
> relatively simple techniques, such as: Brute force use of ATPs · Brute force
> testing of small finite magmas (e.g., all magmas of order at most 4) ·
> Testing of special magmas, such as linear models x ◇ y = ax + by + c on a
> ring, or translation-invariant models x ◇ y = x + f(y − x) · Applying
> transitive closure or duality. This reduced the original set of 22028942
> problems to about a thousand."

**524 finite magmas refuted 13.6 million implications** (13.3M at size 3 alone) for 165 CPU-hours. The residual ~1,000 then consumed the bulk of fifty people's remaining six months, and every one was negative.

**Tools.** Vampire (saturation and finite model building), E, Z3, Prover9/Mace4, SAT inside the greedy-closure analysis; `egg` and `duper` only in forks, kept out of the base repo; `lean4checker`/`lean4lean` for kernel replay; `native_decide` banned to keep the trusted base small. Machine proofs were ingested by Stefan Hetzl's pipeline — implication → Prover9 → gapt → resolution-to-sequent-calculus → line-by-line Lean emission — with a reverse path too, a machine proof "deconstructed" into a two-line human argument.

**Repo mechanics worth copying directly.**

- A custom `equation` command emitting both the semantic form (a sentence over magmas) and the syntactic form (a pair in a `FreeMagma`) from one declaration.
- **`@[equational_result]`** — an attribute CI harvests to rebuild the whole implication graph from the codebase. The graph is *derived*, never restated.
- **`proof_wanted`** — records a non-Lean or machine-generated result as a **conjecture** on the dashboard, upgradeable to `theorem` later. Status lives in the type, not in prose.
- GitHub Projects issues with **CI-enforced single-claimant locking**: "The CI ensured that at most one contributor could claim a task at any time."
- A five-column dashboard (explicit/implicit × true/false, plus unresolved) as the progress metric, plus Equation Explorer, Graphiti and the Finite Magma Explorer as views.

**Human insight multiplies the solver rather than replacing it.** Daniel Weber's observation that one could restrict to magmas whose squaring map is injective, added as an explicit axiom, "led to a 100x speedup in Vampire's algorithm." The project also tracked per-equation **"immunities"** — technique classes known not to apply — which "has helped tremendously with 'weapon selection'".

**Granularity was the design variable.** Day 1: "the atomic tasks to complete are very small and most of them do not require extensive expertise in either Lean or math." Day 2: such projects, "extremely large numbers of independent pieces, each of which are easy to understand and attackable by a variety of methods, are a very good use case".

**Tao's ex-ante criteria for whether a problem is crowdsourcable at all** — the best available triage checklist, from the slides: **modularity**; **verifiability**; **elementary** components; **diversity of technique**; **transferability** between component problems; a **precisely defined goal with a quantitative metric of progress**, so "even small incremental contributions can be seen to 'move the needle'"; and **visualizability**.

**The dog that did not bark** — his own slide title. Modern ML contributed Copilot autocomplete (mainly as a Lean↔LaTeX translation layer), Claude for the visualisation tools, and one ChatGPT contribution to a confluence argument. The work was done by ATPs, which "were far cheaper to run and already handled the overwhelming majority of the implications that the advanced AI tools could" [secondary — https://terrytao.wordpress.com/2025/12/09/the-equational-theories-project-advancing-collaborative-mathematical-research-at-scale/]. A major AI company told him in the first days that their tools resolved over 99.9% of implications "but with quite long and inelegant proofs"; it did not change the project.

**Two operational warnings.** Compile time became a scaling bottleneck ("50 minutes in some cases … efficiency of compile time becomes a concern"), and delaying a Lean/mathlib bump rots a codebase: unmaintained projects' proofs, "while still correct, will become gradually more incompatible with the latest versions of the core Lean libraries."

## II.5 AlphaProof, AlphaGeometry, and mathlib

**AlphaGeometry** (*Nature* 625, Jan 2024): a language model proposing **one auxiliary construction** at a time around a symbolic deduction engine (DD+AR) run to saturation — the LM exists only because auxiliary constructions come from an infinite space no symbolic search can enumerate. Trained on 100M synthetic theorem-proof pairs from 1B random diagrams, 9M needing an auxiliary construction, no human proofs; 25/30 on IMO-AG-30 against 14 for the symbolic engine alone. **AlphaGeometry2** (arXiv:2502.03544) reached 84% and raised formal-language coverage from 66% to 88% — a third of IMO geometry problems previously could not even be *stated*.

**AlphaProof** (*Nature*, 12 Nov 2025, DOI 10.1038/s41586-025-09833-y; blog https://deepmind.google/blog/ai-solves-imo-problems-at-silver-medal-level/): AlphaZero-style RL over Lean. ~1M informal statements auto-formalised *stochastically* into ~80M Lean statements; proof-tree search with AND nodes for conjunctive subgoals; and **test-time RL** — on a hard target it generates variations of the theorem itself and trains on them live. IMO 2024: **28/42** (gold cutoff 29), "one problem within minutes and up to three days to solve the others", after "the problems were manually translated into formal mathematical language". Formal proofs score 7/7 or 0.

Tao's assessment (https://mathstodon.xyz/@tao/112850716240504978): great work; geometry effectively solved; significant compute per problem and human help on formalisation; "the database of formal proofs generated by this effort could be a useful resource if shared more openly"; and, per the AI effect, "once explained, it does not 'feel' like an exhibition of human-like intelligence". He contrasts NuminaMath, "fully automated and orders of magnitude more resource efficient", an LLM generating Python to brute-force numerical answers — "the multidimensional nature of the general challenge" (https://mathstodon.xyz/@tao/112850769040110612). In 2025 Gemini Deep Think scored 35/42 end-to-end in natural language inside the 4.5-hour limit, officially certified, against Buzzard's summary of the two years: "what the last 12 months have given us is a one point improvement." https://xenaproject.wordpress.com/2025/08/03/ai-at-imo-2025-a-round-up/

**mathlib and what reuse buys.** 284,375 theorems and 135,578 definitions across ~2.1M lines (https://leanprover-community.github.io/mathlib_stats.html); maintenance practice in arXiv:2508.21593. Tao: a formalisation project "will typically contribute many basic mathematical results generated through the course of the project to a common mathematical library, which makes it easier for future formalization projects to proceed" — **[MAP]**. The counterpoint is EQT, which "only had a modest reliance on Lean's Mathlib": reuse pays where the domain is deep and prerequisites are shared, little where the problem is elementary and self-contained. An agent should not assume a library is its bottleneck without checking.

## II.6 AlphaEvolve — search whose verifier is the security boundary

Tao, Georgiev, Gómez-Serrano and Wagner, *Mathematical exploration and discovery at scale* — full treatment in `04b`. Two findings feed §III: verifier exploitation (§I.3), which makes the verifier an adversarial boundary where exact arithmetic is not a nicety; and Tao's proposal that such search become a routine pre-publication sanity check, with negative results systematically recorded rather than left as folklore. https://terrytao.wordpress.com/2025/11/05/mathematical-exploration-and-discovery-at-scale/

## II.7 Summary: mechanism → what it buys → what breaks without it

| Mechanism | Buys | Without it |
|---|---|---|
| Statement/proof dependency split | Parallelism across the DAG | Work serialises behind unproved upstream nodes |
| `sorry`-able stated contract | Downstream starts immediately | Every contributor waits |
| Compiler as arbiter | Unbounded, untrusted contributors | Trust caps the team at ~5 |
| Status lattice + dashboard | Progress measurable in units | No way to see whether a run is advancing |
| Derived graph (`@[equational_result]`) | One source of truth | A second list that disagrees |
| `proof_wanted` | Conjectures visible but not load-bearing | Unverified results silently satisfy goals |
| CI-enforced task claiming | No duplicated or clobbered work | Two workers on one node |
| Transitive closure + symmetry | ~37× amplification (EQT) | 37× the proving cost |
| Cheap-first ladder | 22M → ~1,000 before the expensive tier | Frontier-model spend on brute-force work |
| Immunity metadata | Weapon selection | Re-attempting known-dead techniques |
| Exact-arithmetic verifier | Search results mean something | Exploits reported as progress |

---

# III. Design requirements for an autonomous math agent

Each requirement: a testable capability, the evidence implying it, and where it lands in this repository.

## III.1 Claims and verification

- **R1 Machine-checkable goal.** Every asserted claim carries a verification artefact — Lean term, executed program, or exhaustive check. *Test:* a claim with no artefact never reaches the derivation. *Repo:* `research/CLAIMS.md`.
- **R7 Red-team gate.** Nothing is accepted that the agent cannot restate and defend without re-invoking the generator; an independent role re-derives or refutes each claim, and disagreement blocks promotion. *Evidence:* §I.3.
- **R8 Calibrated evidence classes.** Every claim carries an evidence class — proved / machine-verified / numerically checked / heuristic / cited — and "certain" is reserved for machine-checked. *Evidence:* Tao's remark that AI tools "do not rate their own confidence accurately" `[secondary]`.
- **R27 "Table-true" is not proved.** A statement that has merely survived every instance tested is labelled as such and can never satisfy a goal. *Evidence:* TxGraffiti reserves "theorem" for formal proof; the Ramanujan Machine tested to 2,000 digits and still wrote that this "does not replace the need of a formal proof". See `04b`.
- **R9 Citation grounding.** Every literature claim resolves to a retrievable URL whose statement has been checked. An unretrievable citation is a failure.
- **R33 Novelty against the literature.** A claimed discovery carries a literature-search record, and where the problem has a numeric state of the art, the comparison against it. *Evidence:* FunSearch's argument that beating SOTA distinguishes discovery from retrieval; its inverse, Erdős #728 vs Pomerance 2014 (§I.3).

## III.2 The goal graph

- **R2 Explicit statement DAG.** A persistent graph of named statements with `uses` edges and a status lattice (open / stated / proved / verified / retracted), not a linear transcript. *Test:* the graph renders and every attempt names the node it targets.
- **R3 Contract-first decomposition.** A subgoal's *statement* is fixed before its proof is attempted, and downstream work may proceed against an admitted statement. *Test:* two concurrent arms progress, one depending on a stated but unproved sibling. *Evidence:* §II.1–II.2.
- **R4 Statement authorship stays supervised.** The agent cannot weaken a goal statement to make it provable; a statement mutation requires a directive and is logged. *Evidence:* Erdős #728's "trivial solutions … deemed not in the spirit of the question".
- **R21 Atomic granularity.** Subgoals small enough that one attempt can close one; the run reports the size distribution and flags a decomposition whose nodes never close. *Evidence:* EQT Day 1.
- **R22 Claim exclusion.** A node is held by at most one worker, enforced by the runtime. *Evidence:* EQT's CI locking; this repo's own two-containers-on-one-workspace failure.
- **R23 Conjecture status distinct from proved.** An unverified result is recorded as a conjecture (EQT's `proof_wanted`) and upgraded only on verification; the progress metric counts the two separately.
- **R12 Fan-out with a delta merge.** Independent nodes run concurrently, no arm reads another's output, and the merge folds counters by delta. *Repo:* already true post-attempt; R12 extends it to the goal DAG.
- **R13 Reusable lemma library.** Everything proved is stored keyed by statement and recall is attempted before any re-proof. *Test:* a duplicate subgoal in a later run is answered from the library.

## III.3 Search strategy

- **R5 Cheap-first ladder, model last.** Brute-force small structures → special parameterised model families → ATP/SMT under a time budget → closure, *before* any model call. *Test:* cost per resolved subgoal is instrumented by rung; reaching the model rung with cheaper rungs unexhausted fails the check. *Evidence:* 22,028,942 → ~1,000 (§II.4), and "far cheaper to run and already handled the overwhelming majority".
- **R5b Closure engine.** Every established fact is propagated through the problem's entailment relation and symmetry group before new work is scheduled. *Test:* direct proofs vs total answers, ratio ≫ 1. *Evidence:* ~37× in EQT.
- **R5c Immunity metadata.** Each open subgoal records which technique classes are known not to apply, and the scheduler reads it. *Test:* a technique recorded as immune is never re-attempted on that subgoal.
- **R5d Derived-axiom injection.** Solver calls carry an axiom slot for human- or self-derived auxiliary facts. *Evidence:* the 100× Vampire speedup.
- **R10 Numerics before proof.** The agent computes before it argues, and records negative results. *Test:* a conjecture node cannot reach `stated` without an attached numerical check. *(See the gap note — Tao's own writing on this is the part of the research still missing.)*
- **R11 Counterexample search as a parallel arm.** Refutation runs against its own budget alongside proof. *Test:* on a false statement the agent refutes rather than producing a plausible proof. *Evidence:* Autograph, 1989.
- **R35 Refute-then-repair.** A refutation schedules a weakened or corrected variant rather than closing the line. *Evidence:* the knot conjecture killed by braids and repaired via the injectivity radius — Tao's "promising paradigm" (`04b`).
- **R28 Non-redundancy filter.** A conjecture is discarded unless it says something not implied by the existing claim and theorem list. *Test:* given a known theorem and a trivial consequence, neither is emitted. *Evidence:* Dalmatian tests informativeness *before* correctness; "more than half of the program" is the triviality filter.
- **R29 Defined stopping rule.** Conjecture generation halts on a condition, not on budget exhaustion, and reports which fired. *Evidence:* Graffiti's "Bingo" coverage condition.
- **R30 One-line objective swap.** A new extremal problem is posed by supplying a scorer alone. *Evidence:* Wagner — "the only thing we need to change … is the function that calculates the score".
- **R31 Soft scoring available.** Penalised violations, not only hard constraints, and the penalised form is the default for constrained construction search. *Evidence:* `edges − 2·triangles` beating the hard form.
- **R32 Population diversity.** Island-model or otherwise partitioned populations with best-shot prompting rather than one hill-climbing lineage; collapse is detected. *Evidence:* FunSearch's four ingredients.

## III.4 Verifiers and tools

- **R6 Adversarial verifier.** Scored search uses exact arithmetic and conservative bounds, and the agent is assumed to be attacking it. *Test:* a deliberately loose verifier is exploited in a regression test, the hardened one is not. *Evidence:* AlphaEvolve.
- **R17 Ground the toolchain.** Library-dependent output is checked against the *current* library, not the model's memory of it. *Test:* generated Lean compiles against pinned mathlib in CI. *Evidence:* the o1 Lean experiment; EQT's version drift.
- **R26 Pin and bump deliberately.** Library and solver versions are pinned per workspace; a bump is an explicit, tested, visible event.
- **R36 Off-the-shelf over bespoke ML.** No component requires per-problem hyperparameter tuning. *Evidence:* PatternBoost — "machine learning is hard!" — and their unmodified use of Makemore.
- **R34 Interpretable output.** Extremal results ship with the program or argument that generates the object, not only the witness. *Evidence:* FunSearch's cap-set program; AlphaEvolve recovering the Talenti function.

## III.5 Honesty about the harness

R14 (budget honesty) and R15 (methodology declared in advance) expand into one table, from §I.4. Six of the seven map onto files this repo already has.

| Tao's knob | Field the run must record |
|---|---|
| Extended time / "time acceleration machine" | wall-clock and token/compute spend per result |
| Problems rewritten before the exam | whether the statement was hand-formalised or restated, and by whom |
| Unlimited tools, textbooks, internet | which tools were registered (research gating already does this) |
| Six students on one problem, sharing progress | concurrency, and whether arms shared state |
| Leader prompts toward favourable approaches | every directive, timestamped — `config/DIRECTIVES.md` |
| Best solution submitted, rest discarded | attempts generated vs attempts submitted |
| **Silent withdrawal when nothing works** | attempts abandoned, and the run reported *even on failure* |

- **R24 Quantitative progress metric and rendered frontier.** Counters that move on incremental contributions, plus a renderable goal graph. *Test:* every attempt either moves a counter or is recorded as a dead end. *Evidence:* two of Tao's seven crowdsourcability criteria.
- **R19 Failure ledger.** Dead ends, refuted conjectures and exploited verifiers are recorded and consulted. *Evidence:* Tao's proposal that negative results stop being folklore; PatternBoost reporting its failure to beat a conjectured bound as weak evidence *for* it.

## III.6 Scope, direction and delivery

- **R25 Problem-suitability triage.** Before committing budget, score the problem against Tao's seven criteria — modularity, verifiability, elementary components, diversity of technique, transferability, quantitative progress metric, visualizability — and report which are absent. *Test:* a monolithic, non-verifiable problem is flagged, not fed to the decomposition machinery.
- **R16 Brainstorming mode, kept out of the claim ledger.** Candidate strategies are enumerated with rationales, distinctly from asserting a result. *Test:* strategy proposals are filed as directives, never as claims. *Repo:* already the `director` role's rule. *Evidence:* Tao's "drawing out a user's latent knowledge … by being a good listener".
- **R18 Digestion, not just generation.** A result is done when it has a human-readable exposition tied to the machine-checked artefact — the blueprint's two-column discipline. *Test:* every proved node has both.
- **R20 Human interrupt at a boundary.** Direction reaches a live run without restarting it, is queued, and never enters the claim ledger. *Repo:* already satisfied by `./steer`; listed for completeness.
