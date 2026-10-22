# `ledger_claims`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-ledger` | [Ledger](../README.md) | primitive | deferred | read | P2 |

**Given → returns:** filters → paged claims.

Given filters (`kind`, minimum `status`, `alignment`, or a text query),
returns entries, newest first, as `{id, kind, status, alignment, statement}`
with statements trimmed to 300 characters. `ledger_claims {"id": …}` returns
one entry in full, with its effective status and the chain of claims that
limit it.
