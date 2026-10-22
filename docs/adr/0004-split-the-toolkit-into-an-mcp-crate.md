# 4. Split the toolkit into an MCP crate, separate from the runner

- **Status:** Proposed; amended by 6 (tools move to domain crates)
- **Date:** 2026-10-22

## Context

The toolkit ([`specs/toolkit/`](../specs/toolkit/README.md)) began as
OpenHuman host tools inside `supermath-agent`. Two needs pull it out:

- Other agents the maintainers use daily (Claude Code, Codex) speak MCP. A
  toolkit they can call can be tried by hand, compared against, and reused
  outside OpenHuman.
- The toolkit is deterministic: Lean, arXiv, OEIS, CortexDB, arithmetic. The
  runner is not: it drives models. Mixed in one crate, every toolkit test
  compiles all of OpenHuman, and the trust rules (who may set `formalised`)
  depend on which caller is asking.

OpenHuman has an MCP server, but it serves a fixed catalogue of its own
tools from its CLI, is not exposed through `openhuman-embed`, and cannot
serve a host's tools. Its MCP *client* is exposed, as `AgentSpec::mcp`
behind the embed crate's `mcp` feature.

Three shapes were considered:

- **One crate, two faces.** Simple, but the toolkit stays coupled to
  OpenHuman's build and its trust rules live beside the agent loop.
- **Two crates, runner connects over MCP.** A clean process boundary, but
  OpenHuman marks every MCP tool deferred, losing the direct tools, and adds
  a child process per agent.
- **Two crates, runner links the toolkit as a library.** The boundary is
  the crate and its `Tool` trait; the MCP server is the same trait served
  by `rmcp`.

## Decision

- Create `crates/supermath-mcp`: the `Tool` trait, the envelope, every
  primitive tool and its clients (Lean, literature, CortexDB, ledger,
  arithmetic), and a `supermath-mcp serve` stdio MCP server built on
  `rmcp`. It does not depend on OpenHuman and calls no model.
- Keep `crates/supermath-agent` as the OpenHuman runner. It depends on
  `supermath-mcp` as a library, adapts its tools into OpenHuman host tools,
  and owns everything that needs a model: the `agent_*` tools and their
  child agents.
- Move the CortexDB client (`memory/`) and the arithmetic tools from
  `supermath-agent` into `supermath-mcp`.
- Enforce the trust rules inside `supermath-mcp`, identically for every
  caller: a status above `cited` is set only from a check the server ran.
- Enable `openhuman-embed`'s `mcp` feature in the runner, so its agents can
  also use Jacobian and lean-lsp-mcp. Move the top-level agent to
  `ToolScopeSpec::Named`, since `HostOnly` refuses write tools and MCP
  servers.

## Consequences

- One implementation per tool, tested once, reachable in process and over
  MCP, with a test that both publish the same names and schemas.
- `supermath-mcp` builds and tests in seconds without OpenHuman; CI asserts
  it never gains an OpenHuman dependency.
- MCP clients get the primitives but not the `agent_*` tools, which spend
  ladder budget and need OpenHuman.
- A new dependency, `rmcp`, declared with only its server and stdio
  features.
- The top-level agent loses `HostOnly`'s guarantee; the `Named` list carries
  it instead, pinned by a test.
