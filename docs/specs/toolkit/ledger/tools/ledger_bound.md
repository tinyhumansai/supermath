# `ledger_bound`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-ledger` | [Ledger](../README.md) | primitive | deferred | write | P6 |

**Given → returns:** a quantity and a bound → the best bound so far.

Given a named quantity, a direction (`upper` or `lower`), a value, and a
source, records it if it improves on the best bound so far, and returns the
best bound either way. Bounds only improve: a worse bound is kept as a
`fact`, not as the bound.

```json
{ "quantity": "Sendov radius", "direction": "upper", "value": 2.5,
  "source": { "supplied": "…" } }
```

- `quantity` is whitespace-normalised and is the key; `value` is a JSON
  number, compared exactly for integers and as a float otherwise.
- The source earns its status by the same rules as
  [`ledger_record`](ledger_record.md) (`frame` applies to a `tool` source), so
  a bound cannot be raised past its evidence.
- A value equal to the best is not an improvement and is kept as a `fact`;
  offering the best bound again changes nothing.
- The answer is `{improved, best: {id, statement, status, value},
  recorded: {id, kind, status, created}}`. `best.status` is the best
  bound's **effective** status, as [`ledger_claims`](ledger_claims.md)
  reports it (a `formalised` bound whose file has since changed stands as
  `asserted`, with the reason in `unknowns`); `recorded.status` is what the
  offer earned when it was recorded.
- The best bound is found among the bounds one read of the ledger's window
  recalls, by similarity to `quantity`. When that read fills the window,
  `unknowns` says an older, better bound may have been missed
  ([`../README.md#open-questions`](../README.md#open-questions)).
- `status` and `evidence_origin` are not arguments: like any field the
  schema does not list, they are `INVALID_INPUT`.

## Errors

The errors of [`ledger_record`](ledger_record.md) except `BUDGET_EXCEEDED`.
A Lean timeout is `outcome: "unknown"` and records nothing.
