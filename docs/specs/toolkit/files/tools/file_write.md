# `file_write`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-files` | [Files](../README.md) | primitive | deferred | write | P1 |

**Given → returns:** a path and its content → the file written, with its digest.

Creates or replaces a whole Lean or Python file in the project's
[workspace](../README.md). Use it for a new file; use [`file_edit`](file_edit.md) to change part of an existing
one, which costs far fewer tokens.

```json
{
  "path": "lean/Supermath/Sendov/Main.lean",
  "content": "import Mathlib\n\nnamespace Sendov\n\ntheorem main … := by\n  sorry\n\nend Sendov\n",
  "if_sha256": null
}
```

- `path` follows the [path rules](../README.md#path-rules); its first
  segment, `lean/` or `python/`, names the language.
- `if_sha256`, when given, must match the file's current digest, or the
  write is refused with `CONFLICT`; `null` means the file must not exist
  yet. This keeps two turns (or two sub-agents) from overwriting each
  other.
- `content` is at most 1 MB, UTF-8, and may import only what the path
  rules allow for its language.
- Returns `{path, sha256, lines}`. It does **not** check or run the file;
  `next` suggests [`lean_check`](../../lean/tools/lean_check.md) for a Lean
  file and [`py_run`](../../python/tools/py_run.md) for a Python one.

The project slug comes from the host, never from the arguments. Writes are
atomic and [logged](../README.md#writes), so the ledger can say which call
produced the file a result was given for.

## Errors

`INVALID_INPUT` for a bad path, non-UTF-8 content, or a forbidden import;
`CONFLICT` when `if_sha256` does not match; `BUDGET_EXCEEDED` over the size
cap.
