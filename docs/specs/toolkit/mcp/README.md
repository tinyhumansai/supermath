# MCP: composing and serving the toolkit

- **Status:** Implemented
- **Crate:** `supermath-mcp`
- **Parent:** [`../README.md`](../README.md)
- **Decisions:** [`../../../adr/0004-split-the-toolkit-into-an-mcp-crate.md`](../../../adr/0004-split-the-toolkit-into-an-mcp-crate.md),
  [`../../../adr/0006-one-crate-per-tool-domain.md`](../../../adr/0006-one-crate-per-tool-domain.md)

`supermath-mcp` owns no tools. It composes the domain crates into one
`Toolkit` for a project, wires each domain's ports to the others, and
serves the result over MCP, or one call per process (`call`) for the seats
of an rsi-ralph [loop](../../loops.md).

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
| `ClaimIndex` | `supermath-lean` | the ledger claims recorded through this toolkit ([below](#the-claim-index)) |
| `CallLog` | `supermath-tool` | the call log every dispatch writes, and the ledger reads: in memory, or for `serve` and `call` the project's `.log/calls.jsonl`, shared by every process |

A domain whose service is not configured (no `LEAN_URL`, no `PYTHON_URL`,
no `GEMINI_API_KEY`) still registers its tools; they answer
`BACKEND_UNAVAILABLE`, so the catalogue does not change shape with the
environment. `Toolkit::registry()` returns every domain tool (the primitives
and `web_search` and `web_research`, which call Gemini) as a `ToolRegistry`,
for the server, `hive-tools`, and `loop new`. `Toolkit::dispatch`
runs a call in a context built from the composed project, records it in the
call log, and returns the envelope with its `call_id`.

`LeanEvidence` is `LeanBridge`: it runs the checker behind `lean_check` for
the composed project. A missing file, an undeclared name, or a path that is
not a Lean file is `EvidenceNotFound`; no service is `EvidenceUnavailable`; a
check that timed out is `EvidenceTimeout`, which `ledger_record` answers as
`unknown`.

### The claim index

`lean_blueprint` marks each declaration with the ledger claim that rests on
it, through `lean`'s synchronous `ClaimIndex`. The ledger is asynchronous
and remote, so the port cannot query it. `compose` wraps `ledger_record` so
that every successful call with a `lean` source adds
`(project, path, declaration) -> claim id` to an in-process index. It
therefore covers claims recorded through this toolkit since the process
started; a claim recorded earlier shows `claim_id: null` until it is
recorded again, which is idempotent.

### The trust boundary

The trust rules ([`../README.md`](../README.md#claim-status)) hold for every
caller, a loop's seats included:

- A status above `cited` is never taken from arguments. `ledger_record`
  with a `lean` source runs `lean_check` through the `LeanEvidence` port,
  and the claim becomes `formalised` only if that check is `checked` with
  no debt.
- `evidence_origin: collected` means a tool in this process produced the
  evidence, as the call log records. Anything else is `supplied`.

## The MCP server

```sh
supermath-mcp serve [project]          # MCP over stdio
supermath-mcp check [project]          # Lean, Python, CortexDB, Gemini, and arXiv reachable
supermath-mcp call <tool> [project]    # one call: JSON args on stdin, the tools/call body on stdout
supermath-mcp hive-tools [project]     # every tool as a tinyhivemind executable-tool declaration
supermath-mcp loop new <slug> <statement.lean> [dir]   # an rsi-ralph loop folder (docs/specs/loops.md)
supermath-mcp score proof <project>    # that loop's scorer, run by rsi-ralph
# options: --project-dir <dir> (the project's directory), --env-file <file> (unset variables)
```

- `tools/list` serves each tool's name, description, and schema. MCP has no
  deferral, so every tool is listed. MCP's annotations have no field for
  exposure or kind, so they are published in the tool's `_meta`:
  `{"supermath": {"exposure": "direct", "kind": "primitive"}}`, with kind
  `agentic:gemini` for the two Gemini tools.
- `tools/call` returns the envelope as JSON text content with the call's
  id added as `call_id` (a `ledger_record` `tool` source quotes it), and
  sets `isError` on an `error` envelope. An `unknown` envelope is an answer,
  not an error.
- The project comes from the command line, never from a tool argument.
- Read-only tools are annotated `readOnlyHint: true`; `file_write`,
  `file_edit`, and the `ledger_*` writes are not.
- `call` prints the same body as `tools/call` (the envelope with its
  `call_id`, from one function), and exits 0 for every envelope, an
  `error` one included. It exits non-zero only on a host failure, with
  nothing on stdout.
- `hive-tools` writes one declaration per tool: name, description, schema,
  a `call` command fixed to the project with `--project-dir .`, and
  `side_effects` from the tool's permission (`read` or `write`). Exposure
  is not part of a declaration.

`check` reports the Lean toolchain and Mathlib revision, the Python version
and lock digest, whether CortexDB is ready, whether `GEMINI_API_KEY` is set
(never its value), and whether arXiv answers a one-result search. A backend
whose variable is unset is `skipped`; a configured one that does not answer
is `FAILED` and the exit status is 1.

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

Without `CORTEX_API_KEY` the ledger is built with a placeholder key that
CortexDB refuses, so `recall_memory` and the `ledger_*` tools answer
`BACKEND_UNAVAILABLE`; `check` reports CortexDB as skipped.

## Crate

| | |
| --- | --- |
| Name | `supermath-mcp` |
| Depends on | `supermath-tool` and every primitive domain crate (`files`, `lean`, `python`, `literature`, `web`, `ledger`, `calc`), `rmcp` 3 (features `server` and `transport-io`) |
| Public API | `Toolkit` (`compose`, `compose_persistent`, `registry`, `dispatch`, `dispatch_with`, `call_log`), `Config`, `LeanBridge`, `RecordedClaims`, `McpServer`, `serve`, `serve_on`, and the `check` and `cli` modules |
| Tests | composition with every backend stubbed; the ports wired to the right domains (`tests/ports.rs`); `tests/mcp.rs` drives the server over an in-memory pipe; `tests/serve.rs` performs the MCP handshake with the binary, lists tools, and calls `calc_factor` and `file_write`; `tests/cli.rs` runs `call`, `hive-tools`, `loop new`, and `score proof` |

## Acceptance criteria

- `tools/list` returns every primitive tool in the catalogue and no
  `agent_*` tool.
- With no `LEAN_URL`, `lean_check` answers `BACKEND_UNAVAILABLE` and the
  catalogue is unchanged.
- A `ledger_record` with a `lean` source, served over MCP, is promoted only
  by the stubbed Lean check, never by its arguments.
- Exactly six tools are `direct` (`recall_memory`, `lit_search_arxiv`,
  `web_search`, `lean_check`, `py_run`, `ledger_record`) and every
  description passes the contract's description rule.
- `call` prints the envelope and appends it to `.log/calls.jsonl`, with a
  `call_id` unique across processes, and answers an error envelope with
  exit 0 (`tests/cli.rs`). No test compares a `serve` answer with a `call`
  answer directly; both render it through the same function.

## Open questions

1. Should `serve` also speak streamable HTTP, for agents on another
   machine?
