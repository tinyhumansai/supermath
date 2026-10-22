# `file_edit`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-files` | [Files](../README.md) | primitive | deferred | write | P1 |

**Given → returns:** a file and exact replacements → the edited file's digest and changed lines.

Changes part of a Lean or Python file in the project's
[workspace](../README.md) by exact string replacement, the way coding agents
edit code. Replacing one `sorry` with a proof, or one function body, is one
edit, not a rewrite of the file.

```json
{
  "path": "lean/Supermath/Sendov/Main.lean",
  "edits": [
    { "old": "theorem main … := by\n  sorry", "new": "theorem main … := by\n  nlinarith [sq_nonneg (a - b)]" }
  ],
  "if_sha256": "<the digest file_read or file_write returned>"
}
```

- Each `old` must occur exactly once in the file; otherwise the call is
  refused with `INVALID_INPUT`, naming the edit and how many times it
  occurred. An empty `old` and an `old` that does not occur get different
  messages. Edits apply in order, all or none.
- `if_sha256` is required: absent or `null` is `INVALID_INPUT`, and an edit
  against a file that changed since the caller last saw it is refused with
  `CONFLICT`.
- Returns `{path, sha256, changed: [{start_line, end_line}]}` (ranges are
  one-based, inclusive, sorted, and adjacent ranges merge), so the caller
  can re-read only what changed. It does not check or run the file. The
  edited file must still pass the [import rules](../README.md#path-rules).

## Errors

`INVALID_INPUT` for a bad path, a path in a generated directory
(`Witnesses/`, `witnesses/`), a missing `if_sha256`, an empty `old`, or an
edit that matches zero or several times; `NOT_FOUND` for a missing file; `CONFLICT` on a stale digest;
`BUDGET_EXCEEDED` if the edited file would pass the size cap;
`BACKEND_UNAVAILABLE` when the workspace disk fails.
