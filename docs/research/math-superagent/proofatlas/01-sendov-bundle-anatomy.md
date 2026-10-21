# ProofAtlas' Sendov bundle, read as a harness

Source: `SENDOV_CONJECTURE_PROOF_PUBLIC_BUNDLE_2026-08-05.zip` from
proofatlas.ai (3.4 MB zip, 12 MB extracted, 1,272 files), dated 5 August 2026.
Lech Mazur curated; the disclosure names OpenAI GPT-5.6 Pro as the model and
says generative AI did discovery and derivation, not only exposition.

This note is not about Sendov's conjecture. It is about the *shape* of the
artifact, because that shape is an answer to the question this repository keeps
asking: what does a run have to produce before anyone should believe it.

## What is in the bundle

Three trees, and the boundaries between them are stated rather than implied.

| Tree | Content | Measured |
| --- | --- | ---: |
| `lean/` | Lean 4 project proving the conjecture | 1,176 files, 121k lines |
| `reproducibility/` | paper, exact certificates, Python replay | 14 programs, 2 data files |
| root | `SHA256SUMS`, `VERIFY_RELEASE.sh`, manifests | — |

Inside the Lean project:

| Layer | Files | Lines | Role |
| --- | ---: | ---: | --- |
| `Sendov/Optimized/` | 35 | 7,234 | the mathematics — algebra, analysis, geometry |
| `Sendov/Scalar/` | 18 | 3,293 | scalar estimates, the master error theorem |
| `Sendov/Certificate/` | 1,119 | 110,406 | certificate data and its checkers |

Nine tenths of the formalisation is certificate. The *argument* is 10k lines.

## The five controls worth stealing

### 1. The audit is a printed artifact, not a claim

`SendovAudit.lean` is twelve lines and does one thing: `#print axioms
Sendov.sendov_conjecture`. `LEAN_AUDIT.txt` records the command and its literal
output — `[propext, Classical.choice, Quot.sound]` — beside the build command,
its resource limits (`LEAN_NUM_THREADS=1`, 32 GiB `prlimit`, `timeout 1200s`),
and its output (`Build completed successfully (4524 jobs)`). Zero `sorry`, zero
`axiom` declarations across 1,176 files; both verified here by grep.

We already do this bit: `src/orchestrator/lean.rs` reads the axiom list and
`native_decide`'s toolchain-specific axiom name is refused by test. Confirmed:
the bundle uses **`decide +kernel` 1,010 times and `native_decide` zero times**
— the same line we draw, drawn independently.

### 2. Generated data is untrusted *in the file that carries it*

Every one of the 559 modules under `Certificate/Generated/` opens with:

```
THIS FILE IS GENERATED. DO NOT EDIT.
Packet block SHA-256: `2992…`
All declarations below are untrusted certificate data. Mathematical conclusions
must pass through the human-written replay and block checkers. Digests,
generator assertions, and minimality of the generated values are not theorem
premises.
```

Then a hand-written checker consumes it and the kernel reduces the Boolean:

```lean
theorem finiteReplay499Check :
    degreeReplayCheck finiteReplayInput499 = true := by
  decide +kernel
```

The trust boundary is: *data may be machine-generated and wrong; the predicate
that reads it, and the theorem that the predicate's truth implies the
mathematics, are written by hand and proved.* `FiniteSchema.lean` states the
structure and proves `ordinaryLeafContextCheck_spec : check = true ↔ Context`;
`FiniteSoundness.lean` (49 kB) proves that a passing leaf really bounds the
analytic quantity. Everything else is arithmetic the kernel does.

### 3. The certificate is verified twice, by two stacks that do not share code

The Python tree checks the two terminal scalar propositions with exact
`Fraction` arithmetic. The Lean tree proves the whole conjecture, embedding the
same finite data as Lean source. Each document says plainly what it does *not*
cover:

> Lean checks this formal proof and its embedded exact certificate data. It does
> not execute or certify the Python programs. Conversely, the Python programs do
> not check the complex-polynomial reduction. The two packages therefore provide
> distinct, complementary evidence.

