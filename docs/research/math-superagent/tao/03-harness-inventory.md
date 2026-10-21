# What this harness can and cannot do

An evidence-backed capability map of the Riemann mathematical problem-solving harness, at commit `55e14efd` on branch `tao-patterns`. Every claim carries a `file:line`. Where a capability is absent, this says so and names where it would live rather than reading the design charitably.

The documentation here is unusually good — `docs/roles.md`, `docs/solution-loop.md`, `docs/runtime.md` each argue their design from live-run evidence. It is also, in several load-bearing places, describing a runtime that no longer exists. §10 lists every divergence found. Where the docs and the code disagree, this follows the code.

Paths are relative to `src/` unless stated otherwise.

---

## 1. Roles: the eighteen specialists

`default_registry` registers eighteen delegate roles (`orchestrator_registry.rs:44-152`), plus the `orchestrator`, which is the root agent and not a delegate. `RolePrompts::by_role` is the canonical nineteen (`orchestrator_prompts.rs:88-110`).

Three shared bundles do most of the work:

- **`document_tools`** (11): `download_document, read_document, write_document, edit_document, index_document, search_documents, list_workspace, describe_file, refresh_index, search_claims, request_research` (`orchestrator_registry.rs:45-61`).
- **`memory_tools`** (3): `recall_memory, remember_memory, relate_memory` (`:66`).
- **`SCRATCH_TOOLS`**: `note_scratch, recall_scratch` (`orchestrator_prompts.rs:126`); read-only half at `:130`.

Every role receives `AGENTS.md` — the method policy — and nothing else universally (`UNIVERSAL_CONTEXT`, `orchestrator_prompts.rs:135`). Per-role context is `role_context` (`:141-255`).

Models: every definition is `.with_model("openrouter")`; the real split is `REASONING_ROLES = ["inventor","reducer","judge","reflection","director"]` (`orchestrator_core.rs:164`), resolved by `SupportAgents::model_for` (`orchestrator_agents.rs:201-207`). Default `deepseek/deepseek-v4-flash-0731` (`agent/mod.rs:32`), reasoning `deepseek/deepseek-v4-pro` (`:54`). **No temperature is set for any role anywhere in `src/`.**

| Role | Tools beyond documents+memory | Prompt context (+`AGENTS.md`) | Forbidden |
|---|---|---|---|
| `orchestrator` | spawn/await/peek/steer over 15 `DELEGATES` (`orchestrator_core.rs:167-183`) | config.toml, GOAL, TASKS, code/lib/INDEX, CLAIMS, THREADS, APPROACHES, BACKWARD, CONTEXT (`:143-155`) | shell, file write, `apply_patch`, `exa_search`, scratch (`orchestrator_agents.rs:60-63`) |
| `goals` | spawn/spawn_agents/peek/steer/await/await_agents over 13 `SPECIALISTS`; scratch write | same nine files | shell, `write_tool_file`, `apply_patch`, `exa_search` |
| `research` | `exa_search`, `oeis_lookup` (gated) | GOAL, CLAIMS, THREADS, APPROACHES, FRONTIER, CONTEXT (`:185-192`) | **no delegation** — "research holds no delegation tools, so nothing it is handed can spawn further" (`orchestrator_core.rs:141-143`); no shell, no scratch |
| `tool_builder`, `coder`, `sat_solver`, `smt_solver`, `theorem_prover`, `symbolic_math`, `lean_prover` | `write_tool_file`, `execute_command`, `apply_patch`; scratch write (`orchestrator_registry.rs:140-145`) | config.toml, GOAL, TASKS, code/AGENTS, code/INDEX, code/lib/INDEX, CLAIMS, CONTEXT (`:157-167`) — **identical for all seven** | `exa_search`, `oeis_lookup`, delegation (`orchestrator_roles_test.rs:14,52`) |
| `judge` | *registry says* `read_document` only (`orchestrator_registry.rs:178`) | GOAL, INDEX only (`:175`) | **all memory** (`orchestrator_agents.rs:408-410`), scratch, search, shell, delegation, `research/APPROACHES.md` |
| `reflection` | none | GOAL, TASKS, INDEX (`:176`) | shell, search, scratch, delegation, **`CONTEXT.md`** — "material it must not mistake for verification" (`orchestrator_roles_test.rs:180-184`) |
| `pattern_finder` | `analyze_sequence`, `find_linear_recurrence`, `oeis_lookup`, `write_tool_file`, `execute_command`, `spawn_agent`/`await_agent` over `["tool_builder"]`; scratch write | GOAL, code/lib/INDEX, CONTEXT (`:177`) | `exa_search`, `apply_patch` |
| `inventor` | `exa_search`, `oeis_lookup` (gated), `spawn_agent`/`await_agent` over `["research"]` | GOAL, THREADS, APPROACHES, CLAIMS, CONTEXT (`:198-204`) **+ a dossier read from disk at spawn time** (`dossier.rs:151-153,232-278`) | shell, file write, scratch |
| `reducer` | **none** — the narrowest writing role | GOAL, BACKWARD, CLAIMS, THREADS, CONTEXT (`:213-219`) + own dossier | search, shell, delegation, scratch, and **`APPROACHES.md` deliberately** — "a role holding it drifts into proposing methods" (`:206-212`) |
| `librarian` | `exa_search`, `oeis_lookup` (gated) | shares the `research` row | shell, delegation, scratch |
| `scholar` | `recall_scratch` (read only) | GOAL, TASKS, CLAIMS, THREADS, CONTEXT (`:178-184`) | `exa_search` — "The scholar reads; it does not fetch" (`orchestrator_agents.rs:437-439`) |
| `context_curator` | `recall_scratch` | GOAL, TASKS, INDEX, CLAIMS, THREADS, APPROACHES, BACKWARD, CONTEXT (`:227-236`) | shell, web search, delegation (`orchestrator_roles_test.rs:203-215`) |
| `director` | `recall_scratch` | GOAL, TASKS, THREADS, APPROACHES, CONTEXT (`:246-252`) | shell, delegation, and **`research/CLAIMS.md`** — "a directive is asserted rather than established" (`:242-245`) |

