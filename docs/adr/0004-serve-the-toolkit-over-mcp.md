# 4. Serve the toolkit over MCP as well as to OpenHuman

- **Status:** Proposed
- **Date:** 2026-10-22

## Context

The toolkit is written as OpenHuman host tools
([`specs/toolkit/serving.md`](../specs/toolkit/serving.md)). Other agents
the maintainers use daily (Claude Code, Codex) speak MCP, and a toolkit they
can call is one we can test by hand and compare against.

OpenHuman has an MCP server, but it serves a fixed catalogue of OpenHuman's
own tools from its CLI; it is not exposed through `openhuman-embed` and
cannot serve a host's tools. OpenHuman's MCP *client* is exposed, as
`AgentSpec::mcp` behind the embed crate's `mcp` feature, and lets our agents
use Jacobian and lean-lsp-mcp.

Two ways to serve MCP were considered: hand-writing the JSON-RPC protocol,
or `rmcp`, the Rust SDK maintained under the Model Context Protocol
organization.

## Decision

- Keep one `Toolkit`. Add a `supermath-agent mcp [project]` subcommand that
  serves it over stdio with `rmcp`, translating `tools/list` and
  `tools/call` to the same `Tool` implementations OpenHuman calls.
- Enable `openhuman-embed`'s `mcp` feature, so agents can consume Jacobian
  and lean-lsp-mcp as optional MCP servers.
- Move the top-level agent from `ToolScopeSpec::HostOnly` to
  `ToolScopeSpec::Named`, since `HostOnly` refuses MCP servers and write
  tools. Name every allowed tool explicitly.

## Consequences

- One implementation per tool, tested once, reachable two ways.
- A new dependency, `rmcp`, with its own release cadence; it is declared
  with a comment and only the server and stdio features.
- The top-level agent loses `HostOnly`'s guarantee that nothing but host
  tools can run. The `Named` list carries that guarantee instead, and a test
  pins it.
- An MCP caller is not the harness: evidence it supplies is recorded as
  `supplied`, so the ledger's trust rules hold for both faces.
