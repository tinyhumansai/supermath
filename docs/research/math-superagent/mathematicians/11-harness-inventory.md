# What this runtime can and cannot do, at branch HEAD

The capability map the ten subject files are compared against. It replaces
[`../tao/03-harness-inventory.md`](../tao/03-harness-inventory.md), which was
written at commit `55e14efd` and predates `lean.rs`, `search.rs`, `refute.rs`,
`weakened.rs`, `closure.rs` and `blueprint.rs` — six modules, roughly 3,800
lines, that between them close five of the eighteen rows in
[`docs/tao-gap-analysis.md`](../../docs/tao-gap-analysis.md).

Every claim here carries a `file:line` or a path that was opened while writing
it. A stale inventory silently invents gaps that were closed last week, which is
the failure this file exists to stop; §11 records where the *documentation* has
drifted from the code, in the habit `../tao/03` established.

Read against [`00-conventions.md`](00-conventions.md) for the status vocabulary
— **Absent**, **Unenforced**, **Unused**, **Partly**, **Present**.

---

## 1. The roles — twenty-two, not nineteen

`orchestrator_prompts.rs:116-141` is the authority. `by_role()` lists:

`orchestrator`, `goals`, `research`, `tool_builder`, `coder`, `sat_solver`,
`smt_solver`, `theorem_prover`, `symbolic_math`, `lean_prover`, `reflection`,
`judge`, `pattern_finder`, `inventor`, `reducer`, **`weakener`**,
**`searcher`**, **`refuter`**, `librarian`, `scholar`, `context_curator`,
`director`.

The three in bold were added on this branch. `CLAUDE.md` and `docs/roles.md`
both still say nineteen; see §11.

Prompt text is one Markdown file per role under `src/prompts/`, plus
`method_policy.md`, which is prepended to every prompt in every role and must
lead, because the provider cache is keyed on the prefix
(`orchestrator_prompts.rs`, `RolePrompts::load`). There is also a per-workspace
override path, `prompts/<role>.md` inside the workspace — live today in
`workspace/conjectures/erdos-gyarfas/prompts/`.

**Bearing on the subject files.** `weakener` is Tao §1 (turn off nine of ten
difficulties) made into a role. Nothing is Grothendieck's A4 or Scholze's A1 —
change the ambient setting and keep the goal — which is the single most-repeated
absence across `01`, `06`, `08` and `09`.

## 2. The solution loop

Authored declaratively as a `WorkflowGraph` in `workflow.rs`; node bodies live in
`loop_steps.rs`; the routing policy is a pure function in
`solutions_attempt.rs`, and `parity.rs` proves the engine's jq agrees with it
exhaustively.

Thresholds, all in `solutions_attempt.rs`:

| Constant | Value | Line |
|---|---|---|
| `MAX_ATTEMPTS` | 8 | `:6` |
| `STUCK_THRESHOLD` | 2 | `:8` |
| `BLOCKED_THRESHOLD` | 2 | `:15` |
| `RESEARCH_RESCUE_ATTEMPTS` | 5 | `:26` |
| `COMPUTATIONAL_THRESHOLD` | 2 | `:41` |
| `UNVERIFIED_THRESHOLD` | 2 | `:62` |
| `REDUCTION_INTERVAL` | 3 | `:80` |
| `MAX_RESTARTS` | 2 | `:91` |

`Route` (`solutions_attempt.rs:383-395`) has five variants: `Solved`,
`Reported`, `Retry`, `Diversify`, `Blocked`. `Blocked` is checked first, before
the attempt ceiling, because a provider outage is not a mathematical result.

Everything after an attempt is a concurrent fan-out converging on one merge that
folds counters by delta. The arms, in `solutions_routing.rs`:
`diversify_library_arm` (`:65`), `diversify_pattern_arm` (`:104`),
**`refutation_arm`** (`:144`), **`novelty_arm`** (`:201`),
`diversify_invention_arm` (`:263`), plus the reduction arm inside which
`reducer` and `weakener` run concurrently.

**Bearing.** Four subjects independently attack these thresholds.
`STUCK_THRESHOLD = 2` diversifies away from Wiles's six months of orientation
(`04`§A1) and from Grothendieck's rising sea (`01`§A1/A10), both of which
require many consecutive attempts that do not move the goal. Nothing in the
`Route` vocabulary can express "the goal did not move and that is expected".