Two independent witnesses, and a written statement of the gap between them.

### 4. Negative controls run in the replay

`tests/run_mutation_tests.py` corrupts the certificate seven ways and requires
each corruption to be rejected, with the rejecting diagnostic printed:

```
PASS: rejected noncanonical_fraction: noncanonical fraction token: '2/40'
PASS: rejected extraneous_degree:     unexpected leaf count: 16863
PASS: rejected deleted_leaf_gap:      unexpected leaf count: 16861
PASS: rejected duplicate_leaf:        duplicate serialized row at 2
PASS: rejected wrong_depth, reversed_interval, wrong_alg_start
PASS: degree-333 left-edge thin-leaf regression
```

A verifier that accepts everything passes every positive test. These are the
tests that catch it, and they are part of `REPRODUCE.sh`, not a side script.

### 5. Identity is fixed before anyone argues about content

The README pins seven SHA-256 digests (paper TeX, paper PDF, both
certificates, the input manifest), the accepted source commit, the Lean
toolchain (`v4.30.0-rc2`), and the Mathlib commit. The audit then records a
*public-source equivalence check*: 1,162 modules in the theorem's import
closure, 1,080 files differing byte-wise from the accepted commit, **0
non-comment differences** — the public cleanup touched comments only, and that
was proved by a comment-aware normalising diff rather than asserted.

Runtime logs are deliberately excluded from the internal manifest so later
replays can be appended without changing the proof inputs; the detached zip
digest binds the supplied log. That is a careful distinction, not boilerplate.

## What the mathematics looked like as a workflow

The paper's architecture is a reduction with a computational tail, which is the
shape our harness is built for and mostly does not reach:

1. one scaling/rotation normalises a hypothetical failure (`NormalizeFailure`);
2. an algebraic identity chain — reciprocal coordinates, antiderivative
   polynomial, division-free coefficient bridge, double-defect error identity;
3. an error majorant and a centre estimate reduce everything to **one scalar
   inequality**, `Scalar.masterError_strict`;
4. that inequality splits into a *finite* regime (`5 ≤ m ≤ 499`, 16,862 exact
   rational leaves from an interval subdivision of maximum depth 13) and a
   *large-degree* regime (elementary estimates plus a compact Bernstein
   certificate: 162 positive coefficients, 20,994 node boxes, 6,500 checks);
5. degrees 2–5 fall to a single AM–GM comparison.

The AI's leverage is visible in the file counts: the human-shaped work is the
10k-line reduction and the checker specs; the 110k lines of certificate are
machine-generated and machine-checked. The bet is that a proof is worth
attempting when it can be *pushed into that shape* — everything hard reduced to
one scalar statement, and that statement split into a finite pile a kernel can
grind and a tail an estimate can cover.

## The strategy, in detail

Assume a counterexample, normalise it until two real numbers describe it, derive
a lower bound on a scalar from the obstruction, prove the strict reverse upper
bound, and collide them.

**1. One-step normalisation (§2).** Suppose Sendov fails for `p` at zero `α`. Two
cheap kills first: if `p'(α)=0` take `w=α`; if `α=0` Gauss–Lucas puts a critical
point in the disk. So `α ≠ 0` and is simple. Let `d = min_j |w_j − α| > 1`, set
`ρ = 1/d < 1` and `s = ρ·ᾱ/|α|`, and put `P_*(z) = p(s⁻¹z)`. One invertible
change of variable yields a **normalised obstruction**: distinguished zero real
with `0 < a ≤ ρ < 1`, every zero inside radius `ρ`, every critical point at
distance exactly `≥ 1` from `a`. All complex-geometric freedom is gone in one
move, and multiplicities are preserved — so repeated roots never need a case.

**2. Reciprocal coordinates and antiderivative algebra (§3–5).** Set
`q_j = 1/(a − w_j)`. The separation `|w_j − a| ≥ 1` becomes `0 < |q_j| ≤ 1` — the
obstruction is now a unit-disk constraint on the `q`'s. Since `a` is simple,
`p'(a+z)/p'(a) = ∏(1+q_j z)`, and integrating gives

