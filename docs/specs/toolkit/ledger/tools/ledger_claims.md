# `ledger_claims`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-ledger` | [Ledger](../README.md) | primitive | deferred | read | P2 |

**Given → returns:** filters → paged claims.

Given filters (`kind`, minimum `status`, `alignment`, or a text query),
returns entries, newest first, as `{id, kind, status, alignment, statement}`
with statements trimmed to 300 characters. `ledger_claims {"id": …}` returns
one entry in full, with its effective status and the chain of claims that
limit it.

- `status` in a row is the **effective** status. `min_status` filters on it.
- Filters other than `query` are applied client-side
  ([`../README.md#open-questions`](../README.md#open-questions)); `query`
  steers the similarity recall.
- `cursor` is an offset. `limit` is 10 by default, 50 at most.
- `{"id": …}` returns `{claim, effective_status, limited_by}` and cannot be
  combined with filters.

## Errors

`INVALID_INPUT` (a bad cursor, or `id` with filters); `NOT_FOUND` (an
unknown id); `BUDGET_EXCEEDED`; `BACKEND_UNAVAILABLE`.
