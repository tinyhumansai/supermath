# Crates: the MCP toolkit and the OpenHuman runner

- **Status:** Draft
- **Parent:** [`README.md`](README.md)
- **Decision:** [`../../adr/0004-split-the-toolkit-into-an-mcp-crate.md`](../../adr/0004-split-the-toolkit-into-an-mcp-crate.md)

The toolkit and the agent that uses it are two crates. `supermath-mcp` is
the toolkit: every primitive tool, the Lean, literature, and CortexDB
clients, and an MCP server, with no dependency on OpenHuman. `supermath-agent`
is the OpenHuman runner: the runtime, the ladder and Jev wiring, and the
sub-agents. The runner depends on the toolkit; the toolkit never depends on
the runner.

```text
crates/
├── supermath-mcp/      # the toolkit; serves MCP; no OpenHuman
│   └── src/
│       ├── lib.rs      # Tool trait, Toolkit, Envelope: the public surface
│       ├── main.rs     # `supermath-mcp serve [project]`, `supermath-mcp check`
│       ├── config/     # LEAN_URL, CORTEX_*, literature settings
│       ├── error/
│       ├── envelope/   # Outcome, Envelope, ErrorCode, Coverage
│       ├── tool/       # the Tool trait, Exposure, Permission
│       ├── lean/       # Lean service client, verdict reduction
│       ├── literature/ # arXiv, Semantic Scholar, OEIS; LaTeX flattening
│       ├── web/        # Gemini grounded search, redirect resolution, page reading
│       ├── memory/     # the CortexDB client (moved from supermath-agent)
│       ├── ledger/     # entries, status transitions, effective status
│       ├── calc/       # factor, gcd/lcm, counterexample search
│       ├── toolkit/    # one file per area binding clients to tools
│       └── server/     # the MCP server over Toolkit (rmcp)
└── supermath-agent/    # the OpenHuman runner
    └── src/
        ├── lib.rs
        ├── main.rs     # `supermath-agent check [project]`
        ├── config/     # ladder, Jev, models, workspace
        ├── error/
        ├── host/       # adapts supermath-mcp tools into OpenHuman tools
        ├── subagents/  # child agents and the agent_* tools
        └── runner/     # Runtime, Jev ranker, agent registration
```

## Dependency direction

| Crate | Depends on | Never depends on |
| --- | --- | --- |
| `supermath-mcp` | `rmcp`, `reqwest`, `serde_json`, `tokio`, `thiserror` | OpenHuman, the ladder, any model except Gemini for web search |
| `supermath-agent` | `supermath-mcp`, `openhuman-embed`, `openhuman-tinyhumans` (`jev`) | — |

`supermath-mcp` calls no model, with one exception: Gemini, as the backend
of the `web_*` tools, whose prose is kept only where Google Search
grounding ties it to a source ([`web.md`](web.md),
[ADR 0005](../../adr/0005-search-the-web-with-gemini-grounding.md)).
Everything else that needs a model (reading a paper
for meaning, writing Lean, reviewing alignment) is an `agent_*` tool in
`supermath-agent`. This keeps the toolkit deterministic and testable
without a provider, and lets any MCP client use it without our ladder.

Both crates are workspace members and share one `Cargo.lock`. Building or
testing `supermath-mcp` alone (`cargo test -p supermath-mcp`) never compiles
OpenHuman; `supermath-agent` still does.

## `supermath-mcp`

### The `Tool` trait

The toolkit's own trait, small enough to adapt to both OpenHuman and MCP:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn schema(&self) -> serde_json::Value;   // JSON Schema of the arguments
    fn exposure(&self) -> Exposure;          // Direct | Deferred
    fn permission(&self) -> Permission;      // Read | Write
    async fn call(&self, args: serde_json::Value) -> Envelope;
}
```

`call` never returns `Err`: a failure is an `error` envelope with a code
([`README.md`](README.md#one-envelope)). `Toolkit::new(config, project)`
builds every tool bound to one project, and `Toolkit::tools()` lists them
in catalogue order.

### The trust boundary

The trust rules ([`README.md`](README.md#claim-status)) are enforced here,
because this is the only crate that runs Lean and writes the ledger. The
server treats every caller the same, whether it is our runner, Claude
Code, or anything else:

- A status above `cited` is never taken from arguments. `ledger_record`
  with a `lean` source makes the server run `lean_check` itself, and the
  claim becomes `formalised` only if that check is `checked` with no debt.
- `evidence_origin: collected` means a tool in this `supermath-mcp` process
  produced the evidence. Anything else is `supplied`.

So the runner gets no special privileges, and the sub-agents' trust
invariant ([`agents.md`](agents.md#invariants)) holds by construction.

### The MCP server

```sh
supermath-mcp serve [project]      # MCP over stdio
supermath-mcp check                # Lean service, CortexDB, and arXiv reachable
```

- `tools/list` serves each tool's name, description, and schema. MCP has no
  deferral, so every tool is listed; `exposure` is published in the tool's
  annotations for clients that can use it.
- `tools/call` returns the envelope as JSON text content, and sets
  `isError` on an `error` envelope.
- The project comes from the command line, never from a tool argument.
- Read-only tools are annotated `readOnlyHint: true`; `lean_write`,
  `ledger_*` writes are not.

Registering it with Claude Code:

```sh
claude mcp add supermath -- supermath-mcp serve my-project
```

### Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `LEAN_URL` | `http://localhost:3150` | the Lean service |
| `CORTEX_API_URL`, `CORTEX_API_KEY`, `CORTEX_SCOPE_ROOT` | as today | CortexDB, moved from the runner |
| `SUPERMATH_CONTACT` | required for arXiv | the contact in the arXiv `User-Agent` |
| `SEMANTIC_SCHOLAR_API_KEY` | unset | raises the Semantic Scholar rate limit |
| `SUPERMATH_CACHE` | `./.supermath/cache` | literature and web response cache |
| `GEMINI_API_KEY` | unset | Gemini API key; without it the `web_*` tools are unavailable |
| `SUPERMATH_WEB_MODEL` | a pinned Gemini Flash model | the model behind grounded search |
| `SUPERMATH_WEB_DAILY_CAP` | `200` | `web_*` calls per project per day |

