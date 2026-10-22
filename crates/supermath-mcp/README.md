# supermath-mcp

Composes the supermath tool domains into one `Toolkit` per project, wires
their ports to each other, and serves the result over MCP. Spec:
[`docs/specs/toolkit/mcp/`](../../docs/specs/toolkit/mcp/README.md).

The crate owns no tools. It depends on `supermath-tool` and the primitive
domains (`files`, `lean`, `python`, `literature`, `web`, `ledger`, `calc`),
never on OpenHuman, so `cargo test -p supermath-mcp` and the
`docker/mcp/Dockerfile` image do not compile it.

## Public surface

| Item | Role |
| --- | --- |
| `Config` | the union of the domains' environment, read once (`from_env`, `from_lookup`) |
| `Toolkit::compose(&config, project)` | builds every domain, wires the ports, registers 30 tools |
| `Toolkit::registry()` / `dispatch` / `dispatch_with` / `call_log` | the tools, a logged call, and the log the ledger reads |
| `LeanBridge` | the ledger's `LeanEvidence` port, over the Lean checker, bound to the project |
| `RecordedClaims`, `RecordsClaims` | the Lean `ClaimIndex`, fed by wrapping `ledger_record` |
| `McpServer`, `serve`, `serve_on` | the `rmcp` handler and the stdio (or any transport) entry points |
| `check::run` | the backend probes behind `supermath-mcp check` |
| `cli::{parse, execute, run}` | the command line, used by `main.rs` |

## Design

- **One call log.** `Toolkit::dispatch` records every call, tool errors
  included, in the `InMemoryCallLog` that the ledger holds as its `CallLog`.
  A `tool` source in `ledger_record` is valid only if that log has the call,
  in this project, for that tool, ending `ok`.
- **The trust boundary lives in the ledger.** `LeanBridge` returns a verdict
  (running the same checker as `lean_check`); the ledger decides what it
  earns. Arguments naming a status or `collected` evidence are refused before
  Lean runs.
- **Unconfigured is not absent.** The catalogue is the same with or without
  `LEAN_URL`, `PYTHON_URL`, `GEMINI_API_KEY`, or `CORTEX_API_KEY`; the tools
  answer `BACKEND_UNAVAILABLE`.
- **The project is argv.** `serve [project]` slugs the name once. No tool
  schema has a `project` field and the schemas forbid extra properties.
- **Stdout is the protocol.** Nothing but `rmcp` writes to it; errors and
  logs go to stderr.
- **The claim index is in-process.** `lean`'s `ClaimIndex` is synchronous and
  the ledger is remote, so the index learns claims from `ledger_record` calls
  made through this toolkit. After a restart `lean_blueprint` shows
  `claim_id: null` until a claim is recorded again.

## MCP wire form

- `tools/list`: every tool, `readOnlyHint` from its permission, and
  `_meta.supermath = {exposure, kind}` (MCP annotations have no field for
  them).
- `tools/call`: the envelope as one JSON text block, plus `call_id`;
  `isError: true` only for an `error` envelope.

## Tests

- `src/*/test.rs`: configuration, the claim index, descriptors, the server over
  an in-memory pipe, the toolkit with nothing configured.
- `tests/ports.rs`: the ports against a loopback Lean service and a fake
  CortexDB (promotion by a stubbed check, forged status, stale digest, timeout,
  unavailable, file observers, `tool` sources, the claim index).
- `tests/mcp.rs`: the spec's acceptance criteria over MCP.
- `tests/check.rs`, `tests/cli.rs`: the probes and the binary's exit codes.
- `tests/serve.rs`: spawns the binary and handshakes over stdio.

None touch the real network.
