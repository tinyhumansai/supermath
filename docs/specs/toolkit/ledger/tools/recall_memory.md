# `recall_memory`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-ledger` | [Ledger](../README.md) | primitive | direct | read | P1 |

**Given → returns:** a query → this project's notes.

The one memory tool that exists today, in `supermath-agent`'s toolkit. It
moves to `supermath-ledger` with the CortexDB client in P1
([`crates.md`](../../crates.md)) and answers in the
[envelope](../../README.md#one-envelope).

```json
{ "query": "what bounds are known for the Sendov radius" }
```

- Recalls from this project's scope, `{root}/project:{slug}/app:brain`, with
  CortexDB's granular view. The scope comes from the host, never from an
  argument.
- Returns `{"notes": …}`, the rendered memory pack trimmed to 8,000
  characters, or `outcome: "ok"` with `{"notes": ""}` when nothing matched;
  an empty recall is not evidence that nothing was ever recorded, so
  `unknowns` says the recall is by similarity.
- For structured filters (kind, status, alignment) use
  [`ledger_claims`](ledger_claims.md).

## Errors

`BACKEND_UNAVAILABLE` when CortexDB is down or refuses the request;
`INVALID_INPUT` for an empty query or an unknown argument;
`BUDGET_EXCEEDED` when the query is larger than the call's byte budget.