## 3. Reflection verdicts, and what can end a run

`solutions_judging.rs:551-563` holds the verdict contract. Four verdicts:

- **`SOLVED`** — requires a specific final answer *and* verification by a second
  independent route. Rejected if the workspace contains no program
  (`:979`) and rejected if the same reflection reports `PROGRESS: NO` (`:989`).
- **`UNVERIFIED`** — a specific final answer reached by exactly one route, with
  no second route available. Ends the run.
- **`BANKED`** — added on this branch. The attempt did not reach the goal but
  settled something. Never ends the run; honoured only when the established
  claim set actually grew, read off disk, so it cannot be asserted into
  existence.
- **`UNSOLVED`** — otherwise.

Reflection also answers `KIND: MATHEMATICAL | COMPUTATIONAL | NONE`; two
consecutive `COMPUTATIONAL` routes to diversify.

**Bearing.** `SOLVED`'s requirement of a program on disk is a *constructive*
standard, and Erdős's 1947 Ramsey bound would fail it (`02`§A4). Thurston
(`05`§A1) would say every one of the four checks is a check on the answer and
none on whether anything was understood. Gowers's `KIND` distinction
(`03`§A5) is finer than the runtime's and applies *before* the attempt rather
than after.

## 4. The claim ledger and its statuses

`claims.rs` (1,097 lines) derives `research/CLAIMS.md` (`CLAIMS_PATH`, `:35`)
from fenced `claim` blocks in notes. `Status` (`:132`) has six variants:

- `Proved` — *the source* proves it. A statement about somebody else's paper.
- **`Formalised`** — the Lean kernel checked it, in this run, in this workspace.
  Added on this branch, and the module doc states its distinguishing property
  plainly: "It is the only status the ledger does not take on trust. Every
  other one is a word a model typed into a note." Dropped to `Asserted` unless
  `lean::verdict` finds a passing kernel verdict for the file the claim names.
- `Checked` — this run checked it numerically.
- `Asserted` — the default.
- `Heuristic` — suggestive rather than established.
- `Catalogued` — read out of a catalogue. Its doc comment carries the Project
  Euler 241 incident that motivated it: a correct answer produced by summing a
  hardcoded OEIS b-file sat on disk beside a derivation that was wrong for 5 of
  9 terms below 10⁸, and nothing distinguished the two files.

**Bearing.** This is the schema every subject file wants to change, and they
want opposite things. Zeilberger (`06`§A1) wants a *number* — a price that
composes additively along deduction — where this is categorical. Arnold (`09`§A3)
wants derivation *depth* carried, because a long chain is less reliable.
Ramanujan (`07`§A1) wants somewhere to put a believed statement with no
justification at all. Scholze (`08`§A2) wants a targeting rule saying which
claims deserve `Formalised`, which nothing supplies.

## 5. The derived ledgers

Written by code, never by an agent, re-derived from disk. Nine now:

| Path | Module | Lines |
|---|---|---|
| `research/CLAIMS.md` | `claims.rs` | 1,097 |
| `research/THREADS.md` | `threads.rs` | 342 |
| `research/APPROACHES.md` | `approaches.rs` | 396 |
| `research/BACKWARD.md` | `backward.rs` | 724 |
| `research/FRONTIER.md` | `frontier.rs` | 457 |
| `research/REQUESTS.md` | `requests.rs` | 302 |
| **`research/WEAKENED.md`** | `weakened.rs:49` | 827 |
| **`research/BLUEPRINT.md`** | `blueprint.rs:44` | 687 |
| **`research/ENTAILMENT.md`** | `closure.rs:47` | 546 |

The last three are this branch's. `blueprint.rs` derives a node per goal, lemma
and claim, with standing as the minimum over what it rests on, a **ready** list,
and a cycle report above everything else. `closure.rs` derives the entailment
closure from a `follows-from:` field, closed to a fixed point rather than one
hop.

The search board is separate: `search.rs` writes `code/search/SEARCH.md`
(`:85`) off a `scores.jsonl` ledger (`:82`), with `score.py` (`:76`)
unreachable by the `searcher` role.