```
p(a+z)/p'(a) = z·A_q(z),   A_q(z) = ∫₀¹ ∏(1+t q_j z) dt = Σ_k e_k(q)/(k+1) · z^k
```

so the zeros of `A_q` are `z_i − a`. Two evaluations of the cleared factorisation
give the exact product identity `J = (−1)^m Q Z`, hence `|J| = P·R < 1`.

The workhorse is an elementary **product defect** lemma:
`Defect(u) = Σ(1−|u_i|²)∏_{ℓ≠i}u_ℓ = e_{k−1}(u) − U·conj(Σu_i)`, and if every
`|u_i| ≤ 1` then `|Defect(u)| ≤ 1 − |∏u_i|²`. Five-line induction, slack
`(1−r)(1−R)(1−rR) ≥ 0`. Applied to the `q` family *and* the `z` family — the
"double defect" — and after an integration by parts (`aTJ = n(1−K+E)`) it gives

```
| n(1+E) + aJ(μ − ma − 1/a) | ≤ (an/m)(1 − j²)
```

A Maclaurin plus Cauchy–Schwarz chain bounds the error term, `|E| ≤ E_m(λ)`,
where **`λ = m(1−a)` is the single scalar everything collapses onto**. A centre
estimate `|W| ≤ m²a` follows from an explicit algebraic identity engineered so
its residual bracket vanishes identically (because `m(1−a) = λ` and
`(1−h₀)(λ+3) = λ`) and every remaining term is visibly nonnegative.

Net: any obstruction with `m ≥ 5` must satisfy `1 + λ ≤ (m+1)·E_m(λ)`.

**3. The master error theorem (§7).** The strict reverse,
`(m+1)E_m(λ) < 1+λ` for `m ≥ 5`, `0 < λ < λ_m`, split by regime.

*`5 ≤ m ≤ 499`, `λ ≥ 1/20` — the exact certificate.* Two structural facts do the
work: `B_{m,λ}` increases in `λ`, and `√B` is convex in `t`. So on a box `[L,U]`
majorise `f` by the secant through nine dyadic nodes `j/8` with integer ceilings
`ν_j = ⌈2⁶⁴·f_{m,U}(j/8)⌉`. Exact beta integration turns the integral into
geometric sums of integer powers, and the leaf test becomes **one strict integer
inequality**, `p²·N·s < q²·8²·σ^{m−1}·(s+r)`. The partition is deterministic —
start at `[1/20, λ̲_m]`, test, on failure bisect at the exact rational midpoint,
left child first — which is what makes the tree a certificate rather than a
search log. 16,862 leaves, max depth 13. The endpoint gets its own `ALG` leaf,
since at `λ = λ_m` the radicand is a perfect square `(1−a_*t)²`.

*`m ≥ 500`.* Elementary analytic estimates over three ranges of `λ` (`≤1/3`,
`1/3..60`, `≥60`), plus one uniform auxiliary bound `R(y,c) < 481/500` certified
by the compact Bernstein certificate.

**4. Degrees 2–5 (§8).** One weighted AM–GM. With `X(t) = a + (1−a²)t` and
`J_n(a) = ∫₀¹X^m`, only `J_5` is needed:
`1 − J_5(a) = (1−a)³(1+a)(a⁴−3a³+3a+4)/5 > 0`, and `θ = m/4` gives
`J_n ≤ 1 − (m/4)(1−J_5) < 1`, contradicting `1 ≤ ∫c^m`.

### Three habits that made it formalisable

- **Nothing is divided by something that might vanish.** The paper says so
  repeatedly — *no division by `J`, a root, or a product occurs*, *`j = 0` is
  retained*, *only the positive scalar `n²` is cancelled*. Everything stays
  cleared and polynomial, which is why the Lean version needs no nonvanishing
  side conditions and why repeated roots and boundary zeros cost zero cases.
