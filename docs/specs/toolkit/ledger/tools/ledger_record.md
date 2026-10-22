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
- A `uses` id that is not in the ledger is `NOT_FOUND`; one that repeats
  is `INVALID_INPUT`. Recording an existing claim again with `uses` ids it
  did not have merges them into a new revision, but an id that is the claim
  itself or already reaches it is refused as a cycle, `INVALID_INPUT`, as
  [`ledger_link`](ledger_link.md) refuses it.
- An `arxiv` source needs `id`, `version`, and exactly one of `label` or
  `section`; without that it is `INVALID_INPUT`. It earns `cited` only when
  the call's evidence is not supplied: `"status": "asserted"` or
  `"evidence_origin": "supplied"` pins it at `asserted`.
- `rationale`, `reason`, and `refutes` are stored on the entry and returned
  in full by [`ledger_claims`](ledger_claims.md) `{"id": …}`.
- The answer is `{id, kind, status, evidence_origin, alignment, revision,
  created, lean?}`; when the trust rules held the status down, the reason is
  in `unknowns`. `created: false` marks a repeat.

## Errors

`INVALID_INPUT` (empty statement, bad shape, an `arxiv` source without a
version and exactly one locator, missing `frame`, a repeated or cyclic
`uses` id); `NOT_FOUND`
(a `uses` id, or the Lean file or declaration); `FORBIDDEN` (a status or
origin from the caller, a call id that is not usable); `CONFLICT` (stale
`file_sha256`); `BUDGET_EXCEEDED`; `BACKEND_UNAVAILABLE`. A Lean timeout is
`outcome: "unknown"`.
