# Sub-agent tools

- **Status:** Draft
- **Crate:** `supermath-subagents`
- **Parent:** [`README.md`](../README.md)

These are the toolkit's **agentic (OpenHuman)** tools: each call is a full
OpenHuman agent turn behind a tool interface
([`README.md`](../README.md#primitive-and-agentic-tools)). Each tool's file
has a **Turn** section giving its model, the primitives the turn may call,
its budget, and what the tool verifies, returns, and discards.

The `agent_*` tools live in `supermath-agent`, the OpenHuman runner, because
they drive models; they live in `supermath-subagents`, and `supermath-mcp`
does not serve them
([`crates.md`](../crates.md)). Their children use the primitive tools
through the runner's host-tool adapter.

Reading a paper or formalizing a theorem takes dozens of tool calls. Done in
the top-level agent, they would fill its context with Lean errors and
LaTeX. The `agent_*` tools hand each such task to a child agent with its own
prompt, its own tools, and its own model, and return only the outcome.

## How a child runs

OpenHuman's built-in sub-agents (`AgentSpec::subagents`, reached through
`delegate_<id>`) never see host tools, so they cannot call `lean_check` or
`ledger_record`. Each child is instead a separate top-level OpenHuman
`Agent`:

1. `Runner` builds the children when it registers a project's agent, each
   from an `AgentSpec` with its own id (`supermath-<slug>-formalizer`, …),
   a bare prompt, a `Named` tool scope, and its own host tools.
2. The `agent_*` tool captures a clone of the child `Agent` (it is `Clone`
   and `Send`) in the tool factory.
3. `execute` runs one turn on the child with `child.turn(prompt)`, under a
   budget, and reads the `TurnOutcome`.
4. The tool re-checks what the child produced by calling the registry's
   `lean_check` directly, and returns that, in the
   [envelope](../README.md#one-envelope).

Each `execute` starts a new child session, so children carry nothing between
calls except what they wrote to the ledger and the Lean package.

### Scopes

A child that writes Lean files needs `file_write` and `file_edit`, which
are `Write` tools, and `HostOnly` agents are held to read-only. Children
therefore use `ToolScopeSpec::Named` with exactly the tools listed in their
tool's **Turn** section, and `disallow_tools(["memory"])` so OpenHuman's own
memory tool does not shadow the ledger. A child's list holds primitives
only: no child can call an agentic tool
([`README.md`](../README.md#primitive-and-agentic-tools)). The `agent_*` tool
itself is `Read`; its writes happen inside the child, to files in the
project's package, under the [path rules](../lean/README.md#files-not-source).

### Models

Each child runs on its own ladder rung, set by configuration:

| Variable | Default | Used by |
| --- | --- | --- |
| `SUPERMATH_READER_MODEL` | `flash` | `agent_digest_paper` |
| `SUPERMATH_FORMALIZER_MODEL` | `formalize` | `agent_formalize`, `agent_refute` |
| `SUPERMATH_REVIEWER_MODEL` | `reasoning` | `agent_review_alignment` |

`formalize` is a new ladder rung. The first candidate is a Mistral model
through the ladder's Mistral seller; open Lean provers (Kimina-Prover,
DeepSeek-Prover, Goedel-Prover) are candidates to benchmark against it on
the [acceptance set](#acceptance-criteria). The rung is a ladder name, so
the choice changes in the ladder's config, not here. A child sets its model
with `AgentSpec::model`, keeping the runtime's ladder route.

The reviewer runs on a different rung from the formalizer by default, so
the model that wrote a statement is not the one that judges it.

### Budgets

Every call takes a budget and is admitted against it before the child runs:

| Field | Default | Maximum |
| --- | --- | --- |
| `max_iterations` | 24 | 64 |
| `max_tokens` | 200,000 | 1,000,000 |
| `wall_secs` | 600 | 1,800 |

A child that runs out returns `outcome: "unknown"` with what it finished:
the declarations written so far and their verdicts. Running out of budget
never becomes a failed proof.

### Visibility

The tool forwards the child's `ToolStarted` and `ToolEnded` runtime events
to the parent's progress stream, so an operator sees `agent_formalize →
lean_check ✓` without the child's context reaching the parent model. The
child's transcript stays in its OpenHuman session, and the tool's result
carries its `session_id`.

## Crate

| | |
| --- | --- |
| Name | `supermath-subagents` |
| Depends on | `supermath-tool`, `openhuman-embed` |
| Public API | `Subagents::new(runtime, registry, project)`, `ChildSpec`, `Budget`, and the four `agent_*` tools |
| Configuration | `SUPERMATH_READER_MODEL`, `SUPERMATH_FORMALIZER_MODEL`, `SUPERMATH_REVIEWER_MODEL` |
| Tests | a scripted OpenHuman provider (`Provider::custom`) playing fixed tool-call sequences, against fake primitive tools |

The crate does not depend on any domain crate. It receives a `ToolRegistry`
from the composition, holding the primitives by name, and builds each
child's tool list from it; it calls `lean_check`, `ledger_record`, and the
other primitives it verifies with through the same registry. That keeps it
testable with fakes and keeps the "primitives only" rule a property of what
the registry hands it.

## Tools

Each tool is specified in its own file in this crate's `tools/` folder, in
the shape [`../tools.md`](../tools.md) sets out.

| Tool | Given → returns |
| --- | --- |
| [`agent_digest_paper`](tools/agent_digest_paper.md) | an arXiv id → facts recorded from it |
| [`agent_formalize`](tools/agent_formalize.md) | a statement or paper locator → Lean files and their verdicts |
| [`agent_review_alignment`](tools/agent_review_alignment.md) | a declaration and its informal statement → alignment |
| [`agent_refute`](tools/agent_refute.md) | a statement → a counterexample, or a repaired statement |

## Invariants

- No child can write outside its tools, and no child's tools include
  another `agent_*` tool: children do not spawn children.
- A child's reply is never returned as a verdict. Verdicts come from tools
  the parent ran.
- A child that exceeds its budget returns `unknown`, never `failed`.

## Acceptance criteria

- Unit tests run each `agent_*` tool against a scripted provider that plays
  a fixed sequence of tool calls, and assert what the tool recorded and
  returned, including when the scripted child claims success the Lean stub
  contradicts.
- A test asserts the child's tool scope is exactly its listed tools.
- A benchmark set of 30 statements (Mathlib theorems with known
  formalizations, plus 10 from arXiv) is checked into `bench/formalize/`,
  with a `live_formalize_bench` test reporting the `checked` rate per rung.

## Open questions

1. Should `agent_formalize` call `agent_review_alignment` itself, or leave
   that to the solving loop?
2. Is one child per call too costly for small statements, and should the
   tool keep a warm session per project instead?
