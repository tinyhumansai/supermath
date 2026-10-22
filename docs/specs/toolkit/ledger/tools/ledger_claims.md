# `ledger_claims`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-ledger` | [Ledger](../README.md) | primitive | deferred | read | P2 |

**Given → returns:** filters → paged claims.

Given filters (`kind`, minimum `status`, `alignment`, or a text query),
returns entries as `{id, kind, status, alignment, statement}` with
statements trimmed to 300 characters. `ledger_claims {"id": …}` returns
one entry in full, with its effective status and the chain of claims that
limit it.

- `status` in a row is the **effective** status. `min_status` filters on it.
- Filters other than `query` are applied client-side
  ([`../README.md#open-questions`](../README.md#open-questions)); `query`
  steers the similarity recall.
- Rows come in recall order: the order CortexDB ranked the recalled notes
  for `query` (or a default query), each claim at the place of the first of
  its notes. That is not time order, and nothing makes it so: an entry has
  no timestamp, the recalled notes carry only their text, and `revision`
  counts within one claim
  ([`../README.md#open-questions`](../README.md#open-questions)).
- `cursor` is an offset. `limit` is 10 by default and a larger one is
  capped at 50. A cursor past the last row is an empty page with no cursor.
- `{"id": …}` returns `{claim, effective_status, limited_by}` and cannot be
  combined with `kind`, `min_status`, `alignment`, `query`, or `cursor`.
- When the recall returned as many notes as the window holds, `unknowns`
  says older entries may be missing.
- Arguments are checked before the budget: a call with `id` and a filter,
  or a bad cursor, is `INVALID_INPUT` whatever its budget.

## Errors

`INVALID_INPUT` (a bad cursor, or `id` with filters); `NOT_FOUND` (an
unknown id); `BUDGET_EXCEEDED`; `BACKEND_UNAVAILABLE`.
