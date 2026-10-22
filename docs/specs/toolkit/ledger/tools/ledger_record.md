# `ledger_record`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-ledger` | [Ledger](../README.md) | primitive | direct | write | P2 |

**Given → returns:** a claim or fact → its id and status.

Given a kind, a statement, and a source, records an entry and returns its
`id` and status. Optional: `uses`, a `lead`'s rationale, a `dead_end`'s
reason. The status is computed, not passed.

```json
{ "kind": "result", "statement": "…", "source": { "lean": { "path": "lean/Main.lean",
  "declaration": "Main.thm", "file_sha256": "…" } }, "uses": ["clm_…"] }
```

- Also optional: `refutes` (a `refutation`'s claim id) and `frame` (the
  finite range a `tool` source covered; required for `checked`).
- `status` and `evidence_origin` are accepted only so they can be refused:
  see [`../README.md#status`](../README.md#status).
- A `uses` id that is not in the ledger is `NOT_FOUND`.
- The answer is `{id, kind, status, evidence_origin, alignment, revision,
  created, lean?}`; when the trust rules held the status down, the reason is
  in `unknowns`. `created: false` marks a repeat.

## Errors

`INVALID_INPUT` (empty statement, bad shape, missing `frame`); `NOT_FOUND`
(a `uses` id, or the Lean file or declaration); `FORBIDDEN` (a status or
origin from the caller, a call id that is not usable); `CONFLICT` (stale
`file_sha256`); `BUDGET_EXCEEDED`; `BACKEND_UNAVAILABLE`. A Lean timeout is
`outcome: "unknown"`.