**Three standing teams** run beside the loop (`orchestrator_teams.rs:21-101`): `director` (attentive: 4000 cycles / 24 h), `research`→`librarian` (acquiring: 40 cycles / 90 min), `patterns`→`pattern_finder` (custodial: 40 cycles / 90 min / 3 min floor); budgets at `teams.rs:102-141`. The `review` and `context` teams the docs describe no longer exist (`orchestrator_teams.rs:1-14`).

### What the structure buys, and what it does not

The tool boundaries are enforced in code, not asked for in prompts, and the reasoning is recorded at each site. What they do not contain:

- **No adversary.** None of the eighteen is tasked with attacking a result the run believes. Attacking your own method is method-policy rule 7 (`prompts/method_policy.md:38-44`) — an instruction to whichever role is already invested in the result.
- **No role that reads the mathematics for correctness.** `scholar` judges *sources*, `reflection` judges the *attempt report*, `judge` judges *conduct*. Nothing re-derives a step.
- **The seven code-writing roles are one role in seven prompts.** Identical tools, identical context; `docs/runtime.md:57-66` says so and defends it. The consequence is in the same file (`:418-424`): five solver and prover roles were "registered, tool-equipped, prompt-written, and provisioned in the image, and naming them in the planners' prompts did not get a single one spawned". Empirically `lean_prover` produced files in 3 of 8 conjecture workspaces and **0 of 16 Project Euler workspaces**.

---

## 2. The solution loop

Built in Rust, not JSON — there is no workflow JSON in the repo; `solution_loop` constructs a `tinyflows` graph (`workflow.rs:385-526`).

```
start → research (child wf) → seed_context → seed_goals (child wf) → seed_apply
      → solve (Loop head, accumulator = SolutionState)
           body → attempt
                    ├─ reflect ──────────┐
                    ├─ eval_patterns ────┤
                    ├─ eval_invention ───┼→ eval_merge → route (Switch)
                    ├─ eval_library ─────┤     ├ retry|solved|reported|blocked → pass
                    └─ goals → goal_apply┘     └ diversify → diversify_library → pass
           pass → solve
           done → judge → report
```

`EVAL_ARMS = ["reflect","eval_patterns","eval_invention"]` (`workflow.rs:54`) plus `LIBRARY_ARM` (`:77`) and `GOAL_APPLY` (`:83`). Every arm reads only `.nodes.attempt.item.json` (`:317`); the merge folds counters by delta (`solutions_evaluation.rs:103-115`).

**What triggers an attempt.** One loop iteration is one attempt. `attempt_step` increments `attempts` (`solutions_attempt.rs:550`), drains three mailboxes — pattern findings, operator directives, proof-skeleton gaps (`:566-581`) — builds `attempt_prompt` (`:676-722`), and on attempt 1 fire-and-forgets an oracle run (`:598-600`). The attempt is a single `goals` run told to pursue the goal until met.

### The judge, and the fact that it now runs once

**The judge is no longer a fan-out arm.** The only judge node is `FINAL_JUDGE`, wired from the loop's **`done`** port (`workflow.rs:478-480, 494`) — HEAD~1, `3c7e78d5 feat(orchestrator): background the librarian, judge once at the end`. It runs exactly once, after the loop has already terminated.

Rubric (`prompts/judge.md:13-30`): 5 = executed, checked against the statement's own examples, established something new; 4 = executed with a checking gap, **or** opened a new line of attack with cited precedent, a reason it fits, a first step, and an alternative closed with its reason; 3 = executed but thin, **or** a reformulation with no precedent check; 2 = wrote code or notes without running anything; 1 = prose only. Verdicts are `PROCEED | STEER | RESTART`, RESTART reserved for five named faults in conduct, closing with (`:70-71`): "If you cannot name which of those four faults occurred, and point at the words in the report that show it, the verdict is PROCEED." An unparsable reply parses as `Proceed` (`solutions_attempt.rs:825-834`).

**There is no acceptance threshold on the score.** `state.scores: Vec<u8>` (`solutions_attempt.rs:115`) is written at `solutions_judging.rs:48-50`, round-tripped through the accumulator, and **read by nothing** — no routing, no termination, no report. The judge's number influences the run in no way.