**Bearing.** `research/BACKWARD.md` is Perelman's gap (`10`§A1) already in the
right shape — an `id`, a `lemma`, a `status`, and a first move a `tool_builder`
could run today — and it is workspace-local, so nothing outside the run can pick
one up. `research/APPROACHES.md`'s lifecycle (`proposed → grounded → refuted /
adopted / spent`) is where Wiles's revivable abandoned approach (`04`§B1) and
Ramanujan's repairable refuted claim (`07`§B2) both fail: `refuted` and `spent`
are absorbing.

## 6. Verification: what is actually checked in code

Three kernels, and this is where the branch changed most.

- **`lean.rs`** (507 lines) runs `lean`, parses compiled / `sorry` /
  `#print axioms`, files a verdict under `code/out/lean/`. Granted to
  `lean_prover` alone. The check runs at **ledger-derivation time, not write
  time**, so a claim whose Lean file is later edited into a `sorry` loses its
  standing on the next derivation.
- **`refute.rs`** (493 lines) parses Vampire's SZS status into four findings.
  Vampire 5.1.0 is in the image, pinned; its `--saturation_algorithm fmb` does
  finite model building, which is what answers a false conjecture instead of
  timing out on it. `ContradictoryAxioms` is a status the runtime reads, not a
  prompt instruction.
- **`search.rs`** (747 lines) scores candidates the `searcher` submits. The role
  has no file-write tool and no shell, so `submit_candidate` is its only route
  to disk — the AlphaEvolve verifier-exploitation finding turned into a tool
  boundary.

**Bearing.** Scholze (`08`§A6) is the best evidence in the directory *for*
`lean.rs`, and for a property it was not built to have: he reports that
formalising taught him which step his proof turned on, after a year in which he
could not tell. `lean.rs` files a verdict — a boolean — and there is nowhere for
that sentence to go. Arnold (`09`§A5) is the objection: a Lean file proves what
its *statement* says, and nothing checks that the statement encodes the intended
mathematics.

## 7. Research gating

`orchestrator_registry.rs:1-21`. `search_tools()` returns `SearchTools::default()`
when research is off, so `exa_search` and the OEIS adapters are withheld **by
not being registered**. The module doc states the principle: "Both are withheld
by not registering them rather than by asking the model to abstain, because a
prompt instruction is not a control."

**`download_document` is still not gated.** It is `document_tools[0]`
(`orchestrator_registry.rs:45-46`), an ungated array of eleven granted broadly.
This is `docs/tao-proposals.md` #12's first item and it remains open: a one-line
move plus a test, restoring a control that reads as complete and is not.

## 8. Computation

`execute_command`, `write_tool_file` and `apply_patch` go to the code-writing
roles. `/workspace/code` is on `PYTHONPATH`. The solver stack in the image: E,
Z3, cvc5, MiniSat, CryptoMiniSat, CP-SAT, PySAT, PuLP, CBC, GLPK, Lean 4 with
Mathlib, and Vampire.

`method_policy.md` opens by requiring computation before prose: "Understand by
computing, not by writing prose about the problem. … a restatement that has
never been executed is an untested guess."

**Bearing.** This is the most contested single line across the ten files.
Arnold (`09`§A1/A4) endorses it and wants it *continuous* rather than once at
intake. Gowers (`03`§A3) deliberately refuses the machine's speed. Five of the
ten central results in §B sections across this directory — Grothendieck's four,
Wiles's, Scholze's LTE, Perelman's — involved no computation at all.
`COMPUTATIONAL_THRESHOLD` catches a run scaling one method; nothing catches a
run reaching for computation when the problem is not yet parameterised.

## 9. What has no representation at all

The list the gap analysis is built from. Each is **Absent** unless noted.

1. **A change of ambient setting.** `inventor` re-routes to the goal, `reducer`
   finds sufficient lemmas, `weakener` lowers the target. None restates the goal
   in a different category, encoding or ambient structure. — `01`§A4, `06`§A4,
   `08`§A1, `09`§A6
2. **A move ladder ordered by safety.** Routing reads *state*; nothing enumerates
   the applicable moves and orders them by probability of appearing in the final
   argument. — `03`§A1