- **Degeneracy is removed by normalisation, not case analysis.** One scaling
  handles what would otherwise be a tree of special positions.
- **The endgame was designed to be decidable before it was computed.** Choosing
  `λ = m(1−a)`, engineering monotonicity-in-λ and convexity-in-t into a secant
  majorant, scaling by `2⁶⁴` to make ceilings integers — each choice aims at
  turning "prove an inequality about an integral" into "compare two integers".
  The transferable target is not *a scalar inequality*; it is *a scalar
  inequality whose leaf test is one integer comparison*.

## Where our flow falls short

Ordered by how much it would change what a run produces.

**a. We have no certificate/checker separation in Lean.** Our `lean_prover`
writes statements and proofs; nothing in the harness expresses "generated data
+ hand-written predicate + `decide +kernel` + a soundness theorem". Sendov's
proof is 90% that pattern. A run that computes 16,862 anything today writes it
to `code/out/` and *cites* it in prose — asserted, not established. Concretely:
teach the mill a **certificate shape** — a role emits `Generated/*.lean` marked
untrusted, `lean_prover` owns the checker and the `check = true ↔ Spec`
soundness lemma, and `lean_check` refuses a generated module that no checker
consumes. This is the single largest gap.

**b. We do not run negative controls on our own verifiers.** `refute.rs` looks
for counter-models of a conjecture; nothing mutates a certificate we produced
and demands rejection. Every enumeration a run files ("no 99-graph with
property X up to n") rests on an unchecked verifier. A mutation pass belongs
beside `verify` in the ranking, and its output belongs in the ledger row.

**c. Nothing produces a release bundle.** We commit workspaces, which is more
than most, but there is no `VERIFY_RELEASE.sh`, no `SHA256SUMS` over the
artifacts a claim depends on, no pinned toolchain digest recorded beside a
verdict. `derived/CLAIMS.md` says a claim is `verified`; it does not say *by
which Lean, against which Mathlib, over which bytes*. Recording the toolchain
and input digests in the claim row is cheap and makes a verdict re-checkable a
year later.

**d. Our ledgers have no "boundary" field.** Both of their documents state what
their evidence does *not* cover, in the same breath as what it does. Our
`CLAIMS.md` `holds-here` column is close but is about hypotheses, not about
which artifact witnessed the claim and what that artifact ignores. A
`witnessed-by` / `does-not-cover` pair would stop the failure where a Python
run and a Lean file are read as one piece of evidence.

**e. The split they used is the one our roles should target.** `lean_prover`
currently decides what to state next largely from what is nearby. The Sendov
shape suggests a standing instruction with teeth: *drive toward one scalar
statement plus a finite pile*. When a run cannot name the scalar statement it
is trying to reduce to, that is a `FRONTIER.md` entry, not a prompt for more
prose.

**f. Scale, stated plainly.** 4,524 build jobs, one Lean worker, 32 GiB address
space, 20 minutes. Our container ceiling is 8 GiB and that is fixed. A
Sendov-shaped certificate does not fit in it. Either the certificate arm gets
its own resource envelope outside the run container, or the harness should
refuse to chase proofs of this shape and say so rather than time out.

## Would one of our agents come up with this?

Not the strategy as a whole. The useful part is which steps split which way.

**Within range, individually.** The normalisation and the reciprocal coordinates
are both in the literature lineage (Tao 2022, Dégot) that a librarian would pull,
and `inventor` is already asked for "a transform, an invariant". The defect lemma
is an invented elementary inequality with a five-line induction — a model can
produce it and `lean_prover` can check it. The centre identity is a
polynomial-certificate search that `symbolic_math` suits better than a human
does. The finite certificate is deterministic bisection plus an integer
predicate, which is `tool_builder` work. Five of the nine steps, reachable.

**Out of range, and these are load-bearing.**

1. **No intermediate step pays.** Steps 2–7 produce nothing independently
   rewarding — an identity whose consequence is six steps away. The loop scores
   per attempt; a turn ending with `aTJ = n(1−K+E)` had no verdict to earn, so
   the judge routed away long before the chain closed. Our own evidence: every
   open gap in `casas-alvero/derived/BACKWARD.md` ends *no thread — nothing is
   attacking this*.
2. **The reducer had no quantitative target.** It asks what would be enough and
   answers with decompositions — and every skeleton that run produced is a
   restatement of Schaub–Spivakovsky, Ghosh, or Macaulay. Real work; not
   "collapse this onto `λ = m(1−a)` and fight there", which nothing in the role's
   vocabulary could express.
3. **Nobody was charged with designing for decidability.** Our solver roles
   *settle* a finite question. None was asked to *construct* one whose settlement
   implies the theorem. That inversion is the trick.
4. **Nowhere to put the computational third.** No certificate arm, so 16,862
   leaves could only be filed as `asserted` prose.
5. **Scale.** 121k lines, 4,524 build jobs, 32 GiB against a then-16 GiB cap.
6. **The human did the part we had automated away.** Mazur selected and
   reconciled outputs across months. `archivist` adopts a candidate; nothing
   carried a thesis between runs.

**The caveat that cuts the other way.** This is a survivor — the filtered output
of an unknown number of dead ends with a human choosing at every fork. "Would the
model come up with this" and "did this bundle come out of that model" are
different questions and only the second is answered. The ceiling was never "our
agents cannot do mathematics"; the skeletons above are real mathematics. It was
that the loop would not hold a nine-step chain together long enough for the
payoff, and had no home for the endgame.

## What was built in response

Five controls, against the six failures above. Each is a control rather than a
prompt instruction, which is the standard this repository holds itself to.

| Gap | Control | Where |
| --- | --- | --- |
| 2 | `reductions` ledger — a target names its parameter and carries both bounds as separate fields | `ledger/registry.rs`, `derived/REDUCTIONS.md` |
| 1 | `identity` status — banks a chain link that is not yet a result | same ledger; `goals.md` reports it so the judge can score it |
| 4 | certificate arm — a `theorem` under `Generated/` fails the verdict; data no checker reads is reported | `lemmas.rs`, `lean.rs`, `derived/LEMMAS.md` |
| — | clearing discipline — statements dividing by an unguarded denominator are named, advisory | `lemmas::uncleared_divisions` |
| 6 | `thesis` ledger — the standing bet, `refuted-by` required, persists across runs | `ledger/registry.rs`, `derived/THESIS.md` |

Gap 5 was addressed separately by raising the container to 24 GiB with explicit
swap and the per-agent run ceiling to an hour
([`../../docs/runtime.md`](../../docs/runtime.md#the-memory-cap)).

Gap 3 — designing the endgame to be decidable — is the one **not** built as a
control, and the reason is worth recording rather than hiding. There is no
mechanical test for "this finite question, once settled, implies the theorem";
that judgement is the mathematics. It is instead written into `reducer.md` as the
target to aim at, with the Sendov construction as the worked example. That makes
it a prompt instruction, which this repository is on record as not counting as a
control — so it is the weakest of the five, and the one to revisit if a run
produces certificates that check out and prove nothing anybody wanted.

Two more from the bundle remain unbuilt and are worth their own pass: **mutation
controls** on certificates the run itself produces (a verifier that accepts
everything passes every positive test), and a **release bundle** recording which
Lean, which Mathlib, and over which bytes a `verified` claim was earned.

## Verified locally, for the record

```
1,176 .lean files, 120,999 lines; 1,056 theorem, 55 lemma, 19,050 def
0 occurrences of `sorry`; 0 `axiom` declarations (2 hits, both in SendovAudit
  comments and its `#print axioms` line)
0 `native_decide`; 1,010 `decide +kernel`
523 `norm_num`, 112 `nlinarith`, 110 `by decide`
```

Neither `lake build` nor `REPRODUCE.sh` was run here — the Lean build needs the
pinned toolchain and roughly twenty minutes at 32 GiB, and the extended Python
replay regenerates all 16,862 leaves. The bundle's own log records both passing.