## `supermath-agent`

### Host tools

`host/` wraps each `supermath_mcp::Tool` in an OpenHuman `Tool`:

- `name`, `description`, and `parameters_schema` pass through unchanged.
- `exposure` maps `Direct` and `Deferred` to OpenHuman's, so the four
  direct tools stay in the catalogue and Jev ranks the rest.
- `permission` maps `Read` to `PermissionLevel::ReadOnly` and
  `ToolPolicy::read_only()`, and `Write` to `PermissionLevel::Write`.
- `execute` calls `call` and returns the envelope as `ToolResult::json`.

The runner links the toolkit in process rather than connecting to the MCP
server, so there is no second process to supervise, and the direct/deferred
split survives (OpenHuman marks every MCP tool deferred). A test asserts
the adapted tools and the MCP server publish identical names and schemas.

### Agents

- The top-level agent uses `ToolScopeSpec::Named`, listing the adapted
  toolkit, the `agent_*` tools, and `tool_search`. `HostOnly` refuses write
  tools and MCP servers, and the toolkit needs both.
- `subagents/` builds the [child agents](agents.md), each with its own
  `Named` subset of the adapted toolkit.
- The runtime is held as `Arc<Runtime>`.

### Consuming other MCP servers

`openhuman-embed`'s `mcp` feature, enabled in this crate, connects agents
to outside MCP servers with `AgentSpec::mcp`. Each remote tool appears as a
deferred `mcp_<server>_<tool>`, ranked by Jev alongside ours.

| Server | Gives | Configured by |
| --- | --- | --- |
| [Jacobian](https://github.com/morluto/jacobian) | `math.find`, `math.run`: ~1,400 exact operations | `SUPERMATH_JACOBIAN_CMD` |
| [lean-lsp-mcp](https://github.com/oOo0oOo/lean-lsp-mcp) | hover, diagnostics, and goals from a Lean language server | `SUPERMATH_LEAN_LSP_CMD` |

Both are optional. Our `lean_*` tools stay the source of verdicts;
lean-lsp-mcp answers never set a status.

## Compose and images

| Service | Image | Role |
| --- | --- | --- |
| `ladder` | as today | model routing |
| `cortexdb` | as today | memory |
| `lean` | `docker/lean/` | the pinned Lean service ([`lean.md`](lean.md#the-lean-service)) |
| `agent` | `Dockerfile` | `supermath-agent check`, linking `supermath-mcp` |

`docker/mcp/Dockerfile` builds a small image with only `supermath-mcp`, for
running the server without OpenHuman. It is not a compose service, since
stdio servers are started by their client.

## Acceptance criteria

- `cargo tree -p supermath-mcp` contains no `openhuman` crate; a CI step
  asserts it.
- `supermath-mcp`'s tests run without the ladder, OpenHuman, or a model.
- `crates/supermath-mcp/tests/serve.rs` starts `supermath-mcp serve`
  against loopback stubs, performs the MCP handshake, lists tools, and
  calls `calc_factor`.
- A `supermath-agent` test compares every adapted tool's name, schema,
  exposure, and permission with the toolkit's, and its envelope for the
  same call with the MCP server's.
- A test pins the top-level agent's `Named` scope.

## Open questions

1. Should `supermath-mcp serve` also speak streamable HTTP, for agents on
   another machine?
2. Does `supermath-agent` ever need to reach a remote `supermath-mcp` over
   MCP instead of linking it, for example to share one Lean service across
   hosts?
