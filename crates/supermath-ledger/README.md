# supermath-ledger

The claim ledger: what the project knows and how well it knows it, kept in
`CortexDB`. It owns the `CortexDB` client (moved from `supermath-agent`), the
trust rules that decide a claim's status, and five tools: `recall_memory`,
`ledger_record`, `ledger_claims`, `ledger_link`, and `ledger_bound`. The spec
is [`docs/specs/toolkit/ledger/`](../../docs/specs/toolkit/ledger/README.md).

## Public surface

| Module | Exports |
| --- | --- |
| `memory` | `Memory` (recall, `recall_notes`, remember, ready), `MemoryProject`, `slugify`, `render_pack`, `content_digest` |
| `config` | `Config` (`CORTEX_API_URL`, `CORTEX_API_KEY`, `CORTEX_SCOPE_ROOT`) |
| `types` | `Entry`, `Kind`, `Status`, `Source`, `Alignment`, `Direction`, `LeanClaim`, `BoundSpec` |
| `evidence` | `LeanEvidence` (port), `LeanVerdict`, `LeanVerdictKind`, `AXIOM_ALLOWLIST`, `FakeLeanEvidence` (feature `test-support`) |
| `ledger` | `Ledger`, `Claim`, `Recorded`, `BoundRequest`, `BoundOutcome`, `Effective` |
| `tools` | `tools(&Memory, &Arc<Ledger>)`, `RecallMemory`, `LedgerRecord`, `LedgerClaims`, `LedgerLink`, `LedgerBound` |

## Trust, in one table

| Source | Earns |
| --- | --- |
| `supplied`, `web` | `asserted` |
| `arxiv` | `cited` |
| `tool` (`calc_*`, `py_*`, `lean_*`), logged by this process for this project, `ok`, with a `frame` | `checked` |
| `lean`, and `LeanEvidence` says clean with no `sorry`, no debt, axioms in the allowlist | `formalised` |

No tool argument sets a status; `formalised` and `collected` are `FORBIDDEN`
from a caller. A `formalised` claim is re-verified on every read, so a changed
file stands as `asserted`, and a claim is no stronger than anything it `uses`.

## Design constraints

- **Depends on `supermath-tool` only.** Lean is reached through the
  `LeanEvidence` port and `tool` sources through `supermath_tool::CallLog`;
  `supermath-mcp` wires the real implementations.
- **Entries are never edited.** A change writes the next revision; readers
  take the highest.
- **Reads filter client-side.** `CortexDB` recall is by similarity, so the
  ledger recalls a window of recent notes (`DEFAULT_WINDOW`) and filters
  itself. A ledger larger than the window can hide old entries from a list.
- **The scope comes from the host.** Tools read the project from
  `CallContext`; no argument names a scope.

## Tests

`cargo test -p supermath-ledger` runs against loopback stubs only. The
`test-support` feature exposes `memory::stub::{stub, FakeCortex}` and
`FakeLeanEvidence` for other crates' tests.
