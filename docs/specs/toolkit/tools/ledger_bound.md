# `ledger_bound`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Ledger](../ledger.md) | deferred | write | P5 |

**Given → returns:** a quantity and a bound → the best bound so far.

Given a named quantity, a direction (`upper` or `lower`), a value, and a
source, records it if it improves on the best bound so far, and returns the
best bound either way. Bounds only improve: a worse bound is kept as a
`fact`, not as the bound.
