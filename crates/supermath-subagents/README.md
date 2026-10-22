# supermath-subagents

The five `agent_*` tools: each hands one long task to a **child OpenHuman
agent** with its own prompt, primitives, and model, and returns only the
outcome. `agent_settle` runs two children at once, a prover and a refuter. Spec: [`docs/specs/toolkit/subagents/`](../../docs/specs/toolkit/subagents/README.md).

| Tool | Child model | Returns |
| --- | --- | --- |
| `agent_digest_paper` | `SUPERMATH_READER_MODEL` (`deepseek-v4.1-flash`) | ledger ids recorded from an arXiv paper, read back from the ledger |
| `agent_formalize` | `SUPERMATH_FORMALIZER_MODEL` (`deepseek-v4-pro`) | Lean files written, each with a `lean_check` verdict the tool ran |
| `agent_review_alignment` | `SUPERMATH_REVIEWER_MODEL` (`glm-5.3`) | `accepted` or `disputed`; the reviewer never sees a proof |
| `agent_refute` | `SUPERMATH_FORMALIZER_MODEL` | a counterexample only after Lean confirms it, else the frames searched |
| `agent_settle` | `SUPERMATH_DUEL_MODEL` (`deepseek-v4-pro`), both sides | `proved` or `refuted` once the tool checks a side's theorem against the pinned statement, else `unknown` with the bounds searched |

## The rule

A child's reply is never a verdict. After the turn each tool re-runs the
primitive that decides its answer through the same `ToolRegistry`, and reports
that. What a child wrote comes from the registry's own answers to its
`file_write` and `file_edit` calls. Children hold **primitives only**, so they
cannot spawn children, and a child that runs out of iterations, tokens, or wall
time yields `outcome: "unknown"`, never `failed`.

## Design

```text
Subagents::new(runtime, registry, log, project, config)
  per child: registry.subset(primitives) ─ Belt ─ HostTool ─► OpenHuman Agent
                                            │
  agent_* tool .call ─ child.turn(prompt) ──┘   (Turn::tools, stop hook, timeout)
        │
        └─ after the turn: lean_check / file_read / ledger_claims / ledger_record
```

- `child/` describes the four children and the two duel sides (`ChildSpec`:
  primitives, verifying primitives, prompt, default budget), defines the
  `ChildRunner` port and `TurnControl` (a host's steer and cancel handle on a
  running turn, forwarded to OpenHuman's `TurnSteering` and
  `TurnCancellation`), and has
  `OpenHumanChild`, the real runner: a top-level `Agent` with a `Named` scope,
  `Access::full()` (the scope is the bound), and `disallow_tools(["memory"])`.
  A stop hook enforces iterations and tokens; `Turn::timeout` enforces wall time.
- `belt/` is the registry scoped to one turn. Every child call is dispatched
  through it, logged in the shared `CallLog` under a unique call id, and
  journaled (`CallNote`), which is how tools learn what was written and
  recorded. `HostTool` adapts a registry tool to OpenHuman's `Tool` trait.
- `duel/` is the race behind `agent_settle`: the pinned target and its
  host-written `Statement.lean` and verdict files (`target.rs`), the per-side
  write areas (`guard.rs`), the board that reads every call both sides make to
  deliver digests, nudge or restart a drifting turn, and spot a `Main.lean`
  worth verifying (`board.rs`), `duel_post` and `duel_board` (`post.rs`), the
  host-written turn prompts (`brief.rs`), and the two drivers and the
  controller (`mod.rs`). The spec is
  [`agent_settle.md`](../../docs/specs/toolkit/subagents/tools/agent_settle.md).
- `tools/` holds the five tools and their shared steps (`shared.rs`: admission,
  running a turn with accounting; `verify.rs`: reading `lean_check`,
  `file_read`, and the ledger back).
- `limits/` and `config/` hold the budget (`Limits`, `BudgetArg`), the model
  names (`Models`), and the duel's pacing (`DuelSettings`).
- `bench/` runs the checked-in formalization benchmark.

## Wiring

```rust,ignore
let subagents = Subagents::new(
    &runtime,                 // openhuman_embed::Runtime
    &toolkit.registry(),      // the primitives, by name
    call_log,                 // Arc<dyn CallLog>: the one the ledger reads
    &project,
    &SubagentsConfig {
        models: Models::from_env()?,
        duel: DuelSettings::from_env()?,
        ..SubagentsConfig::default()
    },
)?;
subagents.register_into(&mut registry)?;      // or iterate subagents.tools()
for missing in subagents.unavailable() { /* log: tool needs these primitives */ }
```

`provider: None` keeps the runtime's own provider (Surplus Intelligence); each child sets
only its model. The agent ids are
`supermath-<slug>-<reader|formalizer|reviewer|refuter|duel-prover|duel-refuter>`,
so one runtime holds one `Subagents` per project. `Subagents::with_runners`
builds the same tools over `ChildRunner`s the host supplies.

## Testing

- Unit tests (`src/**/test.rs`) run each tool against a scripted `ChildRunner`
  and fake primitives over one in-memory world (`src/fake/`), including, for
  every tool, a child that claims success while the fake Lean says otherwise.
  Every tool also passes `supermath_tool::contract::check`.
- `tests/openhuman.rs` runs the children as real OpenHuman agents on a
  scripted `ChatModel` (`Provider::custom`), checking each child is offered
  exactly its primitives, the per-child model, the iteration cutoff, and that a
  child cannot reach a tool outside its scope. Its duel scenarios check that
  monitor nudges reach a running turn through OpenHuman steering, that a third
  drift cancels the turn, and that a prover's win stops the refuter. The
  scripted duel sides answer after 150 ms, because OpenHuman forwards steers
  every 50 ms and an instant model would end its turn before a steer lands. OpenHuman allows one runtime per
  process, so it is one test.
- `live_subagents_formalize_bench` lives in `supermath-agent`'s `tests/live.rs`
  (it needs the composed toolkit, which this crate never depends on), skipped
  unless `SUPERMATH_LIVE_SUBAGENTS=1`; see its header for the variables. The benchmark set is in
  [`bench/formalize/`](bench/formalize/README.md).

## Constraints

- A duel side writes only under its own prefixes; reads are open. Its
  `Main.lean` wins only after the tool's own `lean_check` with the side's
  theorem as target is `checked` **and** a host-written file restating the
  pinned `Conjecture` from it is `checked`.

- A `CallContext` for another project than the one a `Subagents` serves is
  `FORBIDDEN`: the children are built per project.
- The caller's `Budget` must allow the child's wall time (`wall_time_ms`
  defaults to 30 s in `Budget::default()`; set it for agentic calls).
- `Ledger::set_alignment` exists but no tool exposes it (and this crate may not
  depend on the ledger crate), so a review is recorded as a `fact` that `uses`
  the reviewed claim; the claim's own `alignment` stays `not_reviewed`.
  `agent_settle` takes that fact's id as `review_id` and checks it in place of
  the claim's `alignment`.
- The shapes this crate sends to and reads from the primitives are pinned by
  the fakes in `src/fake/world.rs`, which refuse the same unknown or missing
  fields the real schemas do. Read from `lean_check`: `data.verdict`,
  `data.provenance.file_sha256`, `data.targets` (fully qualified, used to
  qualify `file_read` outline names), `data.error_count`, `data.sorries`.
  Read from `lean_search_counterexample`: `data.found`, `data.result`,
  `witness`, `witness_file`, `theorem`, `frame`. Read from `py_search_counterexample`:
  `data.result`, `witness`, `frame_description`.
