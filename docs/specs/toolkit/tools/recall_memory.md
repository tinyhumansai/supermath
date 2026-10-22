# `recall_memory`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Ledger](../ledger.md) | direct | read | shipped; moves in P1 |

**Given → returns:** a query → this project's notes.

The one memory tool that exists today, in `supermath-agent`'s toolkit. It
moves to `supermath-mcp` with the CortexDB client in P1
([`crates.md`](../crates.md)) and answers in the
[envelope](../README.md#one-envelope).

```json
{ "query": "what bounds are known for the Sendov radius" }
```

- Recalls from this project's scope, `{root}/project:{slug}/app:brain`, with
  CortexDB's granular view. The scope comes from the host, never from an
  argument.
- Returns the rendered memory pack, trimmed, or `outcome: "ok"` with empty
  `data` when nothing matched; an empty recall is not evidence that nothing
  was ever recorded, so `unknowns` says the recall is by similarity.
- For structured filters (kind, status, alignment) use
  [`ledger_claims`](ledger_claims.md).

## Errors

`BACKEND_UNAVAILABLE` when CortexDB is down or refuses the request;
`INVALID_INPUT` for an empty query.
