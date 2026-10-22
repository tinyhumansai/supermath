# `lean_blueprint`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-lean` | [Lean](../README.md) | primitive | deferred | read | P6 |

**Given → returns:** the project → the declaration DAG with status.

```json
{ "root": "Sendov.main", "limit": 50 }
```

Given the project, returns the dependency graph of its declarations: nodes
with their status (`checked`, `checked_with_sorry`, `failed`, `unknown`),
ledger claim id, `uses` (edges to other project declarations, from the
constants each declaration's type and value mention), and `rests_on_sorry`
(the `checked_with_sorry` declarations reachable from it). `root` restricts
the graph to that declaration and what it rests on. The claim id comes from
the `ClaimIndex` port the composition supplies; without one it is `null`.
Files that could not be checked are listed in `unknowns`, one note each: a
file with no verdict (`<path> has no verdict; its nodes may be missing or
marked unknown`) and a file the workspace lists but cannot read, over the
size cap or not UTF-8 (`<path> could not be read (<why>); its declarations
are missing`).

Paged by node, ordered by path and line: `limit` defaults to 10 (maximum
50, and never more than the call's `rows` budget), and `data.total` counts
every node. The `cursor` is the decimal offset of the next node; a numeric
cursor at or past `total` returns an `ok` page with no nodes and no cursor,
and anything else is `INVALID_INPUT`.

The answer to "what does the main theorem still rest on" is the
set of `checked_with_sorry` nodes reachable from it. Formalization effort
goes first to the node most other nodes rest on (Scholze's rule,
[`strategies/04-trust.md`](../../../../research/strategies/04-trust.md)).

## Errors

`INVALID_INPUT` for a bad `root` name or `cursor`; `NOT_FOUND` when `root` is
not declared in the project; `BACKEND_UNAVAILABLE` when the Lean service is
down.
