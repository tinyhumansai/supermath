# `ledger_record`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-ledger` | [Ledger](../README.md) | primitive | direct | write | P2 |

**Given → returns:** a claim or fact → its id and status.

Given a kind, a statement, and a source, records an entry and returns its
`id` and status. Optional: `uses`, a `lead`'s rationale, a `dead_end`'s
reason. The status is computed, not passed.
