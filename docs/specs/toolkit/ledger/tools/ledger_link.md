# `ledger_link`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-ledger` | [Ledger](../README.md) | primitive | deferred | write | P5 |

**Given → returns:** two claim ids → a `uses` edge.

Given two claim ids, records that the first uses the second. Refuses a
cycle (`INVALID_INPUT`).
