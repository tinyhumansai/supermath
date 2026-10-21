# 04b — Machine conjecture generation: mechanisms

Companion to `04-machine-math.md`, split out to keep both files under the repository's 500-line Markdown cap. This file holds the mechanism detail for the conjecturing systems; the design requirements they imply live in `04-machine-math.md` §III (R27–R36).

Every claim carries a URL. Anything that reached us through a page summarizer rather than raw bytes is marked `[secondary]`.

## The organising question

These systems differ far less in how they *generate* candidates than in where their *verifier* comes from. That is the axis worth copying.

| System | Generator | Where the verifier comes from |
|---|---|---|
| Graffiti | Enumerate low-complexity invariant inequalities | A fixed database of graphs |
| Graffiti.pc | Expression-tree search in a user-controlled algebra | Same, plus user-supplied counterexamples via GUI |
| CONJECTURING | Expression search over invariants of arbitrary objects | Database + a seeded list of known theorems |
| TxGraffiti | Affine templates, coefficients fitted by small LPs | A "snapshot table" of instances |
| Optimist/Pessimist | MIP + heuristics over a growing knowledge base | An adversary that generates refuting objects |
| Ramanujan Machine | MITM enumeration; gradient descent to a lattice | Numerical precision escalated to 2,000 digits |
| Davies et al. (DeepMind) | **A human**, reading saliency of a fitted `f̂` | A human, building counterexamples by hand |
| Wagner | RL agent emits constructions as token sequences | The score function itself |
| PatternBoost | Local search ⇄ transformer trained on top outputs | The score function itself |
| FunSearch | LLM mutates programs under best-shot prompting | A sandboxed interpreter with time/memory limits |
| AlphaEvolve | LLM evolves code that generates candidates | Hand-written exact-arithmetic verifier |

The through-line, in Tao's framing: an unreliable generator is safe exactly to the degree its verifier is cheap, exact, and not itself exploitable. Two rows above have verifiers the generator can attack — AlphaEvolve (Tao's team had to rewrite theirs in exact arithmetic) and any score function with floating-point slack. Two rows have no counterexample search at all — the Ramanujan Machine substitutes precision, and Davies et al. substitute a mathematician.

## Graffiti (Fajtlowicz, 1986–)

Primary history: DeLaViña, *Some History of the Development of Graffiti*, DIMACS Ser. **69**, AMS (2005), 81–118 — https://www.uhd.edu/documents/academics/sciences/history.pdf

Conjecture shapes are deliberately tiny: `I ≤ J`, `I ≤ J + K`, `I + J ≤ K + L`, where literals range over ~20 selected invariants plus the constant 1. Fajtlowicz, *On Conjectures of Graffiti*, Discrete Math. 72 (1988) 113–118, quoted in the above:

> "The basic idea of Graffiti is that it 'knows' certain graphs and it is
> capable of evaluating certain formulas from graph-theoretical invariants. If
> none of the graphs with which Graffiti is familiar is a counterexample to a
> formula then the formula is considered to be a conjecture. … more than half
> of the program consists of various heuristics whose purpose is deletion of
> trivial and otherwise noninteresting but true conjectures."

**Read that last clause twice.** The generator is the easy half. The interesting-filter is the program.

Output mattered: Graffiti's conjectures "have inspired about eighty papers, some by researchers such as Alon, Bollobás, Chung, Erdős, Kleitman, Lovász, Pach, Seymour, Shearer and Spencer, and parts of five Ph.D. theses."

**Autograph** — a dedicated counterexample searcher, in 1989. Fajtlowicz, *On Conjectures and Methods of Graffiti*: "Autograph is a recent addition, searching for counterexamples to false conjectures." It starts from a given graph and adds or removes edges according to the effect on live conjectures. Refutation was designed in from the beginning, not bolted on.

Refutation rates were high and that was fine: "After considering some of the first 30 conjectures, the author found nine counterexamples to refute at least 13 conjectures"; "Of the 61 conjectures tested by Builddbs, 42 were refuted by 25 examples of small triangle-free graphs on fewer than 11 vertices."

### The Dalmatian heuristic (1995) — the finding worth stealing

From *On Conjectures of Graffiti V*:

> "The program keeps track of conjectures made in the past and when it runs
> across a new candidate for a conjecture then first of all it verifies if
> there is an example (in the database) demonstrating that the conjecture does
> not follow from the previous conjectures. If there is no such example then
> the conjecture is rejected as non-informative. If there is one, then the
> program proceeds with testing the correctness of the conjecture … those
> conjectures which are less informative than the new one are removed from the
> list and stored separately in the case the new conjecture will be refuted in
> the future."

DeLaViña states the inversion plainly: "the first heuristic applied by the program would now test for informativeness of each conjecture, and correctness would become the second consideration."

So: **check whether a conjecture says anything new before checking whether it is true.** Truth is cheap to test and abundant; non-redundancy is the scarce property. A superseded conjecture is retained rather than deleted, because the statement that replaced it may itself be refuted later.

Halting is defined, not budget-driven. The *touch number* of a relation `α ≤ β` is the number of database models achieving equality; Dalmatian stops — the condition is called **Bingo** — "if and only if for every model G in the database there exists a conjecture on the list whose touch number was contributed to by the model." Coverage of the instance set, not exhaustion of a compute budget.

Modern follow-up: Roucairol & Cazenave, arXiv:2409.18626 — "Out of 13 already refuted conjectures from Graffiti, our algorithms are able to refute 12 in seconds. We also refute conjecture 197 from Graffiti which was open until now."

## CONJECTURING (Larson & Van Cleemput)

Open-access restatement: IJCAI-17 extended abstract, https://www.ijcai.org/proceedings/2017/0713.pdf (the *Artificial Intelligence* 231 (2016) paper is paywalled).

> "Simply put, the heuristic is to produce a considered mathematical statement
> if it is both true — with respect to some given examples (matrices,
> integers, graphs, etc.) — and if the statement gives new information about
> those objects, in particular, if it says something about at least one of the
> objects which is not implied by any other stored statement or conjecture."

> "We see conjecture-making — and conjecture-revision in the face of
> contradictory data (counterexamples) — as a central feature of intelligence.
> We make guesses, based on our previous experience in relevantly similar
> situations, learn that our guesses are wrong, revise them, and test them
> against our experience."

Its distinguishing move: the conjecture list can be *seeded with known theorems*, which guarantees the output is not implied by the existing literature. A machine analogue of a literature review, executed as a filter rather than as a search.

## TxGraffiti

https://arxiv.org/html/2409.19379v2

Architecturally simpler than Graffiti: rather than searching an expression grammar, it "fixes a simple parametric hypothesis class (for example, affine inequalities … under a predicate) and fits the coefficients by solving an optimization model over the corresponding rows of the snapshot table" — a sequence of small linear programs.

Its vocabulary rule is the most directly transferable idea in this file:

> "These statements are conjectures in the traditional mathematical sense: they
> are validated against a finite snapshot table of instances and may fail on
> objects not yet represented. Accordingly, throughout the paper we use
> 'table-true' (or 'true on the snapshot') to mean 'no counterexample among the
> stored instances,' and we reserve 'theorem' exclusively for statements
> established by a formal proof."

The pipeline is stated as a pipeline, which is what makes it copyable:

> "each statement below first appeared as a conjecture output by TxGraffiti
> from a curated snapshot table of graph invariants, and then proceeded through
> the usual mathematical pipeline — counterexample search, refinement of
> hypotheses, and proof development — culminating … in published theorems or
> peer-reviewed preprints."

Results that completed the pipeline: `α(G) ≤ μ(G)` for `r`-regular graphs (arXiv:2104.01092); `Z(G) ≤ β(G)` for connected claw-free graphs; `Z(G) ≤ γ(G) + 2` for connected cubic claw-free graphs, sharp iff diamond necklace or `K₄` (arXiv:2406.19231). One central conjecture from its output remains open.

