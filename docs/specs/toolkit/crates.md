# Crates: one per tool domain

- **Status:** Implemented
- **Parent:** [`README.md`](README.md)
- **Decisions:** [`../../adr/0006-one-crate-per-tool-domain.md`](../../adr/0006-one-crate-per-tool-domain.md),
  [`../../adr/0007-run-python-solvers-as-a-pinned-service.md`](../../adr/0007-run-python-solvers-as-a-pinned-service.md)

Each tool domain is its own crate, with its own spec folder, its own tests,
and its own coverage. A small core crate holds the contract every domain
implements; `supermath-mcp` composes the domains and serves them;
`supermath-agent` is the OpenHuman runner.

```text
crates/
├── supermath-tool/         # the contract: Tool, Envelope, ErrorCode, Kind, Usage, Budget, Project
├── supermath-files/        # the project workspace: Lean and Python files, path rules, write log
├── supermath-lean/         # the Lean service client and the lean_* tools
├── supermath-python/       # the Python solver service client and the py_* tools
├── supermath-literature/   # arXiv, Semantic Scholar, OEIS and the lit_* tools
├── supermath-web/          # Gemini grounded search, page reading and the web_* tools
├── supermath-ledger/       # CortexDB memory, claims, statuses and the ledger_* tools
├── supermath-calc/         # small exact arithmetic: calc_factor, calc_gcd_lcm
├── supermath-subagents/    # the agentic agent_* tools, on OpenHuman
├── supermath-mcp/          # composes the domains into a Toolkit; `supermath-mcp serve`
└── supermath-agent/        # the OpenHuman runner; `supermath-agent check`
```

| Crate | Spec | Kind of tools |
| --- | --- | --- |
| `supermath-tool` | [`tool/`](tool/README.md) | none; the contract |
| `supermath-files` | [`files/`](files/README.md) | primitive |
| `supermath-lean` | [`lean/`](lean/README.md) | primitive |
| `supermath-python` | [`python/`](python/README.md) | primitive |
| `supermath-literature` | [`literature/`](literature/README.md) | primitive |
| `supermath-web` | [`web/`](web/README.md) | agentic (Gemini) and primitive |
| `supermath-ledger` | [`ledger/`](ledger/README.md) | primitive |
| `supermath-calc` | [`calc/`](calc/README.md) | primitive |
| `supermath-subagents` | [`subagents/`](subagents/README.md) | agentic (OpenHuman) |
| `supermath-mcp` | [`mcp/`](mcp/README.md) | none; composition and serving |
| `supermath-agent` | [`../agent-runner.md`](../agent-runner.md), and [below](#supermath-agent) | none; the runner |

## Dependency rules

```text
                         supermath-tool
     ┌────────┬───────────┬────┴────┬────────┬────────┬─────────────┐
   files  literature     web     ledger    calc   subagents ── openhuman-embed
   ┌─┴──┐
 lean  python
     └────────── all primitive domains ──────────┐
                                           supermath-mcp
                                                 │
                                          supermath-agent ── subagents, openhuman-embed
```

- Every domain crate depends on `supermath-tool`. `lean` and `python` also
  depend on `supermath-files`. **No other domain crate depends on another
  domain crate.**
- Where one domain needs another's evidence, it defines a port (a trait),
  and `supermath-mcp` wires the real implementation: the ledger's
  `LeanEvidence` ([`ledger/`](ledger/README.md#crate)) and the files
  crate's `FileObserver` ([`files/`](files/README.md#observers)). Each
  crate is tested against fakes of its ports.
- `supermath-subagents` reaches primitives only through the `ToolRegistry`
  the composition hands it ([`subagents/`](subagents/README.md#crate)).
- Only `supermath-subagents` and `supermath-agent` depend on OpenHuman.
  `cargo test -p` on any other crate never compiles it.
- Only `supermath-web` calls a model (Gemini, as its search backend;
  [ADR 0005](../../adr/0005-search-the-web-with-gemini-grounding.md)), and
  only `supermath-subagents` runs model turns on the ladder.

CI enforces the rules with a script over `cargo metadata`: it fails if a
domain crate depends on a domain crate other than `supermath-files`, or if
a crate other than those two reaches an `openhuman` crate.

## Testing each crate on its own

Each crate is specified, tested, and gated separately:

- **Spec.** Its folder under `docs/specs/toolkit/` holds a `README.md` with
  the domain's shared rules and a `## Crate` section (name, dependencies,
  public API, configuration, tests), and a `tools/` folder with one file
  per tool.
- **Unit tests** in `src/<module>/test.rs`, and **integration tests** in
  the crate's `tests/`, exercising only its public API against loopback
  stubs of its backend and fakes of its ports.
- **Contract tests.** Every domain crate runs
  [`supermath-tool`'s contract checker](tool/README.md#contract-tests) over
  its own tools.
- **Live tests** are named `live_<domain>_*` and gated by the domain's
  environment variables.
- **Coverage** is per crate and per file: at least 90% line coverage,
  reported by `cargo llvm-cov -p <crate>`.

```sh
cargo test -p supermath-lean            # one domain, seconds, no OpenHuman
cargo test --workspace                  # everything
```

## `supermath-agent`

The OpenHuman runner. It depends on `supermath-mcp` for the composed
toolkit and on `supermath-subagents` for the agentic tools.

### Host tools

`host/` wraps each `supermath_tool::Tool` in an OpenHuman `Tool`:

- `name`, `description`, and `parameters_schema` pass through unchanged.
- `exposure` maps `Direct` and `Deferred` to OpenHuman's, so the direct
  tools stay in the catalogue and Jev ranks the rest.
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
- `supermath-subagents` builds the [child agents](subagents/README.md),
  each with a `Named` list of primitives from the registry.
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
lean-lsp-mcp answers never set a status. Jacobian is also importable from
the [Python solver environment](python/README.md#the-solver-environment).

## Compose and images

| Service | Image | Role |
| --- | --- | --- |
| `ladder` | as today | model routing |
| `cortexdb` | as today | memory |
| `lean` | `docker/lean/` | the pinned Lean service ([`lean/`](lean/README.md#the-lean-service)) |
| `python` | `docker/python/` | the pinned Python solver service ([`python/`](python/README.md#the-python-service)) |
| `agent` | `Dockerfile` | `supermath-agent check`, linking the composed toolkit |

`agent`, `lean`, and `python` share the `projects` volume: the agent writes
files there through `file_write` and `file_edit`; the language services
build and run them ([`files/`](files/README.md#layout)). `python` mounts it
read-only; `lean` writes only Lake build outputs. Both sit on an
`internal: true` network with the agent and have no egress.

`docker/mcp/Dockerfile` builds a small image with only `supermath-mcp`, for
running the server without OpenHuman.

## Acceptance criteria

- The dependency-rules script passes in CI, and fails on a fixture
  workspace that breaks each rule.
- Every domain crate runs the contract checker over its tools.
- `cargo test -p <crate>` passes for each domain crate without compiling
  OpenHuman.
- A `supermath-agent` test compares every adapted tool's name, schema,
  exposure, permission, and kind with the composed toolkit's.

## Open questions

1. Should `supermath-calc` fold into `supermath-python` once Python is
   available, or stay as the dependency-free fast path?
