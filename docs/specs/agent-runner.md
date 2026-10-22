# Agent runner

- **Status:** Implemented (toolkit and sub-agents wired; the solving loop is still open)
- **Owner:** Maintainers
- **Decision:** [`../adr/0002-build-on-openhuman.md`](../adr/0002-build-on-openhuman.md)
- **Research:** [`../research/math-superagent/`](../research/math-superagent/README.md),
  [`../research/rea-and-jacobian/`](../research/rea-and-jacobian/README.md)

## Problem

supermath needs an agent that can work on mathematics with tools and durable
memory. Before deciding how it works on a problem, the pieces it runs on have
to exist and be wired together: an agent runtime, model routing, tool
selection, a toolkit, and memory.

## Goals

- `crates/supermath-agent` embeds [OpenHuman](https://github.com/tinyhumansai/openhuman)
  through `openhuman-embed` and registers one agent per project.
- Every chat completion and embedding goes straight to Surplus Intelligence
  ([ADR 0008](../adr/0008-call-surplus-intelligence-directly.md)). Turns run
  on `deepseek-v4.1-flash` at `medium` reasoning effort by default.
- Jev, the decision model OpenRouter serves, ranks `tool_search` over the
  agent's deferred tools.
- The composed toolkit (`supermath-mcp`) and the `agent_*` sub-agents
  (`supermath-subagents`) are handed to the agent as host tools.
- Jacobian and lean-lsp-mcp join as stdio MCP servers when configured.
- CortexDB holds durable memory, scoped per project.
- `docker compose up` runs CortexDB and `supermath-agent check`.

## Non-goals

- How the agent attacks a problem: the loop, prompts, roles, and verdicts.
  They are designed later, with the research notes as input.
- The tools themselves. They are specified in [`toolkit/`](toolkit/README.md)
  and implemented in the domain crates; this crate only adapts them.
- Multi-tenant memory. One deployment owns one scope root.

## Behavior

```sh
supermath-agent check [project]     # default project: scratch
```

1. Reads [configuration](#configuration); a missing required variable is
   `Error::MissingConfig` naming it.
2. Boots the OpenHuman runtime with Surplus as its provider and the Jev
   ranker installed (`tool_search.ranker = jev`, `jev_route = openrouter`).
   The ranker retrieves with Surplus's embedding model.
3. Composes the toolkit for the project (`Toolkit::compose`), builds the
   `agent_*` children on the same runtime, and registers the agent
   `supermath-<project>` with a `ToolScopeSpec::Named` list: every toolkit
   tool, the `agent_*` tools, and `tool_search`. OpenHuman's own `memory`
   tool is disallowed so it cannot shadow `recall_memory`. When
   `SUPERMATH_JACOBIAN_CMD` or `SUPERMATH_LEAN_LSP_CMD` is set, the server is
   attached with `AgentSpec::mcp` and `McpServer::stdio`. No turn runs.
4. Lists the tools and the `agent_*` tools, then checks `GET /v1/admin/ready`
   on CortexDB. Not ready, or degraded (such as mock embeddings), is
   `Error::Memory`.

Exit status `0` means the stack is ready.

```sh
supermath-agent solve <project> <prompt-file> [turns]   # default turns: 1
```

`solve` boots and registers the agent the same way (steps 1–3), then runs
it on the problem in `prompt-file`:

1. The prompt is the first turn of one session; each later turn, up to
   `turns`, sends `CONTINUE_PROMPT` in the same session. A reply with the
   line `STATUS: DONE` ends the run early.
2. Each turn's tool calls are reported on stderr as they start and finish
   (`AgentProgress`), and a watcher tails the toolkit's call log, so calls
   the `agent_*` children make (the duel's sides among them) appear too.
3. Each reply goes to stdout. A turn OpenHuman fails is `Error::Turn`; an
   unreadable prompt file is `Error::Prompt`.

### Toolkit

The tools are the domain crates' (`docs/specs/toolkit/README.md` has the
catalogue); the runner links them in process through `supermath-mcp`, and
`host/` adapts each ([details](toolkit/crates.md#supermath-agent)).

| Group | Tools | Crate | Exposure |
| --- | --- | --- | --- |
| memory | `recall_memory` | `supermath-ledger` | direct |
| ledger | `ledger_record`, `ledger_claims`, `ledger_link`, `ledger_bound` | `supermath-ledger` | per tool |
| calc | `calc_factor`, `calc_gcd_lcm` | `supermath-calc` | deferred |
| files | `file_read`, `file_write`, `file_edit`, `file_list` | `supermath-files` | per tool |
| lean | `lean_check`, `lean_goals`, `lean_decl`, `lean_search`, ... | `supermath-lean` | per tool |
| python | `py_run`, `py_search_counterexample`, `py_solvers` | `supermath-python` | per tool |
| literature | `lit_search_arxiv`, `lit_get_paper`, `lit_citations`, `lit_oeis`, ... | `supermath-literature` | per tool |
| web | `web_search`, `web_research`, `web_read` | `supermath-web` | per tool |
| sub-agents | `agent_digest_paper`, `agent_formalize`, `agent_review_alignment`, `agent_refute`, `agent_settle` | `supermath-subagents` | per tool |
| MCP | `mcp_jacobian_*`, `mcp_lean_lsp_*` | external servers | deferred (OpenHuman's rule) |

Deferred tools stay out of the model's initial catalogue and are revealed by
`tool_search`, which Jev ranks. A tool's own `Exposure` and `Permission`
carry through the adapter. A call to any tool is recorded in the toolkit's
call log under a `call_id`, which the adapted result carries so a ledger
`tool` source can quote it. The `agent_*` tools get `agent-N` ids and an
agentic budget (64 iterations, 1,000,000 tokens, one hour).

### Memory scopes

```text
{CORTEX_SCOPE_ROOT}/project:{slug}/app:brain
```

`CORTEX_SCOPE_ROOT` defaults to `ws:supermath`. The slug comes from the host,
never from model output or a tool argument. The memory client is the
ledger's. CortexDB v0.10 refuses scope types outside its
`allowed_scope_types`, which is why the leaf is `app:` and not
math-superagent's `store:`. Writes wait for the `indexed` barrier, and a write
the server stored without indexing is an error.

### Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `SURPLUS_URL` | `https://api.surplusintelligence.ai/v1` | Surplus Intelligence's API root |
| `SURPLUS_API_KEY` | required | bearer token Surplus requires |
| `SUPERMATH_MODEL` | `deepseek-v4.1-flash` | the Surplus model turns run on |
| `SUPERMATH_REASONING_EFFORT` | `medium` | `none` … `xhigh`, sent on every turn |
| `SUPERMATH_EMBEDDING_MODEL` | `venice-embed-1` | the Surplus embedding model |
| `SUPERMATH_EMBEDDING_DIMS` | `1024` | its width; CortexDB pins it on first ingest |
| `OPENROUTER_API_KEY` | required | Jev's credential, and nothing else |
| `SUPERMATH_WORKSPACE` | `./.supermath` | OpenHuman workspace and logs |
| `SUPERMATH_BACKEND_URL` | OpenHuman's | OpenHuman backend URL; no TinyHumans account is needed |
| `CORTEX_API_URL` | `http://localhost:3141` | CortexDB base URL |
| `CORTEX_API_KEY` | required | CortexDB operator key |
| `CORTEX_SCOPE_ROOT` | `ws:supermath` | scope subtree this deployment owns |
| `SUPERMATH_READER_MODEL`, `SUPERMATH_FORMALIZER_MODEL`, `SUPERMATH_REVIEWER_MODEL`, `SUPERMATH_DUEL_MODEL` | `deepseek-v4.1-flash`, `deepseek-v4-pro`, `glm-5.3`, `deepseek-v4-pro` | the Surplus model of each `agent_*` child |
| `SUPERMATH_DUEL_DRIFT_CALLS`, `SUPERMATH_DUEL_MAX_ROUNDS`, `SUPERMATH_DUEL_DIGEST_EVERY` | `12`, `4`, `8` | how `agent_settle` keeps its two sides on task ([spec](toolkit/subagents/tools/agent_settle.md#focus)) |
| `SUPERMATH_JACOBIAN_CMD` | unset | command line of the Jacobian MCP server (stdio) |
| `SUPERMATH_LEAN_LSP_CMD` | unset | command line of lean-lsp-mcp (stdio) |
| `SUPERMATH_PROJECTS`, `LEAN_URL`, `PYTHON_URL`, ... | see [`toolkit/mcp/`](toolkit/mcp/README.md) | the toolkit's own variables |

## Invariants

- No memory scope is derived from model output.
- A memory write is reported as stored only when CortexDB reports it indexed.
- The Jev route is named (`openrouter`), never left to `auto`.
- Booting the runner never writes to the operator's `~/.openhuman`.

## Acceptance criteria

- Unit tests cover configuration, the host adapter (every adapted tool
  matches the composed toolkit's name, schema, exposure, permission and
  kind), the agent's pinned tool list, and the MCP command parsing.
- `tests/check.rs` runs the binary against loopback stubs of Surplus,
  CortexDB, and the backend, asserts the tool listing, and asserts no
  completion was requested. It also runs `solve` against a scripted
  completion: the run stops at the first reply that declares itself done,
  and otherwise continues the session to the turn limit.
- `tests/live.rs` holds `live_subagents_formalize_bench`, skipped unless
  `SUPERMATH_LIVE_SUBAGENTS=1`.
- `docker compose config --quiet` passes.

## Open questions

1. How the agent works on a problem: the loop, the prompt, and what it
   reports. Settling one aligned conjecture is answered by
   [`agent_settle`](toolkit/subagents/tools/agent_settle.md), a prover and
   a refuter racing in parallel
   ([ADR 0009](../adr/0009-settle-conjectures-by-a-prover-refuter-duel.md));
   the loop around it is still open.
2. Should memory move behind OpenHuman's own memory-engine seam, so every
   OpenHuman agent sees CortexDB?
3. ~~Which tools come next?~~ Answered by [`toolkit/`](toolkit/README.md):
   a pinned Lean service, literature tools, a claim ledger, sub-agents, and
   Jacobian over MCP.
