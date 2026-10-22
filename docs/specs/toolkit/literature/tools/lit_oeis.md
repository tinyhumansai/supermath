# `lit_oeis`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-literature` | [Literature](../README.md) | primitive | deferred | read | P3 |

**Given → returns:** terms or a query → matching sequences.

Given either a list of at least four integers or a query, returns matching
OEIS sequences: `{id, name, terms, formula, references}` with terms capped at
30 and formulas at five lines. `coverage` notes that OEIS matches a
subsequence, so a match is a lead, not an identification.
