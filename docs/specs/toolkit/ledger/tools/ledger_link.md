# `ledger_link`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-ledger` | [Ledger](../README.md) | primitive | deferred | write | P5 |

**Given → returns:** two claim ids → a `uses` edge.

Given two claim ids, records that the first uses the second. Refuses a
cycle (`INVALID_INPUT`), including a claim linked to itself, and an unknown
id (`NOT_FOUND`). Linking what is already linked is a no-op
(`created: false`); otherwise the first claim gets a new revision whose
`uses` includes the second.

```json
{ "from": "clm_…", "to": "clm_…" }
```
