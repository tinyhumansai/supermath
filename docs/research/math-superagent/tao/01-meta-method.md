# Tao's explicit problem-solving method

Terence Tao's *meta-writing* — what he says about how to attack a problem, not what he proves. Each entry is tagged `[STATED]` (he says it) or `[INFERRED]` (drawn from his practice, or a principle he applies without naming).

**Accuracy conventions.** Text in quotation marks is reproduced from the cited source. Where a page could only be reached through a summarising fetch, the entry says *summarised* rather than presenting words as his. Two caveats carry through the whole document:

- **There is no verbatim "5% chance" quote.** A search of the Lex Fridman #472 transcript, the AMS *Notices* article, Mathstodon and Quanta found no Tao statement putting an explicit probability or expected-value argument on long-shot proof attempts. The nearest real source is his November 2024 risk-tolerance thread (<https://mathstodon.xyz/@tao/113479000564381543>), which is about decision-making under uncertainty generally and *not* about attacking theorems. Do not attribute a "5% approach" to him. The operational content people usually mean by it is §4 and §23 below.
- **"General polymath rules"** (<https://polymathprojects.org/general-polymath-rules/>) is unsigned Polymath house doctrine that self-describes as derived from Gowers's guidelines for the first project. Tao is a blog administrator and principal author of the mini-polymath guidelines it descends from, and he enforced it consistently, but it is not signed Tao prose. Entries resting on it say so.

Short source keys: **[Lex]** = <https://lexfridman.com/terence-tao-transcript/>; **[SMP]** = *Solving Mathematical Problems: A Personal Perspective*, 2nd ed., ch. 1; **[Notices]** = *Machine assisted proof*, Notices AMS Jan 2025, <https://terrytao.wordpress.com/wp-content/uploads/2024/03/machine-assisted-proof-notices.pdf>; **[Slides]** = *Polymath projects*, <https://terrytao.wordpress.com/wp-content/uploads/2015/07/polymath.pdf>; **[Retro]** = Tao's §2 of arXiv:1409.8361, <https://arxiv.org/abs/1409.8361>.

---

## A. Attacking a problem

### 1. Cheat strategically — turn off nine of the ten difficulties `[STATED]`

His first move on an unfamiliar problem. Enumerate what makes it hard, build a deliberately falsified version with all but one difficulty disabled, solve that — then recombine. Mathematics uniquely permits this because you own the problem statement.

> "One thing you pick up as a mathematician is I call it cheating strategically. So the beauty of mathematics is that you get to change the problem and change the rules as you wish." … "if there are 10 things that are making your life difficult, find a version of the problem that turns off nine of the difficulties, but only keeps one of them and solve that." … "And after you know how to solve the 10 problems, 10 difficulties separately, then you have to start merging them a few at a time." — [Lex]

**Agent:** keep an explicit named list of difficulties, generate variants that disable all but one (dimension 1, no error term, finite-field model), and maintain a lattice of sub-results keyed by which difficulties are enabled. Progress is the size of the largest solved subset, not solved/unsolved. Never spend the first attempts on the full statement.

### 2. Forward reconnaissance — assume the blocker away, then count debts `[STATED]`

When one bad case blocks the argument, grant it by fiat and push the rest through, to learn whether the blocker is the *only* problem. This is his stated rule for abandoning versus persisting.

> "You can just assume by fiat this bad case doesn't occur. So you do some magical thinking, but strategically okay for the point to see if the rest of the argument goes through. If there's multiple problems with your approach, then maybe you just give up. But if this is the only problem but everything else checks out, then it's still worth fighting." — [Lex]

**Agent:** let attempts carry explicit `assume(...)` debts and continue past a blocker instead of halting. At the end, count debts: one means attack it, several independent ones mean abandon the approach. This is a better stuck-policy than a fixed attempt cap.

### 3. Try anything — the stupider the better `[STATED]`

His cure for paralysis. The value of a doomed attempt is diagnostic: the *way* it fails identifies which hypothesis is load-bearing.

> "the next step then is to try anything no matter how stupid and in fact almost the stupider, the better, which technically is almost guaranteed to fail, but the way it fails is going to be instructive. It fails 'cause you are not at all taking into account this hypothesis. Oh, this hypothesis must be useful. That's a clue." — [Lex]

**Agent:** never emit "no applicable technique". On a zero-idea state, force a cheap doomed attempt and record its failure mode; the hypotheses it left unused become the next attack's focus.

### 4. Classify the problem type before choosing a method `[STATED]`

Three kinds — "Show that…"/"Evaluate…", "Find a…"/"Find all…", "Is there a…" — and the kind determines the approach, with one genuinely easier than the others.

> "'Show that . . .' or 'Evaluate . . .' problems start with given data and the objective is to deduce some statement or find the value of an expression; this type of problem is generally easier than the other two types because there is a clearly visible objective, one that can be deliberately approached." — [SMP], p. 1-2

**Agent:** set a typed goal field at intake that drives strategy selection — "Find all" triggers guess-and-tweak plus an exhaustiveness obligation; "Is there a" triggers the concurrent construct/refute split of §10.

### 5. Modify the problem slightly — seven named moves `[STATED]`

His enumerated repertoire when stuck at the start, with a warning attached.

> "(a) Consider a special case of the problem, such as extreme or degenerate cases. (b) Solve a simplified version of the problem. (c) Formulate a conjecture which would imply the problem, and try to prove that first. (d) Derive some consequence of the problem, and try to prove that first. (e) Reformulate the problem (e.g. take the contrapositive, prove by contradiction, or try some substitution). (f) Examine solutions of similar problems. (g) Generalize the problem." — [SMP], p. 4

He immediately qualifies (a): "special cases are, by their nature, special … Start with modest assumptions first" (same page). Note that (g) runs *against* specialisation — the general statement often has more symmetry and more induction room, which is what makes the tensor-power trick (§15) work.

**Agent:** these are seven named mutation operators over the goal. Apply them by name and log which was used, instead of improvising an unlabelled "let me simplify".

### 6. Modify the problem significantly — push it until it breaks `[STATED]`

The aggressive counterpart: deliberately damage the problem to locate where its difficulty actually lives.

> "we perform major modifications to a problem such as removing data, swapping the data with the objective, or negating the objective (e.g. trying to disprove a statement rather than prove it). Basically, we try to push the problem until it breaks, and then try to identify where the breakdown occurred; this identifies what the key components of the data are, as well as where the main difficulty will lie." — [SMP], p. 5

**Agent:** a reconnaissance pass that deletes each hypothesis in turn and asks whether the conclusion survives, plus a deliberate disproof attempt. Hypotheses whose deletion breaks the statement are the ones the proof must consume.

### 7. Ask dumb questions and answer them `[STATED]`

Delete hypotheses from standard lemmas to test necessity; try alternative proofs to gauge the relative power of methods; probe degenerate cases; check whether converses hold. Textbook mathematics shows only polished results, which hides why the conventional route is conventional.

> <https://terrytao.wordpress.com/career-advice/ask-yourself-dumb-questions-and-answer-them/> (retrieved as summary: the page recommends deleting hypotheses from standard lemmas to test their necessity, attempting alternative proof methods, and exploring degenerate cases and converses.)

**Agent:** for every lemma the run relies on, spawn cheap probes — drop each hypothesis, try the converse, evaluate at a degenerate parameter — and file results as durable knowledge about the *tool*, not just about the problem.

### 8. Attack at the 90% boundary `[STATED]`

His problem-selection criterion: not the famous impossible ones, not the easy ones, but the band where known technique nearly suffices.

> "What's really interesting are the problems just on the boundary between what we can do rather easily and what are hopeless, but what are problems where existing techniques can do 90% of the job and then you just need that remaining 10%." — [Lex]

His matching abandonment criterion, on Riemann: "there's no even viable stretch. Even if I activate all the cheats that I know of in this book, there's just still no way to get from A to B" ([Lex]).

**Agent:** score candidate sub-goals by distance from existing technique and prefer the 90%-covered ones. Emit an explicit *out of reach* verdict when even the fully-cheated version has no path, and route effort elsewhere rather than looping.

### 9. The decomposition smell test `[STATED]`

The check he says machines currently lack and that he applies to his own moves: a decomposition is good only if each child is *strictly easier* and *still plausibly true*. The base rate is against you.

> "'Oh, this looks good. The two tasks look like they're simpler tasks than your main task and they've still got a good chance of being true. So this is good to try.' Or 'No, you've made the problem worse, because each of the two subproblems is actually harder than your original problem,' which is actually what normally happens if you try a random thing to try normally it's very easy to transform a problem into an even harder problem. Very rarely do you transform into a simpler problem." — [Lex]

**Agent:** gate every decomposition on two explicit questions (easier? still plausible?) and reject on either failure. A decomposition step with a high acceptance rate is not doing the check.

### 10. Prove and disprove concurrently; look for the counterexample first `[STATED]`

Counterexample search is cheaper than proof and both outcomes are wins. The Equational Theories Project is his own worked example at scale.

> "If you are trying to prove some ambitious claim, you might try to first look for a counterexample; either you find one, which saves you a lot of time and may well be publishable in its own right, or else you encounter some obstruction." — <https://terrytao.wordpress.com/career-advice/be-sceptical-of-your-own-work/>

> "we look at all the pairs between these 4,000 laws and this up 22 million of these pairs. And for each pair we ask, does this law imply this law? If so, give a proof. If not, give a counterexample." — [Lex]

He also wants the negative result persisted: there is "value in using these tools to systematically record negative results" — <https://terrytao.wordpress.com/2025/11/05/mathematical-exploration-and-discovery-at-scale/>.

**Agent:** every conjecture spawns two concurrent tasks on one budget; whichever returns first kills the other. Persist "no counterexample below N" with its search bound so later attempts inherit it, and let the obstruction encountered become the proof's design constraint.

---

## B. Tools and technique

### 11. Structure versus randomness `[STATED]`

His unifying decomposition across analysis, combinatorics, number theory and PDE.

> "An arbitrary object can be decomposed into some structured component and some pseudorandom component." … "If an object is orthogonal (or has small correlation with) all structured objects, then it is necessarily pseudorandom, and vice versa." — Simons Lecture I, <https://terrytao.wordpress.com/2007/04/05/simons-lecture-i-structure-and-randomness-in-fourier-analysis-and-number-theory/>

The payoff: if the structured objects relevant to a problem can be classified, the statistic you want is computed by correlating against that finite list.

**Agent:** when a quantity resists direct estimation, attempt an explicit split into a low-complexity part (enumerable, classifiable) and a remainder shown negligible by an orthogonality or equidistribution argument.

### 12. The hard/soft analysis correspondence `[STATED]`

A translation table between infinitary-qualitative and finitary-quantitative statements, so an argument stuck on one side can be attempted on the other.

> "Hard analysis is the mathematics of ε, N, O(), and ≤; soft analysis is the mathematics of 0, ∞, ∈, and →." … "Qualitative analysis can be viewed as a convenient abstraction of quantitative analysis, in which the precise dependencies between various finite quantities has been efficiently concealed from view by use of infinitary notation." — <https://terrytao.wordpress.com/2007/05/23/soft-analysis-hard-analysis-and-the-finite-convergence-principle/>

Related: *cheap nonstandard analysis*, "a 'cheap' version of nonstandard analysis which is less powerful than the full version, but is not as infinitary in that it is constructive" — <https://terrytao.wordpress.com/2012/04/02/a-cheap-version-of-nonstandard-analysis/>.

**Agent:** keep two formulations of the goal — a limiting/qualitative one and an explicit-constant one — and switch representation when stuck. The finitary version is what a computation can test; the infinitary one is often shorter to prove.

### 13. Amplification, arbitrage and the tensor power trick `[STATED]`

Strengthen a weak or lossy estimate by transforming the object it applies to, exploiting a symmetry the two sides of the inequality do not share.

> Amplification "arbitrage[s] differing amounts of symmetry between the left- and right-hand sides of an estimate" — <https://terrytao.wordpress.com/2007/09/05/amplification-arbitrage-and-the-tensor-power-trick/> (summarised)

The tensor case: when you can only prove `X ≤ CY` with an unwanted constant, apply it to `M`-fold tensor powers to get `X^M ≤ C·Y^M`, take `M`-th roots, let `M → ∞`. It needs the constant to grow at most subexponentially in `M`, and the problem phrased abstractly enough to tensor (<https://terrytao.wordpress.com/2008/08/25/tricks-wiki-article-the-tensor-product-trick/>).

**Agent:** when an established bound is off by a constant or a logarithm, do not re-derive it — find a symmetry (phase rotation, homogeneity, tensoring) under which one side is invariant and the other is not, and apply the bound to the transformed object.

### 14. No self-defeating object `[STATED]`

A reusable non-existence template: assume the object exists, then use its own claimed powers to build a counterexample to those powers.

> the argument establishes that "X's powers are 'self-defeating': the very existence of X and its powers can be used (by some clever trick) to construct a counterexample to that power" — <https://terrytao.wordpress.com/2009/11/05/the-no-self-defeating-object-argument/> (summarised)

He catalogues it across Euclid's theorem, Russell's paradox, Cantor, Gödel, the halting problem, Busy Beaver, and strategy-stealing.

**Agent:** on any "no such object exists" goal, try the template first — instantiate the object, feed it to itself, look for the contradiction.

### 15. The 245A trick list `[STATED]`

Twenty-one named, reusable moves compiled for graduate real analysis: give yourself an epsilon of room; split an equality into two inequalities; approximate rough objects by smoother ones; work locally instead of globally; be willing to throw away an exceptional set; exploit Zeno's paradox (split one epsilon into countably many); pass to subsequences to improve convergence; don't fix epsilon's value until you have collected every constraint on it; once one constant is lost, lose others freely; exploit symmetries to normalise; abstract away irrelevant information. — <https://terrytao.wordpress.com/2010/10/21/245a-problem-solving-strategies/> (titles as given; descriptions summarised)

**Agent:** a named, citable technique registry, so an attempt reports "applied *epsilon of room*" rather than an unlabelled manipulation. Named failures are searchable and reusable; unnamed ones are not.

### 16. Know your tools' limits — and keep an obstruction ledger `[STATED]`

A tool you cannot see around is a liability, and the highest-value knowledge is often about what *cannot* work. His averaged Navier-Stokes construction exists to explain why blowup could not be disproved; the "censored primes" construction does the same for twin primes.

> "Knowing a library of counterexamples, or easily analysed model situations, is very important, as well as knowing the type of obstructions that your tool can deal with." … "If you view one of your favorite tools as some sort of 'magic wand' which mysteriously solves problems for you, with no other way for you to obtain or comprehend the solution, this is a sign that you need to understand your tool much better." — <https://terrytao.wordpress.com/career-advice/learn-the-limitations-of-your-tools/>

> "it's not just about taking a technique that is going to work and applying it, but you need to not take the techniques that don't work… having these counterexamples for nearby problems rules out… it saves you a lot of time because you're not wasting energy on things that you now know cannot possibly ever work." — [Lex]

**Agent:** maintain a ledger of adversarial nearby variants that any candidate proof must fail on, and test a strategy against it *before* executing. A strategy that would also "prove" the false variant is discarded unstarted. This is the strongest pruning device in the list.

### 17. Choose notation to expose symmetry, then externalise the whole state `[STATED]`

Notation is a design decision (triangle sides `b−d, b, b+d` rather than `a, a+d, a+2d`), and the written state is itself a working tool — which is why it must be visible all at once rather than as a scrolling log.

> "Putting everything down on paper helps in three ways: (a) you have an easy reference later on; (b) the paper is a good thing to stare at when you are stuck; (c) the physical act of writing down of what you know can trigger new inspirations and connections." — [SMP], p. 3

He qualifies it immediately: highlight the facts likely to be useful and keep "more questionable, redundant, or crazy ideas in another part of your scratch paper" (same page). At research scale: "in my office I have four giant blackboards and sometimes I just have to write everything I know about the problem on the four blackboards and then sit my couch and just see the whole thing" ([Lex]).

**Agent:** an explicit notation-choice step before derivation, and a regenerated "everything known" artifact — facts, bounds, failed approaches, obstructions, with promoted and speculative sections kept apart — re-read *whole* at each attempt boundary, never an append-only transcript whose tail is all the agent sees.

---

## C. Scepticism and verification

### 18. Stress-test your own argument `[STATED]`

Two named self-tests. A transformation that trades one hard problem for another is almost certainly wrong: "Transform the difficult problem to another difficult problem… then there is almost certainly a major error in your argument." And for proofs by contradiction:

> "A good way to stress-test this sort of false argument is to try to run the same argument without the initial assumption that X is false. If one can easily modify the argument to again lead to a contradiction, it shows the problem wasn't with X – it was with the argument." — <https://terrytao.wordpress.com/career-advice/be-sceptical-of-your-own-work/>

Scepticism should be "especially enforced when dealing with a problem which is known to be difficult, or one which is outside your usual area of expertise" (same page).

**Agent:** a judge that runs the contradiction stress test automatically on any proof by contradiction, and flags any step whose net effect is difficulty-preserving.

### 19. Check the literature *after* a solve — a short proof raises the prior that it is known `[STATED]`

His observation from the Erdős problems database inverts the usual reflex, and the same breadth argument makes literature recall a continuous activity rather than a start-up task.

> "the AI tools are now becoming capable enough to pick off the lowest hanging fruit amongst the problems listed as open in the Erdos problem database, where by 'lowest hanging' I mean 'amenable to simple proofs using fairly standard techniques'. However, that category is also precisely the category of nominally open problems that are most likely to have been solved in the literature, perhaps without much fanfare due to the simple nature of the arguments." — <https://mathstodon.xyz/@tao/115788262274999408>

> "one of the advantages of the Polymath projects, with their broad level of participation, is that connections to relevant literature are very likely to be unearthed by at least one of the participants." — [Retro]

**Agent:** a standing librarian role running *concurrently* with attempts, invoked on each newly formulated lemma; plus a mandatory post-solve check. A short standard-technique proof is filed as "confirmed known" unless the search comes back empty.

### 20. Numerics before theory `[STATED]`

He wants a far larger experimental component and reports his own practice changing as the cost of a quick computation collapsed.

> "I'm using more and more computers to do simple explorations." … "99% of mathematics is theoretical mathematics, and there's a very tiny amount of experimental mathematics." — [Lex]

The conjecture-editing loop, from the knot-theory case: "Further numerics disproved their initial conjecture, but suggested a modified version of the conjecture which they were able to prove rigorously." — [Notices]

**Agent:** small-case enumeration, plots and pattern hunting precede any proof attempt, and a numerically refuted conjecture is *edited into a survivor* rather than dropped.

### 21. Formalisation as a regression harness `[STATED]`

Once a proof is machine-checked, tightening it is cheap: perturb a constant, recompile, read the failures as a map of which steps bind.

> "in your headline theorem, you change your 12 to 11, you run the compiler and off the thousands of lines of code, you have 90% of them still work and there's a couple that are lined in red… but immediately isolates which steps you need to change" — [Lex]

Formalisation also produces mathematics: it "can sometimes reveal simplifications or strengthenings of the argument, for instance by revealing that a seemingly important hypothesis in a lemma was in fact unnecessary" — [Notices].

**Agent:** after verification, run automatic hypothesis-deletion and constant-tightening sweeps and let the checker report which steps bind. Strengthenings get discovered by the harness rather than guessed.

### 22. Verification is what makes fallible search worth running `[STATED]`

His account of what ad-hoc methods are actually good for, and the condition under which they pay off. This — not any probability figure — is the real content of the "long-shot attempt" idea.

> "the ability to solve broad classes of complex problems via somewhat ad hoc means. These means may be stochastic or the result of brute force computation; they may be ungrounded or fallible… And yet, they can have a non-trivial success rate at achieving an increasingly wide spectrum of tasks, particularly when coupled with stringent verification procedures to filter out incorrect or unpromising approaches, at scales beyond what individual humans could achieve." — <https://mathstodon.xyz/@tao/115722360006034040>

On the assistant's role: "not the science-fiction conception of an superintelligent AI that can solve complex mathematical problems autonomously, but a valuable assistant that can suggest new ideas, filter out errors, and perform routine case checking, numerical experiments and literature review tasks" — [Notices].

**Agent:** breadth of cheap attempts is the asset and the verifier is the load-bearing component. Unverified output has no value and must not enter the claims ledger.

### 23. Formalisation makes collaboration trustless `[STATED]`

Machine checking changes the *social* structure of an attack: sub-tasks are defined and verified independently, so the number of collaborators stops being bounded by mutual trust.

> "A traditional mathematics collaboration rarely involves more than five or so co-authors, in part due to the need for every co-author to trust and verify the work of every other; but formalization projects routinely involve scores of people who may have had no prior interaction, precisely because the formal proof assistant allows for individual subtasks in the project to be precisely defined and verified independently of the other subtasks." — [Notices]

For PFR: "the human-written proof was 33 pages long, but largely self-contained, and a group of about 20 collaborators was able to formalize it in three weeks" ([Notices]).

**Agent:** define sub-lemmas as independently checkable contracts so many parallel sub-agents can work without any of them being trusted. Verification, not reputation, admits a result to the shared ledger.

### 24. Rigour serves intuition; post-rigorous work is deliberately informal `[STATED]`

Three stages: pre-rigorous (intuitive, computational), rigorous (formal, epsilon-delta, abstract manipulation without necessarily knowing what the objects mean), and post-rigorous, where one uses "informal and semi-rigorous" methods and converts to rigour on demand.

> "rigour is not to destroy all intuition; instead, it should be used to destroy bad intuition while clarifying and elevating good intuition." The stated ideal is that "every heuristic argument naturally suggests its rigorous counterpart, and vice versa." — <https://terrytao.wordpress.com/career-advice/theres-more-to-mathematics-than-rigour-and-proofs/>

**Agent:** allow explicitly flagged heuristic reasoning as a distinct mode with its own confidence level, and require a discharge step converting each heuristic claim to a rigorous one before it enters the ledger. A run that permits only rigorous steps never reaches the mode the ideas come from.

---

## D. Pacing, portfolio and failure

### 25. Partial progress is the unit of work `[STATED]`

Trying a technique known in advance to be insufficient is often correct, because *where* it fails delineates the problem.

> "It can often be profitable to try a technique on a problem even if you know in advance that it cannot possibly solve the problem completely." … "The precise point in the argument at which it fails can be very instructive, as it can delineate what portion of the problem can be handled and highlights the key component needing further resolution." — <https://terrytao.wordpress.com/career-advice/on-the-importance-of-partial-progress/>

**Agent:** record the failure *coordinate* of every attempt (which step, which hypothesis, which parameter range) as structured data, and choose the next technique to cover the complementary portion.

### 26. Fox, not hedgehog — reconciled by reconnaissance `[STATED]`

His default reaction to a dead end is lateral movement: "Well, for me, I switch to a different problem. So I'm a fox, I'm not a hedgehog." … "I like moving on from a problem if it's giving too much difficulty." He is explicit this is temperament, crediting collaborators who "have third, fourth, and the fifth, which works" ([Lex]).

Against that stands his two-year PDE saga, where a false dawn kept the project alive:

> "if we hadn't had that initial false dawn of nearly solving a problem, we would've given up by month two or something and worked on an easier problem. If we had known it would take two years, not sure we would've started the project." — [Lex]

The tension resolves through §2: forward reconnaissance tells you which regime you are in. Note also that the eventual solution "actually didn't generate these problematic terms" rather than repairing the old argument ([Lex]).

**Agent:** drive the stuck-policy by debt count rather than a fixed cap, and treat "how close did the best attempt get" as a budget signal that can *raise* the allocation for a near-miss. On a near-miss, search for a strategy that never generates the problematic term instead of patching the existing one.

### 27. Use the wastebasket, but archive everything `[STATED]`

Abandonment is a skill: know when to "be persistent and patient" and when to "be pragmatic and realistic". But nothing is deleted — even "embarrassingly wrong" work is stored privately, since it may yield salvageable material or a lesson about a recurring mistake. — <https://terrytao.wordpress.com/career-advice/use-the-wastebasket/> (summarised; quoted fragments are the page's own words)

> "If you have an incomplete (or otherwise unsatisfactory) argument for a problem that you are working on, and you are planning to abandon it, you may still wish to write an informal sketch of it just for yourself." — <https://terrytao.wordpress.com/career-advice/write-down-what-youve-done/>

**Agent:** abandoned branches are written to a retained ledger with their reason, never discarded — and that ledger is fed into the prompt of subsequent attempts, not merely stored on disk.

### 28. Think ahead before sinking effort `[STATED]`

Before proving a lemma, ask what it is for; before executing a technique, ask where it could possibly take you.

> "If the lemma were proven, how would it be used? What features of the lemma are most important for you?" … "These questions can help you reformulate your lemma to its optimal form before sinking too much time into trying to prove it." — <https://terrytao.wordpress.com/career-advice/think-ahead/>

> "it is also a good idea to not apply any given technique or method blindly, but to think ahead and see where one could hope such a technique to take one; this can allow one to save enormous amounts of time by eliminating unprofitable directions of inquiry before sinking lots of effort into them" — [SMP], p. 6

**Agent:** a cheap forward-projection before each expensive attempt, stating what success would yield and whether that closes the goal. Attempts whose success would not advance the goal are cancelled unstarted.

### 29. Keep a portfolio; don't obsess on one big problem `[STATED]`

The chess principle — play opponents slightly stronger than you — applied as a research allocation: low-risk work inside your range, medium-risk work slightly beyond your tools, a small high-risk allocation. Concrete generators he gives: take the simplest unsolved problem in the field or add constraints to a harder one; reprove a known result under restricted methods; generalise a theorem where most steps already work. — <https://terrytao.wordpress.com/career-advice/continually-aim-just-beyond-your-current-range/> (summarised)

> "I would strongly advocate a more balanced, patient, and flexible approach instead: one can certainly keep the big problems in mind, and tinker with them occasionally, but spend most of your time on more feasible 'low-hanging fruit.'" — <https://terrytao.wordpress.com/career-advice/dont-prematurely-obsess-on-a-single-big-problem-or-big-theory/>

Eliminating avenues *is* the work: "by patiently eliminating fruitless avenues of attack, you are setting things up so that when the breakthrough does come, one can conclude the problem in relatively short order", and "there are remarkably few 'Eureka!' moments" (<https://terrytao.wordpress.com/career-advice/be-patient/>).

**Agent:** split the budget across difficulty tiers rather than spending it all on the headline goal; use "reprove a known result with a restricted toolkit" as a self-calibration task; and count eliminated approaches as progress in the run's own metrics.

### 30. Quality is high-dimensional — do not optimise one axis `[STATED]`

He lists twenty-one distinct senses of "good mathematics": problem-solving, technique, theory, insight, discovery, application, exposition, pedagogy, vision, taste, rigour, beauty, elegance, creativity, usefulness, strength, depth, intuitiveness, definitiveness, and more.

> "While each one of the above attributes is generally accepted to be a desirable trait to have in mathematics, it can become detrimental to a field to pursue only one or two of them at the expense of all the others." — *What is good mathematics?*, arXiv:math/0702396, §1

He adds "the remarkable phenomenon that good mathematics in one of the above senses tends to beget more good mathematics in many of the other senses as well" (same section).

**Agent:** do not reduce run success to a single judge score. A run that produced no answer but a clean obstruction, a reusable lemma, or a legible exposition produced something on Tao's list, and the ledger should be able to say so.

---

## E. Collaboration, records and scale

### 31. The quantum of progress `[STATED — house rules, see caveats]`

The unit of contribution is deliberately sized: a real insight, small enough to be absorbed and acted on immediately.

> "An ideal polymath research comment should represent a 'quantum of progress'. On the one hand, it should contain a non-trivial new insight (which can include negative insights, such as pointing out that a particular approach to the problem has some specific difficulty), but on the other hand it should not be a complex piece of mathematics that the other participants will have trouble absorbing." … "once your thought processes reach a point where one could efficiently hand the baton on to another participant, that would be a good time to describe what you've just realised" — *General polymath rules* #6

**Agent:** cap the granularity of writes to shared state — one claim, one obstruction, or one bound improvement per write. The flush trigger is "another agent could act on this now", not "I have finished".

### 32. Half-baked is publishable; the reason for failure is the payload `[STATED — house rules; extension INFERRED]`

> "It's OK for a mathematical thought to be tentative, incomplete, or even incorrect. Often, progress on a mathematical problem proceeds by first eliminating some ostensibly plausible approaches; the reason for the failure of the approach is often instructive, and gives clues as to what the correct approach actually is." — *General polymath rules* #3

Tao's own framing in [Slides]: contributions come from participants "each working for a short period of time before sharing their (not necessarily complete or correct) findings."

**Caveat.** Tao nowhere says, in those words, "record dead ends so they are not re-explored". The nearest signed support is the "negative insights" language above and [Slides]' remark that on genuinely hard problems "one may perhaps get a better insight as to why all known methods fail". The stronger explicit claim belongs to Gowers. Treat the re-exploration rule as `[INFERRED]`.

**Agent:** a `FAILED`/`OBSTRUCTION` verdict is a retained first-class artifact with its reason attached and an explicit confidence field, so speculative leads can be posted without contaminating settled knowledge.

### 33. Three tiers: working thread, control channel, settled store `[STATED — house rules + Tao]`

> "The wiki pages for that project. … will store all the 'settled knowledge' for that project" … "The research thread … Lengthier computations and arguments should be placed on the wiki and summarised on the research thread." … "The discussion thread … is where the project is managed and evaluated" — *General polymath rules* §1-3

Tao applied the split himself when launching Polymath8: "with a wiki page to keep track of all the progress and links to resources, and with a separate thread to discuss administrative issues" — [Retro]. And the overflow rule is explicit: "Once the number of comments here becomes too large to easily digest at once, participants are encouraged to work on the wiki page to summarise the progress made so far" — Tao, <https://polymathprojects.org/2011/07/19/minipolymath3-project-2011-imo/>.

**Agent:** separate the attempt transcript (high-volume, disposable), the claims ledger (curated, promoted on verification), and the control channel (orchestrator summaries and steering). Long derivations go to the workspace and are *referenced* by a one-line summary. Compaction fires on a measured threshold — message count, tokens, elapsed passes — and reseeds a fresh working context from the summary. Scheduled, not discretionary.

### 34. Modularise so no participant needs the whole argument `[STATED]`

Tao names modularity as a reason Polymath8 worked: "a modular structure to the problem, so that people could contribute to one aspect without necessarily being expert with all other aspects" ([Slides]). It "split into five active and loosely interacting components", and the effect was that "no one participant had to absorb the entire 163-page argument at any given time while the research was ongoing" ([Retro]).

The complement, from the house rules: "If you are planning to think about some aspect of the problem offline for an extended length of time, let the rest of us know… the insights that you have are supposed to be shared amongst all of us, not kept in isolation until you have resolved all the difficulties by yourself" (#5).

**Agent:** components with typed interfaces — an improved bound on X is an input to Y — so a specialist improves one without loading the others. "Loosely interacting" is the target coupling: shared numbers, not shared reasoning. Any long-running sub-agent registers an intent record before starting, so others neither duplicate nor block on it.

### 35. The factory production line, and one monotone statistic `[STATED]`

The most directly mechanisable finding: build the evaluation pipeline early, in code, so any local gain propagates automatically to the headline number.

> "We had managed to organise ourselves into a sort of factory production line: an advance in, say, the Type I estimates would be handed over to the combinatorics group to produce a new distributional estimate in primes, which the sieve team would then promptly convert into a revised value of k, which the prime tuples team would then use to update their value of H₁." … "A database had been set up … to automatically record the narrowest known prime tuples for a given value of k" — [Retro]

Polymath8 also had "a easily comprehended way to measure progress (and one which was guaranteed to terminate!)" ([Slides]) — and he records the failure mode of a metric that improves without crossing the threshold that matters:

> "There was then a lengthy and frustrating 'Zeno's paradox' period in which the efficiency M₄ of our sieves kept improving incrementally (from 1.845, to 1.937, to 1.951, ...), but never quite enough to surpass the magic threshold of 2" — [Retro]

It broke only via a trick that deliberately *worsened* the optimised quantity in exchange for a wider admissible class.

**Agent:** cache the best known value of each intermediate parameter in a shared store any agent can monotonically improve, and wire the conversions as code so no model sits in the propagation path. Define one scalar with a guaranteed floor, log every improvement with its cause, and detect the asymptoting regime — a metric converging below threshold means change the search space, not grind. A run posting 0.3% improvements is stalled, not progressing.

### 36. Know where the method stops working `[STATED]`

A triage rule, stated plainly, plus the coordination cost that caps the whole approach.

> "Polymath projects have only made progress on problems where there was already some number of promising ways to make progress, for instance by trying to adapt some arguments already in the literature. For the truly difficult mathematical problems, where all known methods have failed and some genuinely new idea or insight is needed, it doesn't look as if a Polymath project would get much further than an individual mathematician would (although one may perhaps get a better insight as to why all known methods fail)." — [Slides]

Successful projects "invariably needed a project leader to moderate and guide the discussion… This can be very time consuming" ([Slides]), and problems "that require a lot of very specialised and technical mathematical expertise to even comprehend are poor candidates" (same).

**Agent:** triage before spending a run. With no existing partial route to adapt and no legible metric, a multi-agent run will underperform a single deep agent — so set the goal to *characterise why known methods fail*, which Tao counts as a genuine deliverable. Measure orchestration cost explicitly, since it is what caps the run's scale.
