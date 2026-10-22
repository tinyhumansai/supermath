# MCP: composing and serving the toolkit

- **Status:** Draft
- **Crate:** `supermath-mcp`
- **Parent:** [`../README.md`](../README.md)
- **Decisions:** [`../../../adr/0004-split-the-toolkit-into-an-mcp-crate.md`](../../../adr/0004-split-the-toolkit-into-an-mcp-crate.md),
  [`../../../adr/0006-one-crate-per-tool-domain.md`](../../../adr/0006-one-crate-per-tool-domain.md)

`supermath-mcp` owns no tools. It composes the domain crates into one
`Toolkit` for a project, wires each domain's ports to the others, and
serves the result over MCP. The runner links the same composition.

## Composition

```rust
let toolkit = Toolkit::compose(&config, project)?;
```

`compose` builds every domain's tools and wires the ports:

| Port | Defined in | Wired to |
| --- | --- | --- |
| `FileObserver` (Lean) | `supermath-files` | `supermath-lean`'s verdict cache |
| `FileObserver` (Python) | `supermath-files` | `supermath-python`'s run log |
| `LeanEvidence` | `supermath-ledger` | `supermath-lean`'s checker |
| `CallLog` | `supermath-tool` | the process-wide call log every tool writes |

A domain whose service is not configured (no `LEAN_URL`, no `PYTHON_URL`,
no `GEMINI_API_KEY`) still registers its tools; they answer
`BACKEND_UNAVAILABLE`, so the catalogue does not change shape with the
environment. `Toolkit::registry()` returns the primitives as a
`ToolRegistry` for `supermath-subagents`.

### The trust boundary

The trust rules ([`../README.md`](../README.md#claim-status)) hold for every
caller, our runner included:

- A status above `cited` is never taken from arguments. `ledger_record`
  with a `lean` source runs `lean_check` through the `LeanEvidence` port,
  and the claim becomes `formalised` only if that check is `checked` with
  no debt.
- `evidence_origin: collected` means a tool in this process produced the
  evidence, as the call log records. Anything else is `supplied`.

## The MCP server

```sh
supermath-mcp serve [project]      # MCP over stdio
supermath-mcp check                # Lean, Python, CortexDB, Gemini, and arXiv reachable
```

- `tools/list` serves each tool's name, description, and schema. MCP has no
  deferral, so every tool is listed; `exposure` and `kind` are published in
  the tool's annotations.
- `tools/call` returns the envelope as JSON text content, and sets
  `isError` on an `error` envelope.
- The project comes from the command line, never from a tool argument.
- Read-only tools are annotated `readOnlyHint: true`; `file_write`,
  `file_edit`, and the `ledger_*` writes are not.
- The `agent_*` tools are not served: they need OpenHuman and live in
  `supermath-subagents`, which this crate does not depend on.

Registering it with Claude Code:

```sh
claude mcp add supermath -- supermath-mcp serve my-project
```

## Configuration

The union of the domains' variables, read once and handed to each domain:

| Domain | Variables |
| --- | --- |
| files | `SUPERMATH_PROJECTS` |
| lean | `LEAN_URL` |
| python | `PYTHON_URL`, `SUPERMATH_PY_DEFAULT_TIMEOUT`, `SUPERMATH_PY_MAX_TIMEOUT` |
| literature | `SUPERMATH_CONTACT`, `SEMANTIC_SCHOLAR_API_KEY`, `SUPERMATH_CACHE` |
| web | `GEMINI_API_KEY`, `SUPERMATH_WEB_MODEL`, `SUPERMATH_WEB_DAILY_CAP` |
| ledger | `CORTEX_API_URL`, `CORTEX_API_KEY`, `CORTEX_SCOPE_ROOT` |

## Crate

| | |
| --- | --- |
| Name | `supermath-mcp` |
| Depends on | `supermath-tool` and every primitive domain crate (`files`, `lean`, `python`, `literature`, `web`, `ledger`, `calc`), `rmcp` (server and stdio features) |
| Public API | `Toolkit`, `Config`, and the `serve` entry point |
| Tests | composition with every backend stubbed; the ports wired to the right domains; `tests/serve.rs` performs the MCP handshake, lists tools, and calls `calc_factor` and `file_write` |

## Acceptance criteria

- `tools/list` returns every primitive tool in the catalogue and no
  `agent_*` tool.
- With no `LEAN_URL`, `lean_check` answers `BACKEND_UNAVAILABLE` and the
  catalogue is unchanged.
- A `ledger_record` with a `lean` source, served over MCP, is promoted only
  by the stubbed Lean check, never by its arguments.

## Open questions

1. Should `serve` also speak streamable HTTP, for agents on another
   machine?