**Optimist/Pessimist** (https://arxiv.org/html/2411.09158v1) makes the adversary a component:

> "Through interaction with a counterexample generator — referred to as the
> *Pessimist*, which can be either a human user or an autonomous agent —
> *Optimist* adapts its conjectures and knowledge base. When a counterexample
> is found that disproves a conjecture, the *Pessimist* introduces a new graph,
> prompting *Optimist* to re-evaluate its existing conjectures."

The refuting object is *retained*, so one counterexample permanently raises the bar for every future conjecture.

## Ramanujan Machine

arXiv:1907.00205v4 — https://arxiv.org/pdf/1907.00205v4

Two algorithms. **MITM-RF** stores right-hand sides in a hash table: "This makes the algorithm's time complexity O(M+N)" instead of a naive `O(MN)`, running first at low precision to prune, then re-testing survivors at high precision. **Descent&Repel** runs gradient descent on many points at once, adds a Coulomb-like `C/‖a−b‖²` repulsion so they do not collapse onto the same minimum, and alternates with a lattice loss to force integrality. They observe "all minima are global, and their errors are zero" — the search landscape is unusually benign, which is why the method works here and generalises poorly.

Verification is precision, and they say so:

> "the probability of finding a random match for an enumeration space of 10⁹
> and result accuracy of more than 50 digits, is smaller than 10⁻⁴⁰. Our
> algorithms tested the conjectures for up to 2000 digits of accuracy.
> Nevertheless, such an accuracy does not replace the need of a formal proof."

The aftermath is the part an autonomous agent should note, because it is a result about *publishing* conjectures rather than proving them:

> "Following the appearance of the initial version of our work on arXiv, many
> people ran our algorithms, and a few even found new conjectures. Others
> responded with proofs to the new formulas found by our algorithms. In fact,
> over the span of a few months, proofs for all the formulas in the original
> manuscript were presented."

Follow-up at scale: Elimelech et al., PNAS 121(25), 2024, https://doi.org/10.1073/pnas.2321440121 — "one of the most extensive automated discovery efforts in experimental mathematics, engaging thousands of volunteers to execute a massively parallel algorithm for more than two years."

## Davies et al., *Nature* 600 (2021)

Open full text: https://pmc.ncbi.nlm.nih.gov/articles/PMC8636249/

The loop, stated formally:

> "it helps guide a mathematician's intuition about the relationship between
> two mathematical objects X(z) and Y(z) associated with z by identifying a
> function f̂ such that f̂(X(z)) ≈ Y(z) and analysing it… If f̂ is more accurate
> than would be expected by chance, it indicates that there may be such a
> relationship to explore. If so, attribution techniques can help in the
> understanding of the learned function f̂ sufficiently for the mathematician
> to conjecture a candidate f′. … This iterative process might need to be
> repeated several times before a viable conjecture is settled on."

They call this a "test bed for intuition". Note what they deliberately do *not* do — the sharpest contrast in this file:

> "Rather than use machine learning to directly generate conjectures, we focus
> on helping guide the highly tuned intuition of expert mathematicians."

Their assessment of the alternatives is worth quoting alongside Graffiti's eighty papers: prior systems "have either contributed genuinely useful research conjectures via methods that do not easily generalize to other mathematical areas, or have demonstrated novel, general methods for finding conjectures that have not yet yielded mathematically valuable results."

### Refute-then-repair, in full

This is the episode Tao singles out in the *Notices* as "a promising paradigm", and it is worth having both ends of it on the page.

> "**Conjecture:** There exist constants c₁ and c₂ such that, for every
> hyperbolic knot K, |2σ(K) − slope(K)| < c₁ vol(K) + c₂"

> "While this conjecture was supported by an analysis of several large datasets
> sampled from different distributions, we were able to construct
> counterexamples using braids of a specific form. Subsequently, we were able
> to establish a relationship between slope(K), signature σ(K), volume vol(K)
> and one of the next most salient geometric invariants, the injectivity
> radius inj(K)."

> "**Theorem:** There exists a constant c such that, for any hyperbolic knot K,
> |2σ(K) − slope(K)| ≤ c vol(K) inj(K)⁻³"

Surviving several large sampled datasets did not make the first statement true. The repair — not the refutation — is where the theorem came from.

Representation theory, same paper: saliency on the 40-year-old combinatorial invariance conjecture showed "extremal reflections … appear more commonly in salient subgraphs than one would expect, at the expense of simple reflections", leading to the hypercube decomposition, verified on "all of the ∼3 × 10⁶ intervals in the symmetric groups up to S₇ and more than 1.3 × 10⁵ non-isomorphic intervals sampled from S₈ and S₉."

Marcus du Sautoy, on the DeepMind blog: "It feels like Galileo picking up a telescope and being able to gaze deep into the universe of data."

## Wagner, *Constructions in combinatorics via neural networks* (2021)

https://arxiv.org/pdf/2104.14516v1

The design claim that matters for an agent runtime:

> "An advantage of using reinforcement learning algorithms in this manner is
> that we can use what is essentially the same exact program to try and attack
> all mathematical conjectures which might have a finite counterexample – the
> only thing we need to change in the code is the function that calculates the
> score of a given construction."

> "at no point does it have any knowledge on what problem it is trying to solve
> and how the reward is calculated."

Termination is the refutation: "while the best construction found is not a counterexample do …". And a scoring nuance worth copying — soft beats hard: `score(G) = #edges(G) − 2·#triangles(G)`, "which allows for the occasional mistake, leads to slightly better results" than forbidding triangles outright.

Refuted, among others: Aouchiche–Hansen's `λ₁ + μ ≥ √(n−1) + 1` (smallest counterexample at n=19) — a conjecture **AutoGraphiX itself had generated**, so a machine conjecture killed by a machine search; `π + ∂_⌊2D/3⌋ > 0` (203-vertex counterexample); Collins' unimodality conjecture, refuted "in a strong sense" by an infinite family; Brualdi–Cao's permanent guess, where the true sequence "has the unexpected initial segment 1, 2, 4, 8, 16, 32, 64, 120"; and Király–Nagy–Pálvölgyi–Visontai's `g(a,b) < 2·C(a+b,b)`, answered no by a (4,4)-set system of size 146 > 140.

His own scoping is honest and should be quoted next to any claim about this class of method: "while we did not succeed in refuting any of the most famous conjectures in the field, we will present counterexamples and constructions to a wide variety of lesser known open problems."

## PatternBoost (Charton, Ellenberg, Wagner, Williamson, 2024)

https://arxiv.org/pdf/2411.00566v1

Alternates a classical local search with a transformer trained on the top-scoring outputs of the previous round, then samples from it to seed the next. "One may think of PatternBoost as an extra layer that can be placed on top of any local search method … whatever local search algorithm we have, PatternBoost can often make it better."

Why the architecture changed from Wagner's: "The problem with the cross-entropy method (in the bare form used in [49]) is its scaling: the vanilla neural network becomes difficult to train when the sequence length exceeds a few hundred tokens."

Headline result — Graham's conjecture on hypercube subgraphs, open 30 years: "for d = 6 we were able to find a graph with 81 edges (as opposed to the 2⁶ + C(6,3) − 2 = 82 edges in the construction above) … This disproves the above conjecture, and marks the first progress on this problem in 30 years."

Others: no-`C₄` at n=33, where 50M plain local searches capped at 89 edges and PatternBoost found 96; isosceles-free grids, 108 → 110 points at n=64 after three months of bespoke search had stalled, and 154 → 160 at n=100; saturated 8-Sperner systems of size 108, improving ε from ≈0.0385 to 0.04085.

Also a negative result, reported: on cross-Sperner families "We did not manage to beat their conjectured upper bound for any values of the parameters. To us, it seems likely that the conjecture is true." Failure to refute, recorded as weak evidence for the conjecture.

Two remarks aimed straight at tooling:

> "One difficulty for using machine learning as a practical tool in mathematics
> is that machine learning is hard! One can lose many hours tuning
> hyperparameters, exploring different tokenization schemes, and so on. A
> virtue of PatternBoost, as we see it, is that the transformer architecture
> appears very resilient, and can often be used 'off the shelf' without much
> tinkering by the mathematician whose expertise and interest may be
> elsewhere."

> "Indeed, there is nothing in the method which is specific to mathematics at
> all! … One obvious challenge is that a proposed example in mathematics can
> often be evaluated mechanically, reliably, and quickly, and this is crucial
> for PatternBoost; in other domains, evaluation may pose more difficulties."

They use Karpathy's Makemore unmodified. Where it works: "when the best constructions to the problem clearly follow some pattern, but these patterns are too complicated for us to fully understand."

## FunSearch (*Nature* 625, 2024)

Open full text: https://pmc.ncbi.nlm.nih.gov/articles/PMC10794145/

Four ingredients, all reusable verbatim:

> "First, we sample best performing programs and feed them back into prompts
> for the LLM to improve on; we refer to this as best-shot prompting. Second,
> we start with a program in the form of a skeleton (containing boilerplate
> code and potentially known structure about the problem), and only evolve the
> part governing the critical program logic … Third, we maintain a large pool
> of diverse programs by using an island-based evolutionary method that
> encourages exploration and avoids local optima. Finally, leveraging the
> highly parallel nature of FunSearch, we scale it asynchronously."

The filter is a sandbox, and rejection is silent and cheap: "Programs that were incorrect (that did not execute within the imposed time and memory limits, or produced invalid outputs) are discarded."

The epistemics — the argument an autonomous agent most needs to be able to make about its own output:

> "Surpassing state-of-the-art results on established open problems provides a
> clear indication that the discoveries are truly new, as opposed to being
> retrieved from the LLM's training data."

Cap set, `n = 8`: "we do not just discover the set of 512 eight-dimensional vectors in itself, but a program that generates it … Through inspecting the code, we obtain a degree of understanding of what this set is." Capacity lower bound 2.2180 → 2.2184, then via a discovered symmetry to **2.2202** — "a great improvement to the lower bound compared to research in the last 20 years", with the honest rider that "it is still far from the upper bound."

Human-in-the-loop stated as method, not as caveat:

> "FunSearch suggests a solution, which is examined by researchers, who may
> note features of interest. These features are used to refine the search,
> leading to better solutions. This process can be iterated, with both human
> and search consistently in the loop."

Jordan Ellenberg: "The solutions generated by FunSearch are far conceptually richer than a mere list of numbers. When I study them, I learn something."

Model was Codey (PaLM2), on the order of 10⁶ samples per result — cheap enough that the method is not gated on a frontier model.

## AlphaEvolve

Tao, Georgiev, Gómez-Serrano and Wagner, *Mathematical exploration and discovery at scale* — https://terrytao.wordpress.com/2025/11/05/mathematical-exploration-and-discovery-at-scale/

The system does not optimise inputs. An LLM evolves *code* that generates inputs, scored by an executable verifier. Across 67 problems: 20 matched or beat the literature, 39 met expectations, 8 fell short. It recovered exact solutions in readable form (the Talenti function for Gagliardo–Nirenberg), sometimes generalised a construction from small parameters to large, and struggled on analytic number theory even when given expert hints. On famous open problems (Sidorenko, Sendov, Crouzeix) it located the conjectured optimisers and found no counterexample — one-sided evidence, not resolution.

The finding that matters most is that the generator attacks the verifier: the system is "extremely good at locating exploits in the verification code", satisfying an imprecise distance constraint by placing points nearly on top of one another. Tao's team rewrote the verifiers in exact arithmetic with conservative bounds, and he warns that "blindly trusting the AE values can be risky as they may be a consequence of verifier exploits rather than any true progress." This is the one row of the table above where the verifier is written by the same people the search is trying to fool, and it is the row an autonomous agent occupies by default.

He also proposes making this routine: a standard sanity check run against a new conjecture *before* publication, with the negative results — currently folklore — systematically recorded.

## Sources

- Graffiti history (DeLaViña, PDF) — https://www.uhd.edu/documents/academics/sciences/history.pdf
- *Written on the Wall II* — http://cms.dt.uh.edu/faculty/delavinae/research/wowII/
- Roucairol & Cazenave — https://arxiv.org/abs/2409.18626
- Larson & Van Cleemput, IJCAI-17 — https://www.ijcai.org/proceedings/2017/0713.pdf
- TxGraffiti — https://arxiv.org/html/2409.19379v2 · https://arxiv.org/abs/2104.01092 · https://arxiv.org/abs/2406.19231 · https://arxiv.org/abs/2507.17780
- Optimist/Pessimist — https://arxiv.org/html/2411.09158v1
- Ramanujan Machine — https://arxiv.org/pdf/1907.00205v4 · https://doi.org/10.1073/pnas.2321440121 · https://arxiv.org/abs/2308.11829
- Davies et al. — https://pmc.ncbi.nlm.nih.gov/articles/PMC8636249/ · https://doi.org/10.1038/d41586-021-03512-4 · https://www.nature.com/articles/d41586-021-03593-1
- Wagner — https://arxiv.org/pdf/2104.14516v1
- PatternBoost — https://arxiv.org/pdf/2411.00566v1 · https://github.com/zawagner22/transformers_math_experiments
- FunSearch — https://pmc.ncbi.nlm.nih.gov/articles/PMC10794145/ · https://deepmind.google/discover/blog/funsearch-making-new-discoveries-in-mathematical-sciences-using-large-language-models/
