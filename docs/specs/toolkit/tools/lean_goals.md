# `lean_goals`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | primitive | deferred | read | P4 |

**Given → returns:** a file and a position → the goal state there.

```json
{ "path": "Supermath/Sendov/Main.lean", "line": 52, "column": 4 }
```

Returns the goals at that position in the file as it is on disk:
hypotheses and targets, pretty-printed, at most 8,000 characters. A
position inside a `sorry` gives that `sorry`'s goal. The answer carries the
file's `sha256`, and is computed from the cached elaboration when the file
has not changed since its last [`lean_check`](lean_check.md).

## Errors

`INVALID_INPUT` for a position outside the file; `NOT_FOUND` for a missing
file; `TIMEOUT` as for `lean_check`.
