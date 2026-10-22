# `lit_oeis`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-literature` | [Literature](../README.md) | primitive | deferred | read | P3 |

**Given → returns:** terms or a query → matching sequences.

Given either a list of at least four integers or a query, returns matching
OEIS sequences: `{id, name, terms, formula, references}` with terms capped at
30 and formulas at five lines. `coverage` notes that OEIS matches a
subsequence, so a match is a lead, not an identification.

```json
{ "terms": [1, 1, 2, 3, 5, 8] }
```

- Exactly one of `terms` (4 to 60 integers) or `query` (words) is required,
  else `INVALID_INPUT`. `limit` defaults to 10 (maximum 50); `cursor` is the
  offset of the next page.
- A term too large for a JSON integer is returned as a decimal string.
- `unknowns` carries the subsequence caveat for a `terms` search, and says
  that no match is silent about sequences the OEIS does not hold.
- `coverage.searched` is `["oeis"]`.

Errors: `INVALID_INPUT`, `RATE_LIMITED`, `TIMEOUT`, `BACKEND_UNAVAILABLE`.
