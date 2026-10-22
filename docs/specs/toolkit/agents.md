# Sub-agent tools

- **Status:** Draft
- **Parent:** [`README.md`](README.md)

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
4. The tool re-checks what the child produced and returns that, in the
   [envelope](README.md#one-envelope).

Each `execute` starts a new child session, so children carry nothing between
calls except what they wrote to the ledger and the Lean package.

### Scopes

A child that writes Lean files needs `lean_write`, a `Write` tool, and
`HostOnly` agents are held to read-only. Children therefore use
`ToolScopeSpec::Named` with exactly the tools listed for them below, and
`disallow_tools(["memory"])` so OpenHuman's own memory tool does not shadow
the ledger. The `agent_*` tool itself stays `ReadOnly`: the top-level agent
cannot write Lean files except through a child, whose tools are fixed.

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

## Tools

### `agent_digest_paper`

Given an arXiv id, reads the paper and records what a later turn will need.

- **Child tools:** `lit_get_paper`, `lit_extract_statements`, `lit_citations`,
  `ledger_record`, `ledger_link`, `recall_memory`.
- **Records:** each definition and main result as a `fact` citing its label;
  the techniques the proofs use; open problems and conjectures the paper
  states, as `lead`s; the paper's own dependency edges as `uses`.
- **Returns:** the ids recorded, grouped by kind, and a one-paragraph
  account of what the paper proves, at most 120 words.

Every fact it records is `cited`, never higher: reading is not checking.

### `agent_formalize`

Given a statement or a locator, writes it in Lean.

```json
{
  "source": { "arxiv_id": "2012.04125", "label": "thm:main" },
  "depth": "statement",
  "module": "Supermath/Sendov/Main.lean"
}
```

- `source` is `{arxiv_id, label}` or `{statement}` (informal text) or
  `{claim_id}`.
- `depth: "statement"` produces the theorem statement with a `sorry` proof,
  plus any definitions it needs. `depth: "proof"` also attempts the proof.
- **Child tools:** `lit_get_paper`, `lean_search`, `lean_decl`, `lean_goals`,
  `lean_try_tactics`, `lean_write`, `lean_check`, `ledger_record`,
  `recall_memory`.
- **Child method**, in its prompt: find the Mathlib names first
  (`lean_search`), reuse Mathlib definitions rather than redefining them,
  state the theorem, check that it elaborates, then work each `sorry` with
  `lean_goals` and `lean_try_tactics`. If the statement cannot be written
  from Mathlib primitives, say which definition is missing and stop: that
  is a `formalisation_depth` finding, not a failure
  ([`proofatlas/02-refutation-path.md`](../../research/math-superagent/proofatlas/02-refutation-path.md)).
- **Returns:** the module, its declarations, and for each the verdict from
  **the tool's own `lean_check`**, run after the child finished; the child's
  claims about its work are ignored. Each declaration is recorded in the
  ledger as a `result` with that verdict, alignment `not_reviewed`, and a
  `uses` edge to the fact it formalizes.

### `agent_review_alignment`

Given a Lean declaration and the informal statement it is meant to state,
judges whether they say the same thing.

- **Child tools:** `lean_decl`, `lean_search`, `lit_get_paper`,
  `recall_memory`. No write tools and no `lean_goals`.
- The child sees the statement and the definitions it uses, **never the
  proof**, and starts in a fresh context with no access to the formalizer's
  session. It checks quantifiers, hypotheses, degenerate cases, and whether
  every hypothesis of the informal statement appears
  ([`strategies/04-trust.md`](../../research/strategies/04-trust.md)).
- **Returns:** `accepted` or `disputed`, and for `disputed` the specific
  mismatch. The harness sets the claim's alignment from that.

A model's review is not evidence the way a kernel check is; the ledger
records it as the reviewer's judgment, with the reviewer's model and
session.

### `agent_refute`

Given a statement, looks for a counterexample before anyone tries to prove
it.

- **Child tools:** `calc_search_counterexample`, Jacobian over MCP,
  `lean_check`, `lit_oeis`, `ledger_record`, `ledger_bound`.
- The child states the search frame first (what is searched, over what
  range) and records it, so "nothing found" says how far it looked.
- **Returns:** a witness, re-checked by the tool with `lean_check` on a
  `decide`-able instance where possible; or "none found in frame" with the
  frame, as `unknown`. When it finds a counterexample it proposes the
  nearest repaired statement as a `lead`
  ([`strategies/03-failure.md`](../../research/strategies/03-failure.md)).

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