**RESTART is inert.** `MAX_RESTARTS = 2` (`solutions_attempt.rs:91`) is enforced in `judge_step` (`solutions_judging.rs:52-67`), but since the judge runs only after the loop exits, its `restarts += 1` and `steer` write into a state nothing subsequently attempts with. `state.steer` has no other writer, so the steer plumbing in `attempt_prompt` (`:694-697`) can never fire; the parity corpus pins `restarts` to 0 (`parity.rs:156`).

The judge is handed `evidence_briefing` (`solutions_judging.rs:145-193`), which counts on disk rather than trusting the report — captured outputs under `code/`, claims split by evidence class, approaches, threads, open/discharged gaps — because "the report above is written last and is the first thing lost when an attempt is cut off". Two real mechanical checks ride with it: `disagreement_warning` greps captured output for markers like `agree? false` / `mismatch` (`:204-271`, written after PE761's `agree? False` on every line went unread), and `oracle_unchecked` finds a `brute.py` never run (`:355, 375`).

### Reflection — the only thing that can end a run as solved

Four fields (`solutions_judging.rs:505-530`): `VERDICT` (SOLVED / UNVERIFIED / UNSOLVED), `PROGRESS` (YES/NO), `KIND` (MATHEMATICAL / COMPUTATIONAL / NONE), `LESSON`. Then `record_verdict` (`:824-934`):

```rust
state.solved = claimed && evidenced && progressed;    // :861
```

where `evidenced` = `has_executable_artifact` — any non-empty `.py`/`.sh` under `code/` or the workspace root (`:443-464`). The gate on "solved" is therefore: the model said SOLVED, the model said PROGRESS: YES, and *some* Python file exists. The last is a presence check, not a correctness check.

Other counters: `unverified` (+1 on UNVERIFIED when evidenced, else reset), `blocked` via `provider_blocked` (`solutions_attempt.rs:456-474`), `unproductive` (reset on progress, else +1), `computational` (+1 on COMPUTATIONAL, reset on MATHEMATICAL, unchanged on an unparsable reply).

### Routing and termination

`reflect_ladder` jq (`workflow.rs:161-170`), mirrored by `route` (`solutions_attempt.rs:393-426`), with `parity.rs` proving them equal exhaustively:

```
blocked      >= BLOCKED_THRESHOLD       (2) → blocked
solved OR attempts >= MAX_ATTEMPTS      (8) → solved
unverified   >= UNVERIFIED_THRESHOLD    (2) → reported
unproductive >= STUCK_THRESHOLD         (2) → diversify
computational>= COMPUTATIONAL_THRESHOLD (2) → diversify
else                                        → retry
```

All constants in `solutions_attempt.rs`: `MAX_ATTEMPTS = 8` (:6), `STUCK_THRESHOLD = 2` (:8), `BLOCKED_THRESHOLD = 2` (:15), `RESEARCH_RESCUE_ATTEMPTS = 5` (:26), `COMPUTATIONAL_THRESHOLD = 2` (:41), `UNVERIFIED_THRESHOLD = 2` (:62), `REDUCTION_INTERVAL = 3` (:80), `MAX_RESTARTS = 2` (:91).

Loop `until` (`workflow.rs:193-199`): `solved or attempts >= 8 or blocked >= 2 or unverified >= 2`. Four endings, worded distinctly in `outcome()` (`solutions_attempt.rs:315-356`): solved; "Answered but not independently verified"; "an infrastructure failure, not a result about the mathematics"; "Not solved within N attempt(s)". Note the port name lies — `attempts >= MAX_ATTEMPTS` routes to `"solved"` (`:404`) while the reported outcome is "Not solved".

---

## 3. Research

| Source | Tool | Where | Gated by `MATH_AGENT_RESEARCH=off`? |
|---|---|---|---|
| Exa web/literature search | `exa_search` | `hello_agent/mod.rs:264-380`, POSTs `https://api.exa.ai/search` (`:341`) | **yes** |
| OEIS | `oeis_lookup` | `oeis.rs:40` (`https://oeis.org/search`) | **yes** |
| Any `http`/`https` URL | `download_document` | `documents_tool.rs:250, 268-297` | **no** |

That is the complete list. No arXiv API, no MathSciNet, no zbMATH, no Semantic Scholar, no citation database. arXiv is reached only as an Exa result URL. Outbound HTTP from a workflow node is refused (`caps/network.rs:33-48`).

**The research gate has a hole.** `documents.tools()` (`documents_store.rs:41-63`) constructs every `DocumentToolKind::ALL` including `Download`, unconditionally, and registers it on all thirteen harnesses (`orchestrator_agents.rs:55, 79, 160, 243, 313, 358, 390, 405, 431, 446, 484, 504`). `--no-research` removes *discovery* but leaves *retrieval* of any URL the model can name, including `https://oeis.org/…` and any published answer page. Validation is scheme plus a 5 MiB cap (`documents_tool.rs:31-35`, `documents.rs:32`). `caps/network.rs:8-11` states the intent — a fetch tool that "bounds the response, records what was fetched, and can be withheld" — and the third clause is not true here.

Six ledgers under `research/` are **derived by code from fenced Markdown blocks, never hand-written**, re-rendered on every relevant write (`documents_tool.rs:319-360`):

| File | Module | Holds |
|---|---|---|
| `CLAIMS.md` | `claims.rs:35` | one row per `claim` block, plus Contradictions / Load-bearing but unverified / Taken from a catalogue |
| `THREADS.md` | `threads.rs:42` | directions of attack: `Open`/`Blocked`/`Dead`/`Settled` (`:52-64`) |
| `APPROACHES.md` | `approaches.rs:40` | candidate reformulations: `Proposed`/`Grounded`/`Adopted`/`Spent`/`Refuted` (`:49-66`) |
| `BACKWARD.md` | `backward.rs:51` | proof skeletons and per-lemma gaps |
| `FRONTIER.md` | `frontier.rs:44` | citation graph of held sources, ranked by in-degree then goal overlap (`:319-329`); doubles as the fetch ledger |
| `REQUESTS.md` | `requests.rs:40` | stated gaps: `need` / `why` / `falsifies` (`:58-66`) |

Downloads split three ways: `raw/` (original bytes), `research/sources/<name>.full.md`, and `research/summaries/` (a ≤4000-char *structural* digest built from labelled statements and headings, `digest.rs:33`).

### Claim vs assertion — mechanically distinguished

`Claim` (`claims.rs:201-227`) carries `id, statement, hypotheses, holds, status, bearing, contradicts, answers, anchor, source`. `Status` (`:129-159`): `Proved` | `Checked` | **`Asserted` (default)** | `Heuristic` | `Catalogued`. `Holds` (`:69-78`): `Yes` | `No` | `Unchecked` (default). The ledger then accuses on its own:

- `Holds::Yes` + `Status::Asserted` → **"Load-bearing but unverified"**: "Taken to hold here on a source's word alone. Verify by a second route" (`:603-628`).
- `Status::Catalogued` → **"Taken from a catalogue"**: "A catalogue is good evidence that a result is right and no evidence at all about why, so one of these may confirm a final answer and may never be the reason for one" (`:634-664`; the PE241 story is at `:139-158`).
- `contradicts` naming another claim id → **"Contradictions"** (`:578-601`).

A directive never enters this ledger: the `director` is denied `CLAIMS.md` (`orchestrator_prompts.rs:242-245`).

**This is the strongest single component of the harness** — the one place where evidence class is a typed field rather than a prompt request, and where the run's own bookkeeping generates an accusation nobody asked for. Its limit is that `Status::parse` (`claims.rs:162-187`) prefix-matches prose the model typed; nothing verifies that a claim marked `Proved` is proved.

---

## 4. Computation

`execute_command` (`exec.rs:327`) spawns `/bin/sh -lc <command>` in its own process group, cwd `/workspace` (`:220-230`). Workflow `code` nodes are refused by `RefusingCodeRunner` (`caps/execution.rs:47-67`), so there is exactly one execution path.

In the image (`Dockerfile`): Python 3 with sympy/numpy/scipy/gmpy2/networkx/mpmath/pandas/matplotlib (`:22-24`); SageMath (`:38`); Lean 4 with prebuilt Mathlib and a `lean` wrapper (`:55-84`); z3, cvc5, minisat, cryptominisat, glpk, cbc, nauty, eprover, PARI/GP, Singular, python3-z3/pulp/pycosat/igraph, and pip-installed ortools and python-sat (`:134-141`). **No C/C++/Rust/JS toolchain** — `layout.rs:66` files `.c`, `.rs`, `.js` as programs and nothing compiles them. pip installs go to `/workspace/.python-packages` (`:172`), and `PYTHONPATH=/workspace/.python-packages:/workspace/code` (`:182`) is what makes `code/lib/` importable — the reuse mechanism.

```rust
pub(super) const MAX_COMMAND_OUTPUT_BYTES: usize = 64 * 1024;   // exec.rs:30
const HEAD_BUDGET: usize = MAX_COMMAND_OUTPUT_BYTES / 4;        // exec.rs:42
const COMMAND_LOG: &str = "code/out/commands.log";              // exec.rs:64
const COMMAND_LOG_MAX_BYTES: usize = 512 * 1024;                // exec.rs:79
const DEFAULT_RUN_MINUTES: u64 = 30;                            // budget.rs:49
const DEFAULT_TOOL_MINUTES: u64 = 10;                           // budget.rs:51
```

Container: `mem_limit: 8g` (`compose.yaml:76`), `pids_limit: 256` (`:42`), `read_only: true` (`:35`), `tmpfs /tmp size=64m` (`:37`), `cap_drop: ALL` (`:38`), `no-new-privileges` (`:40`), one bind mount at `/workspace` (`:77-80`), network open. The `mem_limit` comment (`:43-75`) records 2g and 4g each OOM-killing live runs and closes: "an OOM from here is a finding about the approach, not a request for more memory". Middle-of-output is discarded as it streams (`exec.rs:111-123`) — a memory bound, not only a context bound. A timed-out command is SIGKILLed with its whole process group (`:240-243, 306-318`) and its partial output returned with "timed out after N seconds, killed. The output above is what it had printed by then" (`:384-387`).

**The complexity gate is real.** The schema requires `command`, `complexity`, `complexity_class` with `additionalProperties: false` (`exec.rs:334-365`), the class drawn from an enum, and `oracle_bound` required for `exponential`/`factorial`. `validate_complexity` (`:461-528`) refuses a non-enum class, an unbounded exponential, a prose/class mismatch checked against `["exponential","factorial","o(2^","o(2**","n!","2^n","2**n"]`, and — since PE185 — a declaration naming a search strategy (`SEARCH_METHODS`, `:555-561`) with no `oracle_bound`, redirecting to `sat_solver`. Three historical evasions are documented at `:436-460` and `:530-554`. This is one of the few places a method-policy rule is a control rather than a request.

### Long-lived computation: no

- Every command is synchronous and hard-capped at 10 minutes. Nothing daemonises, nothing backgrounds, nothing survives — the process group is SIGKILLed specifically to ensure that (`exec.rs:210-218`).
- **No checkpoint or resume for a computation.** No job store, no resume handle, no "continue this computation" tool.
- What crosses an attempt boundary is files plus `code/out/commands.log`, whose own comment names the problem: "`DEFAULT_RUN_MINUTES` stops the agent thirty minutes in and takes its context with it, so output that existed only in that context is destroyed" (`exec.rs:58-63`).
- The entire cross-attempt continuity mechanism is a prompt paragraph, `continuation_briefing` (`solutions_judging.rs:89-121`).

A four-hour computation cannot be run. It must be manually decomposed by the model into ≤10-minute chunks handing state through files, with no framework help — no incremental or resumable search, no checkpointed BFS, no parameter-sweep runner, no experiment ledger keyed by input size. Problem 763's own `MEMORY.md` recorded "exact BFS stops at N=14" — a sandbox limit written down as a mathematical result (`CLAUDE.md`).

The one non-ad-hoc numerical facility is `patterns.rs` — `analyze_sequence` and `find_linear_recurrence`, exact integer and rational arithmetic, "calculators, not heuristics", claiming only what holds for every term supplied (`:1-8`), with `MAX_TERMS = 512`, `MAX_RECURRENCE_ORDER = 12`, `MAX_MODULUS = 24` (`:18-22`), granted to `pattern_finder` alone.

---

## 5. Memory and reuse

**Within a run.** The bind-mounted workspace with layout enforced in code (`layout.rs`) and swept after every `execute_command` so shell heredocs cannot litter the root (`docs/workspace.md:97-114`); `.workspace-history`, a separate git dir committing after every successful write (`checkpoint.rs:34-49, 108`) — **history, not resume state**, since nothing reads it back programmatically; `CONTEXT.md`, one owner (`context_curator`), budgeted at `DEFAULT_CONTEXT_TOKENS = 10_000` (`shared_context.rs:47`) and clamped where it is spent, on the way into a prompt (`orchestrator_environment.rs:221-225`); and `note_scratch`/`recall_scratch`, in a store durable recall deliberately cannot reach (`vector_values.rs:106-116, 141-144`).

**Between runs on the same problem.** `./euler 763` continues from what is on disk. The Cognee session dataset is scoped to the *project*, not the run (`vector_store.rs:127-141`), after a bug stranded seven of eight datasets for one problem restarted eight times in a day.

**Across problems.** `math_agent_brain` is a dataset and node set (`vector.rs:21, 40`) that `durable_node_sets` includes for every project (`vector_values.rs:110-116`) and that `remember_memory` writes to (`vector_store.rs:213-223`).

Whether it spans problems is a deployment choice, and the default is that it does not. `scripts/run-agent:123-126`: with `COGNEE_NETWORK` unset it calls `scripts/memory-up "$workspace_subdir"`, bringing up one Cognee and one Neo4j *per problem* — `docker compose -p cognee-<slug>` with its own volumes and network (`memory-up:70, 87`). `math_agent_brain` then exists identically inside each problem's private stack, holding only that problem's memories.

That is deliberate, and the history is recorded at the call site (`run-agent:113-122`): a shared server was the earlier arrangement and failed on **availability, not privacy** — "four concurrent runs turned `recall_memory` into a ten-minute hang ending in `409 Conflict`, and a run cannot retry ten minutes it has already spent". Sharing is one variable away: "Set `COGNEE_NETWORK` yourself to opt back into a shared stack", and the runtime is unaware there is more than one server. The capability exists and is switched off by default on measured grounds; what is untested is whether a shared server survives today's concurrency.

**What is genuinely absent is a cross-problem library of technique.** `code/lib/` is per workspace (`layout.rs:54`); `workspace/tools/` holds one file, `hello.sh`; there is no shared claim ledger, no technique index, no lemma database, no transferable `research/` tree. The container mounts exactly one directory and `workspace_from_env` refuses anything else (`orchestrator_environment.rs:317-321`), so a shared library would need a second mount and a relaxation there. Even with `COGNEE_NETWORK` shared, what transfers is recalled prose, not a reusable lemma or a callable helper.

**Who can read and write.** `register_memory` grants all three durable tools as a bundle (`orchestrator_environment.rs:1-11`). Every role has them **except the judge** (`orchestrator_agents.rs:408-410`: "recall is the invitation to investigate, and the judge is the one role whose budget cannot absorb it"). Scratch write goes to `goals`, the seven code writers, and `pattern_finder`; read-only to `scholar`, `context_curator`, `director`.

`docs/roles.md:225-272` describes `search_workspace` (`recall.rs`), `recall_research`, and `remember_research`. **None exist** — grep over `src/` returns nothing and there is no `recall.rs`.

---

## 6. Verification

**Lean 4 and Mathlib are genuinely installed and smoke-tested at build time** (`Dockerfile:41-84`, including `import Mathlib.Combinatorics.SimpleGraph.Finite` as the `agent` user). The `lean_prover` prompt is strict (`prompts/lean_prover.md`): "A Lean proof that compiles with no `sorry` is not evidence; it is a proof"; "**Never report a proof you did not compile.** Run `lean` on the file and paste its real output"; "Run `#print axioms <name>` … anything beyond `propext`, `Classical.choice`, and `Quot.sound` means the proof rests on something the kernel did not check."

**Solvers that can establish universals**: z3/cvc5 via `smt_solver` ("proving a universal claim by refuting its negation", `orchestrator_registry.rs:114-115`), eprover via `theorem_prover`, CP-SAT/SAT via `sat_solver` where `UNSAT` is a theorem. **Oracle cross-check is enforced at the exec boundary** by the complexity gate (§4), and the loop fire-and-forgets an oracle run on attempt 1 (`solutions_attempt.rs:726-761`). **Two mechanical evidence checks** feed the judge — `disagreement_warning` and `oracle_unchecked` (§2). **`patterns.rs` is exact**: a conjectured recurrence is verified against every supplied term before it is reported (`:302-303, 356-359`).

### What does not exist

- **Nothing parses Lean output.** Grepping `sorry` and `#print axioms` across `src/orchestrator/` and `src/agent/` finds a doc comment (`orchestrator_agents.rs:109`) and tests asserting the *prompt string* contains those words (`orchestrator_roles_test.rs:97-100`). No code runs `lean`, reads its exit status, or promotes a claim on a kernel result. There is no `lean.rs`.
- **`Status::Proved` is self-reported** (`claims.rs:162-187`).
- **No verification gate on the answer.** `state.solved` needs a SOLVED string, a PROGRESS: YES string, and the existence of any non-empty `.py`/`.sh` (`solutions_judging.rs:861, 443-464`). "Verified by a second independent route" is a phrase in a prompt the same model self-certifies against.
- **No adversarial or red-team role**, **no replication** (nothing re-runs a computation independently and compares), and **no external answer check** — nothing in `runs.rs` or `src/bin/` validates a Project Euler answer.

**Where it would live.** `Status` (`claims.rs:129-159`) would need a machine-fed variant, and `evidence_briefing` (`solutions_judging.rs:145`) — which already walks the workspace counting artifacts and already reports failing outputs via `disagreement_warning` — is the natural place to run `lean` over `code/lean/*` and report kernel results the same way.

**The verification story is entirely social**: excellent prompts telling the model to distinguish proof from evidence, an excellent typed ledger for recording which is which, and no point at which anything mechanical checks the recording.

---

## 7. Goal management

**Creation.** The `reducer` writes `research/backward/<slug>.md`: one fenced `skeleton` block (`goal`, `implies`, `status`, `rests-on`, `killed-by`) and one `gap` block per lemma (`id`, `lemma`, `status`, `discharged-by`, `thread`, `next`) (`prompts/reducer.md:87-107`). `BACKWARD.md` is derived (`backward.rs:676`), and open gaps reach the next attempt through their own mailbox (`gap_briefing`, `solutions_attempt.rs:660-670`). The prompt is sharp about what makes it a decomposition: "`implies` is the field that makes this a proof skeleton rather than a wish list… Three attractive lemmas that do not recombine into the goal is exactly what a decomposition gets wrong" (`:109-113`), and "`next` has to be something a tool_builder could run today" (`:128-133`).

**Cadence.** `workflow_goals.rs` is only a cadence gate, not a goal tracker: hold if solved, hold if `since_reduction < REDUCTION_INTERVAL` (3), else check (`:88-94`). Three independent bounds on opening a reduction (`reduce_arm`, `solutions_judging.rs:687-734`): cadence, workspace fingerprint, and an in-flight gate against two reducers writing the same file.

**Abandonment is ledger-level only.** A skeleton moves to `Broken` (with `killed-by`) or `Spent`, a gap to `Refuted` (`backward.rs:60-146`). **No runtime control abandons a subgoal** — no timeout on a gap, no per-gap attempt counter, and nothing in the routing ladder reads gap state. `since_reduction`'s own doc says "nothing in [`route`] reads it" (`solutions_attempt.rs:136-139`).

**Weaker target, partial result, barrier result — absent from the harness.** `state.solved` is binary and demands a specific final answer verified by a second route (`solutions_judging.rs:509-511, 861`). The one softened ending, `UNVERIFIED` → `Route::Reported`, is explicitly not success — "Treat the answer as resting on the single route stated, not as confirmed" (`solutions_attempt.rs:326-332`) — and still requires a specific final answer, not a weaker theorem. **Discharging every gap of a skeleton does nothing**: `Stance::Discharged` is a ledger state, nothing converts it to `solved`, nothing routes on it, and the only place gaps surface is one line of the judge's `evidence_briefing` (`solutions_judging.rs:167`) — output no code reads (§2). Nothing weakens a goal statement; there is no "prove it for n ≤ N instead", no conditional-on-a-hypothesis bookkeeping, no barrier result as an outcome class. `COMPUTATIONAL` progress is treated as a reason to *diversify* (`solutions_attempt.rs:29-41`), not as a partial result to bank.

Where the notion does exist, a human wrote it. `workspace/conjectures/gilbreath/GOAL.md` opens: "A **proof, or a genuine partial result stated exactly**… the working assumption is that you will not prove it", then enumerates six qualifying partial results — a proved invariant under stated hypotheses, a theorem for a general class, a constant made explicit, a located error in Proth's 1878 claimed proof, a Lean formalisation with `#print axioms`. `scripts/solve-conjecture:113` carries the same framing. The loop's `solved` predicate cannot read any of it.

---

## 8. Failure handling

Three ledgers keep closed work deliberately visible:

- `APPROACHES.md` — `Refuted`/`Spent` retained with `killed-by` under "What closed, and why — Do not propose these again. A reason stated precisely is what makes that possible; one left blank makes this row worthless" (`approaches.rs:267`). A blank renders "_no reason recorded — say what closed it, or the next inventor will propose it again_" (`:272-275`). Empty `precedent` "means nobody has checked, which is a different statement from 'nothing was found'" (`:107-127`).
- `THREADS.md` — `Dead` retained: "A known dead end is a result, and a table that dropped these would let the next planner re-open a direction the run has already paid to close" (`threads.rs:52-64`).
- `BACKWARD.md` — `Broken`/`Spent` skeletons and `Refuted` gaps retained, plus a "Re-opened after being discharged" section (`backward.rs:553`).

Reflections are archived to `reflections/L0.<n>/<epoch>_<outcome>.md` with the outcome in the filename, indexed by the loop, and `refresh_index` refuses the folder (`folder_index::loop_owned`) so a hand refresh cannot replace verdicts with `_(undescribed)_`.

**The anti-retry mechanism is delivery, not blocking.** Closed approaches are packed into the inventor's dossier in priority order — goal, then what is ruled out, then what is established — against `DEFAULT_DOSSIER_TOKENS = 16_000` (`dossier.rs:23-25, 41`), with every cut announced. Nothing refuses a re-proposal; the inventor is shown the graveyard and asked not to dig. There is no "wastebasket" abstraction — the term appears nowhere in the codebase — and nothing prevents the same failed *computation* being run again, since the ledgers record approaches and lemmas, not commands.

---

## 9. Time horizon

| Bound | Value | Where |
|---|---|---|
| One tool call | 10 min | `budget.rs:51` |
| One agent run (incl. the attempt) | **30 min** | `budget.rs:49` |
| Model calls per run | 250 | `budget.rs:17` |
| Tool calls per run | 4000 | `budget.rs:28` |
| Turn output tokens | 48,000 | `budget.rs:88` |
| Judge / reflection run | 12 calls, 60 tools, 5 min | `budget.rs:109,115,122` |
| Housekeeping run | 25 calls, 300 tools, no clock | `budget.rs:140,147` |
| Attempts per loop | **8** | `solutions_attempt.rs:6` |
| Standing team (research, patterns) | 40 cycles / 90 min | `teams.rs:102,111` |
| Standing team (director) | 4000 cycles / 24 h | `teams.rs:135` |
| Concurrent child runs | 50 | `async_subagents.rs:142` |

One container run is bounded at roughly **8 × 30 min = 4 hours of attempts**, with 90 minutes of standing-team support beside it. At exhaustion, `LimitBehavior::StopWithPartial` applies on the model-call path (`budget.rs:335-354`), but the tool-call and wall-clock paths do **not** honour it and discard the run's context and report (`budget.rs:20-27, 44-48, 182-191`); what survives is files on disk.

**A multi-week program is not representable.** There is no plan object spanning runs, no run-to-run state beyond the workspace directory, no schedule, no milestone tracking. Continuity across launches is: the files are still there, and the next attempt is told "This run continues earlier work" (`solutions_judging.rs:89-121`). Everything the previous run understood but did not write down is gone, and with per-problem memory stacks (§5) the note store is scoped to the problem too.

**Empirically no investigation here has ever spanned more than two days.** Commit-date spans of every conjecture workspace: `erdos-gyarfas` 2026-08-12..13 (393 commits), `erdos-straus` 08-13 (129), `erdos-ternary-2n` 08-13 (72), `gilbreath` 08-13 (450), `magic-square-of-squares` 08-13 (320), `singmaster` 08-13 (289), `unitary-perfect` 08-13 (154). Seven of eight are a single day.

---

## 10. Where the documentation has drifted from the code

1. **The judge is not a fan-out arm.** `docs/solution-loop.md:11-21`, `CLAUDE.md`, and `solutions.rs:12-21` all show `judge` beside `reflect`. It runs once, off the `done` port (`workflow.rs:494`). RESTART and STEER therefore reach nothing.
2. **There is no `review` team.** `workflow.rs:66-68` and `docs/solution-loop.md:520-522` describe one posting mid-flight verdicts into the mailbox. `standing_teams()` returns three, none of them `review` (`orchestrator_teams.rs:21-101`). There is no mid-flight judging.
3. **`search_workspace`, `recall_research`, `remember_research` do not exist.** `docs/roles.md:225-272` and `CLAUDE.md` describe them; grep finds nothing and there is no `recall.rs`.
4. **Qdrant is gone.** `CLAUDE.md` says the orchestrator "starts the runtime and Qdrant through Docker Compose" and lists `QDRANT_URL` as required. There is no Qdrant service in `compose.yaml` and no reference in `src/`; the store is Cognee.
5. **Run ceiling.** `README.md:176` and `docs/runtime.md:317` say two hours; `.env.example:63` says 120. `DEFAULT_RUN_MINUTES = 30` (`budget.rs:49`).
6. **Turn output.** `.env.example:66` says 12000; the code default is 48,000 (`budget.rs:88`).
7. **Judge tools.** The registry advertises `read_document` alone (`orchestrator_registry.rs:178`) and a test pins it (`orchestrator_roles_test.rs:396`), but the harness registers all eleven document tools (`orchestrator_agents.rs:405-407`). Since `definitions.rs` derives the workflow grant from the registry, a workflow reader sees a one-tool judge the runtime does not implement.
8. **Budget drift.** `definitions.rs:52-53` declares `for_judging()` for `reflection` and `for_housekeeping()` for `librarian`; both harnesses use the base budget (`orchestrator_agents.rs:386, 421`).
9. **`organizer` is gone** from the registry but still named in `prompts/orchestrator.md:9-10`.
10. **`parse_reflection`** is registered as a workflow capability (`orchestrator_core.rs:395`) but no node in the current graph calls it.
11. **`--no-research` does not withhold `download_document`** (§3), though `caps/network.rs:8-11` states the intent that it should.
12. **`docs/roles.md:296-323` already flags its own drift** — `MEMORY.md` retired, four files unrouted, the routing table narrower than the prose. That section is honest and worth keeping as a model for the rest.

---

## 11. The map, compressed

**Can.** Run eighteen tool-boundaried specialists concurrently (50 at once), each with a narrow, code-enforced authority argued from live-run evidence at each site. Execute Python/Sage/Lean/z3/cvc5/eprover/PARI/Singular/nauty/CP-SAT in a hardened container — unprivileged, all caps dropped, read-only root, 8 GiB, 256 pids, one bind mount. Refuse an intractable method *before* it runs, via a declared complexity class with three documented evasions already closed. Maintain six code-derived ledgers — claims with typed evidence class and `holds-here`, threads, approaches, backward proof skeletons, a citation frontier, a request queue — that generate their own accusations. Work backward from the goal into lemma-level subgoals each carrying a first move somebody could make today. Absorb tool failures, provider outages, and truncated turns without losing the run. Take verbatim human direction mid-run through a file, exactly once, with a written receipt and no inbound port. Checkpoint every workspace write into a private git history.

**Cannot:**

- **Verify anything.** Lean and Mathlib are installed and nothing reads their output; `Status::Proved` is a string the model typed. "Solved" requires only that the model said SOLVED, said PROGRESS: YES, and that some `.py` exists.
- **Judge the mathematics.** The judge scores conduct, runs once at the end, and its score is read by no code. Its RESTART verdict routes nothing.
- **Attack its own results.** No adversarial, skeptic, or replication role; counterexample hunting is a prompt instruction to the role that produced the result.
- **Carry technique between problems.** Memory sharing is a deliberate default (§5) and one variable away, but even shared it moves recalled prose — there is no cross-problem library of lemmas, techniques, or callable helpers.
- **Run a long computation.** 10 minutes per command, SIGKILL, no checkpointing, no resume, no incremental search facility. What survives an attempt is files.
- **Count a partial result as success.** `solved` is binary and demands a specific final answer. Discharging every gap of a proof skeleton changes no state. No weaker target, no conditional result, no barrier result. Where the notion exists, a human wrote it into `GOAL.md` and the loop cannot read it.
- **Abandon a subgoal by policy.** No timeout, no attempt counter, no runtime control — only a stance the model may choose to write.
- **Hold a program longer than a day.** 8 attempts × 30 minutes, 90-minute standing teams, no plan spanning runs. Seven of eight conjecture workspaces here are a single day of commits.
- **Reach the literature properly.** Exa and OEIS only — no arXiv API, no MathSciNet, no zbMATH, no Semantic Scholar, no citation database.
