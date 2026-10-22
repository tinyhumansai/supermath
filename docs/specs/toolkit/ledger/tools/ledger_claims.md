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
- Every filter is applied client-side to an exact listing of the
  project's notes (`GET /v1/events`), not to a similarity recall
  ([`../README.md#open-questions`](../README.md#open-questions)).
- Rows come newest first: the order the server stored the notes in,
  reversed, each claim at the place of its newest revision. `query` orders
  the rows, the claims whose statements hold the most of its words first;
  it never removes a row.
- `cursor` is an offset. `limit` is 10 by default and a larger one is
  capped at 50. A cursor past the last row is an empty page with no cursor.
- `{"id": …}` returns `{claim, effective_status, limited_by}` and cannot be
  combined with `kind`, `min_status`, `alignment`, `query`, or `cursor`.
- When the listing stopped at the window (`DEFAULT_WINDOW`, 5,000 notes)
  with notes left unread, `unknowns` says newer entries may be missing.
- Arguments are checked before the budget: a call with `id` and a filter,
  or a bad cursor, is `INVALID_INPUT` whatever its budget.

## Errors

`INVALID_INPUT` (a bad cursor, or `id` with filters); `NOT_FOUND` (an
unknown id); `BUDGET_EXCEEDED`; `BACKEND_UNAVAILABLE`.