3. **Overshoot.** No measure of whether a run's output exceeds its goal. —
   `01`§A2, `02`§A2
4. **An inherited-hypothesis check.** `research/BACKWARD.md` records `rests-on`
   per gap, so a hypothesis no step below cites is detectable. Nothing looks. —
   `01`§A3, `03`§A10, `08`§A7. *Three subjects, one graph walk.*
5. **A place for an unjustified belief.** Every ledger schema demands
   justification structure at write time; `note_scratch` is unreachable from
   durable recall. — `07`§A1, `03`§A11
6. **A consumer for refuted work.** `refuted` and `spent` are absorbing states.
   — `04`§B1, `07`§B2
7. **Difficulty estimation.** Not for the goal, not per gap, not per approach. —
   `02`§A3
8. **A formalisation targeting rule.** `blueprint.rs` computes in-degree over
   the statement graph and nothing reads it as a verification priority. —
   `08`§A2
9. **Quantifier structure on a goal.** — `08`§A3, and `02`§B1 records it going
   wrong
10. **Legibility as a scored property.** `research/ROOT.md` — "what the library
    means" — is agent-written and read by no verdict. — `05`§A1, `05`§B1
11. **Derivation depth or price in the ledger.** — `06`§A1, `09`§A3
12. **Cross-workspace reading of a `BACKWARD.md` gap.** — `10`§A1, and
    `docs/tao-proposals.md` #6

## 10. Unused: the judge's score

`solutions_judging.rs:49` pushes the judge's 1–5 score onto `state.scores`. The
only other references are serialisation (`solutions_attempt.rs:244`) and
rebuild-from-state (`:303`). **No policy reads it.** This row was **Unused** in
`../tao/03` and is still Unused.

`../tao/01`§35 and `10`§A2 both argue a long programme needs one monotone
statistic. The runtime writes a number every judging cycle and acts on none of
it — and Perelman's entropy is the reminder that the statistic worth having is
about the mathematics, not about the run's conduct.

## 11. Where the documentation has drifted from the code

Verified against the working tree while writing this file.

- **The role count is current, and was not when `../tao/03` was written.**
  `AGENTS.md:23` and `docs/roles.md:3,52` both say twenty-two, matching
  `orchestrator_prompts.rs:116-141`, and `docs/roles.md` covers `weakener`,
  `searcher` and `refuter`. Likewise the ledger count: `AGENTS.md:33` says nine,
  matching §5. Both were fixed on this branch. Recorded here because
  `../tao/03` is stale on both and a reader comparing the two files needs to
  know which is which.
- **`organizer` is documentation residue.** It is not in
  `orchestrator_prompts.rs:116-141` and is not registered, and `docs/roles.md`
  still gives it a tool-boundary paragraph (`:322`) and a context-routing row
  (`:349`), while `docs/workspace.md` uses it as the actor in four separate
  arguments (`:69`, `:85`, `:99-103`, `:334`). The `:99-103` passage is a live
  run's lesson attributed to a role that no longer exists. Trust
  `orchestrator_prompts.rs` for the roster.
- **The Tao documents are not linked from `AGENTS.md`.** Its *Where the rest of
  this lives* list runs `roles`, `solution-loop`, `routing`, `runtime`,
  `workspace`, `ledgers` — and neither `docs/tao-gap-analysis.md` nor
  `docs/tao-proposals.md` appears. By `AGENTS.md`'s own rule, a document with no
  rule above it is a document nobody has a reason to open, so both are currently
  unreachable from the working agreement.
- **`docs/tao-proposals.md`** listed #8 (entailment closure) and #9 (post-solve
  novelty check) as *Not built* while `closure.rs`, `blueprint.rs` and
  `novelty_arm` (`solutions_routing.rs:201`) exist. A concurrent session was
  mid-repair on that file when this inventory was written; check its state
  before citing it.
- **`docs/solution-loop.md`** flags an open issue of its own: four of five
  standing teams duplicate a loop arm, and the duplication was never chosen. It
  says it should be resolved by measurement, and it has not been. Wiles
  (`04`§A2) is an argument for doing that sooner — observation has a cost the
  runtime has never measured.
