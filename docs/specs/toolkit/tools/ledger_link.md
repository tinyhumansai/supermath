# `ledger_link`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Ledger](../ledger.md) | deferred | write | P4 |

**Given → returns:** two claim ids → a `uses` edge.

Given two claim ids, records that the first uses the second. Refuses a
cycle (`INVALID_INPUT`).
