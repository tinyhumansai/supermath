# `lean_edit`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | primitive | deferred | write | P2 |

**Given → returns:** a file and exact replacements → the edited file's digest and changed lines.

Changes part of a file in the project's Lean package by exact string
replacement, the way coding agents edit code. Replacing one `sorry` with a
proof is one edit, not a rewrite of the file.

```json
{
  "path": "Supermath/Sendov/Main.lean",
  "edits": [
    { "old": "theorem main … := by\n  sorry", "new": "theorem main … := by\n  nlinarith [sq_nonneg (a - b)]" }
  ],
  "if_sha256": "<the digest lean_read or lean_write returned>"
}
```

- Each `old` must occur exactly once in the file; otherwise the call is
  refused with `INVALID_INPUT`, naming the edit and how many times it
  occurred. Edits apply in order, all or none.
- `if_sha256` is required: an edit against a file that changed since the
  caller last saw it is refused with `CONFLICT`.
- Returns `{path, sha256, changed: [{start_line, end_line}]}`, so the caller
  can re-read only what changed. It does not check the file.

## Errors

`INVALID_INPUT` for a bad path or an edit that matches zero or several
times; `NOT_FOUND` for a missing file; `CONFLICT` on a stale digest.
