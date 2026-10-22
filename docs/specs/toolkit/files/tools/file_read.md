# `file_read`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-files` | [Files](../README.md) | primitive | deferred | read | P1 |

**Given → returns:** a file and a line range → its numbered lines and digest, or its outline.

```json
{ "path": "lean/Supermath/Sendov/Main.lean", "start_line": 40, "end_line": 80 }
```

- Returns `text` (one `N: line` per line), at most 400 lines per call, with
  a `cursor` for the rest (pass it back as `cursor`), and the file's `sha256`
  for a following [`file_edit`](file_edit.md). `start_line` defaults to 1 and
  `end_line` to the end of the file.
- With no range, returns the file's outline instead of its text: for Lean,
  each declaration with its kind, line range, and whether its proof
  contains a `sorry`; for Python, each top-level function and class with
  its signature and line range. Most turns need the outline, not the file.

## Errors

`INVALID_INPUT` for a bad path, range, or cursor; `NOT_FOUND` for a missing
file; `BUDGET_EXCEEDED` if the reply (lines or outline) would pass the
call's byte budget, or the file is over the 1 MB cap;
`BACKEND_UNAVAILABLE` when the workspace disk fails.
