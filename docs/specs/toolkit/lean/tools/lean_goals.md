# `lean_goals`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-lean` | [Lean](../README.md) | primitive | deferred | read | P5 |

**Given → returns:** a file and a position → the goal state there.

```json
{ "path": "lean/Supermath/Sendov/Main.lean", "line": 52, "column": 4 }
```

`line` is one-based and `column` zero-based, as in `lean_check`'s `errors`
and `sorries`.

Returns the goals at that position in the file as it is on disk:
hypotheses and targets, pretty-printed, at most 8,000 characters. A
position inside a `sorry` gives that `sorry`'s goal. The answer carries the
file's `sha256`. The service may reuse an elaboration of an unchanged file.

## Errors

`INVALID_INPUT` for a position outside the file; `NOT_FOUND` for a missing
file; `BUDGET_EXCEEDED` for a file over the byte budget; `TIMEOUT` and
`BACKEND_UNAVAILABLE` as for `lean_check`.
