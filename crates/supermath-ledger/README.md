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
| `arxiv`, with a `version` and exactly one of `label` or `section` | `cited` |
| `tool` (`calc_*`, `py_*`, `lean_*`), logged by this process for this project, `ok`, with a `frame` | `checked` |
| `lean`, and `LeanEvidence` says clean with no `sorry`, no debt, axioms in the allowlist | `formalised` |

No tool argument sets a status; `formalised` and `collected` are `FORBIDDEN`
from a caller. Supplied evidence (`"status": "asserted"`,
`"evidence_origin": "supplied"`, or a supplied `CallContext`) earns
`asserted` from every source, `arxiv` included. A `formalised` claim is re-verified on every read, so a changed
file stands as `asserted`, and a claim is no stronger than anything it `uses`.

## Design constraints

- **Depends on `supermath-tool` only.** Lean is reached through the
  `LeanEvidence` port and `tool` sources through `supermath_tool::CallLog`;
  `supermath-mcp` wires the real implementations.
- **Entries are never edited.** A change writes the next revision; readers
  take the highest.
- **Reads filter client-side.** `CortexDB` recall is by similarity, so the
  ledger recalls a window of notes (`DEFAULT_WINDOW`) ranked by similarity
  and filters itself. A read that fills the window may miss entries, which
  `ledger_claims` and `ledger_bound` say in `unknowns`. Nothing records when
  an entry was written, so reads come in recall order, not time order.
- **The scope comes from the host.** Tools read the project from
  `CallContext`; no argument names a scope.

## Tests

`cargo test -p supermath-ledger` runs against loopback stubs only. The
`test-support` feature exposes `memory::stub::{stub, FakeCortex}` and
`FakeLeanEvidence` for other crates' tests.
