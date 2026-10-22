# Serving the toolkit

- **Status:** Draft
- **Parent:** [`README.md`](README.md)
- **Decision:** [`../../adr/0004-serve-the-toolkit-over-mcp.md`](../../adr/0004-serve-the-toolkit-over-mcp.md)

One `Toolkit` has two faces: OpenHuman host tools for supermath's own
agents, and an MCP server for any other agent (Claude Code, Codex, another
OpenHuman deployment). Both serve the same tools, with the same names,
schemas, and envelopes.

## As OpenHuman host tools

`Runner::agent` registers the top-level agent with the toolkit, as it does
today, with two changes:

- **Scope.** The agent moves from `ToolScopeSpec::HostOnly` to
  `ToolScopeSpec::Named`, listing the toolkit's tools plus `tool_search`.
  `HostOnly` forbids MCP servers and holds every tool to read-only, and the
  toolkit needs both. Naming the tools keeps OpenHuman's other built-ins
  (shell, file, web) out of the agent.
- **Children.** `Runner` builds the [sub-agents](agents.md) alongside the
  top-level agent and hands their handles to the `agent_*` tools.

`Runner` holds the runtime as `Arc<Runtime>` so the MCP server and the host
tools can share it.

## As an MCP server

```sh
supermath-agent mcp [project]      # MCP over stdio
```

- Boots the same `Runner`, so `agent_*` tools work, and registers the
  project's toolkit.
- Serves `tools/list` from the toolkit: each tool's name, description, and
  JSON schema, exactly as OpenHuman sees them. The direct/deferred split
  does not apply over MCP, which has no deferral; every tool is listed.
- Serves `tools/call` by calling the tool's `execute` and returning the
  envelope as the call's text content. An `error` envelope sets the MCP
  result's `isError`.
- The project comes from the command line, never from a tool argument.
- Over MCP the caller is not the harness, so ledger writes from an MCP
  caller have `evidence_origin: supplied` unless a tool in this process
  produced them.

The server is built on `rmcp`, the Rust MCP SDK
([ADR 0004](../../adr/0004-serve-the-toolkit-over-mcp.md)). OpenHuman's own
MCP server serves a fixed catalogue and cannot serve host tools.

Registering it with Claude Code:

```sh
claude mcp add supermath -- supermath-agent mcp my-project
```

## Consuming MCP servers

`openhuman-embed` connects an agent to MCP servers with `AgentSpec::mcp`,
behind its `mcp` feature, which this crate enables. Each remote tool
appears to the agent as a deferred `mcp_<server>_<tool>`, so Jev ranks it
alongside ours.

| Server | Gives | Configured by |
| --- | --- | --- |
| [Jacobian](https://github.com/morluto/jacobian) | `math.find`, `math.run`: ~1,400 exact operations (SymPy, NetworkX, Z3, FLINT) | `SUPERMATH_JACOBIAN_CMD` |
| [lean-lsp-mcp](https://github.com/oOo0oOo/lean-lsp-mcp) | hover, diagnostics, and goals from a Lean language server | `SUPERMATH_LEAN_LSP_CMD` |

Both are optional and off unless their variable is set. The agent's `check`
lists which are connected. Our `lean_*` tools stay the source of verdicts;
lean-lsp-mcp is for interactive exploration only, and its answers never set
a status.

## Compose

`compose.yaml` gains the `lean` service ([`lean.md`](lean.md#the-lean-service)),
and the `agent` service gets `LEAN_URL=http://lean:3150`. Jacobian runs
inside the agent image as a stdio child when enabled.

## Acceptance criteria

- `tests/mcp.rs` starts `supermath-agent mcp` against loopback stubs,
  performs the MCP handshake, lists tools, calls `calc_factor`, and compares
  the result to the host tool's.
- A test asserts the top-level agent's `Named` scope lists exactly the
  toolkit and `tool_search`.
- `supermath-agent check` reports the Lean service and each connected MCP
  server.

## Open questions

1. Should the MCP server also serve HTTP, for agents on another machine?
2. Does an MCP caller get the `agent_*` tools, which spend ladder budget,
   or only the primitives?
