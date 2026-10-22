# supermath-agent

The supermath agent runner. It embeds the
[OpenHuman](https://github.com/tinyhumansai/openhuman) runtime and wires it to:

- the **LLM ladder** for every chat completion and embedding (`flash` at
  `medium` reasoning effort by default);
- **Jev**, the decision model OpenRouter serves, which ranks `tool_search`
  over the agent's deferred tools;
- the **composed toolkit** from [`supermath-mcp`](../supermath-mcp/README.md),
  linked in process and adapted into OpenHuman host tools;
- the **`agent_*` sub-agents** from
  [`supermath-subagents`](../supermath-subagents/README.md), built on the same
  runtime;
- optional **MCP servers**: Jacobian and lean-lsp-mcp, as stdio servers;
- **CortexDB** for durable memory, through the ledger.

The binary checks that everything is wired and lists what the agent can call:

```sh
supermath-agent check [project]     # exit 0 when the stack is ready
```

The behavior is specified in
[`docs/specs/agent-runner.md`](../../docs/specs/agent-runner.md) and
[`docs/specs/toolkit/crates.md`](../../docs/specs/toolkit/crates.md); why it is
built on OpenHuman is in
[`docs/adr/0002-build-on-openhuman.md`](../../docs/adr/0002-build-on-openhuman.md).

## Layout

| Module | Holds |
| --- | --- |
| `config/` | settings read from the environment: the ladder, Jev, the toolkit, the child rungs, and the MCP commands |
| `host/` | `Host` and `HostTool`: each `supermath_tool::Tool` as an OpenHuman `Tool`, dispatched through the toolkit's call log |
| `runner/` | the OpenHuman runtime, the Jev ranker, and `Runner::agent`, which composes the toolkit and registers the agent |
| `main.rs` | `supermath-agent check` |

This crate holds no tool. Tools live in the domain crates; the old in-crate
`factor_integer`, `gcd_lcm`, and memory client are now `calc_factor`,
`calc_gcd_lcm`, and the ledger.

## The agent

`Runner::agent(project)` registers `supermath-<project>` with
`ToolScopeSpec::Named`: every toolkit tool, the four `agent_*` tools, and
`tool_search`. OpenHuman's own `memory` tool is disallowed so it cannot shadow
`recall_memory`. The adapter passes a tool's name, description and schema
through, maps `Exposure` to OpenHuman's, maps `Read` to `ReadOnly` with a
read-only policy and `Write` to `Write`, and returns the envelope as JSON with
the call's id added as `call_id`. An `error` envelope is a reported failure.

`SUPERMATH_JACOBIAN_CMD` and `SUPERMATH_LEAN_LSP_CMD` hold command lines (the
first word runs, the rest are its arguments). When set, the server joins the
agent through `AgentSpec::mcp`, and its tools appear as deferred
`mcp_jacobian_*` and `mcp_lean_lsp_*`.

## Developing

From the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Unit tests cover the host adapter against the real composed toolkit (with no
service configured) and the agent's pinned tool list. `tests/check.rs` runs
the binary against loopback stubs of the ladder, CortexDB, and the OpenHuman
backend. `tests/live.rs` holds `live_subagents_formalize_bench`, which runs
the thirty-statement formalization benchmark through `agent_formalize` against
a real ladder and Lean service; it is skipped unless
`SUPERMATH_LIVE_SUBAGENTS=1`:

```sh
SUPERMATH_LIVE_SUBAGENTS=1 cargo test -p supermath-agent --test live
```

OpenHuman allows one runtime per process, and its agent turns need the large
worker stacks of `openhuman_embed::process::tokio_runtime()`, so tests that
boot the runner run on that runtime.
