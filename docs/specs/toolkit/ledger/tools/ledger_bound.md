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
  recorded: {id, kind, status, created}}`.

## Errors

The errors of [`ledger_record`](ledger_record.md) except `BUDGET_EXCEEDED`.
