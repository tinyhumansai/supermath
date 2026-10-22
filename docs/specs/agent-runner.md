# Agent runner

- **Status:** Draft (scaffold implemented)
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
- Every chat completion and embedding goes through the LLM ladder. Turns run
  on the `flash` ladder at `medium` reasoning effort by default.
- Jev, the decision model OpenRouter serves, ranks `tool_search` over the
  agent's deferred tools.
- A small toolkit is handed to the agent as host tools.
- CortexDB holds durable memory, scoped per project.
- `docker compose up` runs the ladder, CortexDB, and `supermath-agent check`.

## Non-goals

- How the agent attacks a problem: the loop, prompts, roles, and verdicts.
  They are designed later, with the research notes as input.
- The conjecture toolkit, including Lean checking. It is specified in
  [`toolkit/`](toolkit/README.md).
- Multi-tenant memory. One deployment owns one scope root.

## Behavior

```sh
supermath-agent check [project]     # default project: scratch
```

1. Reads [configuration](#configuration); a missing required variable is
   `Error::MissingConfig` naming it.
2. Boots the OpenHuman runtime with the ladder as its provider and the Jev
   ranker installed (`tool_search.ranker = jev`, `jev_route = openrouter`).
   The ranker retrieves with the ladder's `vectors` model.
3. Registers the agent `supermath-<project>` with `ToolScopeSpec::HostOnly`
   and the toolkit. No turn runs.
4. Checks `GET /v1/admin/ready` on CortexDB. Not ready, or degraded (such as
   mock embeddings), is `Error::MemoryNotReady`.

Exit status `0` means the stack is ready.

### Toolkit

| Tool | Exposure | Does |
| --- | --- | --- |
| `recall_memory` | direct | recalls this project's CortexDB memory |
| `factor_integer` | deferred | exact prime factorization, 2 to 10^12 |
| `gcd_lcm` | deferred | exact gcd and lcm of two `u64` |

Deferred tools stay out of the model's initial catalogue and are revealed by
`tool_search`, which Jev ranks. Every tool is read-only and takes typed JSON
arguments; none evaluates text the model wrote.

### Memory scopes

```text
{CORTEX_SCOPE_ROOT}/project:{slug}/app:brain
```

`CORTEX_SCOPE_ROOT` defaults to `ws:supermath`. The slug comes from the host,
never from model output or a tool argument. CortexDB v0.10 refuses scope types
outside its `allowed_scope_types`, which is why the leaf is `app:` and not
math-superagent's `store:`. Writes wait for the `indexed` barrier, and a write
the server stored without indexing is an error.

### Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `LADDER_URL` | `http://localhost:6969/v1` | the ladder's API root |
| `LADDER_API_KEY` | required | bearer token the ladder requires |
| `SUPERMATH_MODEL` | `flash` | the ladder turns run on |
| `SUPERMATH_REASONING_EFFORT` | `medium` | `none` … `xhigh`, sent on every turn |
| `SUPERMATH_EMBEDDING_MODEL` | `vectors` | the embeddings ladder |
| `SUPERMATH_EMBEDDING_DIMS` | `1024` | its width; CortexDB pins it on first ingest |
| `OPENROUTER_API_KEY` | required | Jev's credential, and nothing else |
| `SUPERMATH_WORKSPACE` | `./.supermath` | OpenHuman workspace and logs |
| `SUPERMATH_BACKEND_URL` | OpenHuman's | OpenHuman backend URL; no TinyHumans account is needed |
| `CORTEX_API_URL` | `http://localhost:3141` | CortexDB base URL |
| `CORTEX_API_KEY` | required | CortexDB operator key |
| `CORTEX_SCOPE_ROOT` | `ws:supermath` | scope subtree this deployment owns |

## Invariants

- No memory scope is derived from model output.
- A memory write is reported as stored only when CortexDB reports it indexed.
- The Jev route is named (`openrouter`), never left to `auto`.
- Booting the runner never writes to the operator's `~/.openhuman`.

## Acceptance criteria

- Unit tests cover configuration, project slugs, the memory client (against a
  loopback stub), readiness, and every toolkit tool's answers and refusals.
- `tests/check.rs` runs the binary against loopback stubs of the ladder,
  CortexDB, and the backend, and asserts no completion was requested.
- `docker compose config --quiet` passes.

## Open questions

1. How the agent works on a problem: the loop, the prompt, and what it
   reports.
2. Should memory move behind OpenHuman's own memory-engine seam, so every
   OpenHuman agent sees CortexDB?
3. ~~Which tools come next?~~ Answered by [`toolkit/`](toolkit/README.md):
   a pinned Lean service, literature tools, a claim ledger, sub-agents, and
   Jacobian over MCP.
