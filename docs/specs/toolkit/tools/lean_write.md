# `lean_write`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | primitive | deferred | write | P2 |

**Given → returns:** a module path and its content → the file written, with its digest.

Creates or replaces a whole file in the project's Lean package. Use it for
a new module; use [`lean_edit`](lean_edit.md) to change part of an existing
one, which costs far fewer tokens.

```json
{
  "path": "Supermath/Sendov/Main.lean",
  "content": "import Mathlib\n\nnamespace Sendov\n\ntheorem main … := by\n  sorry\n\nend Sendov\n",
  "if_sha256": null
}
```

- `path` follows the [path rules](../lean.md#files-not-source).
- `if_sha256`, when given, must match the file's current digest, or the
  write is refused with `CONFLICT`; `null` means the file must not exist
  yet. This keeps two turns (or two sub-agents) from overwriting each
  other.
- `content` is at most 1 MB, UTF-8, and may import only `Mathlib` and
  `Supermath.*` modules of this project.
- Returns `{path, sha256, lines}`. It does **not** check the file; call
  [`lean_check`](lean_check.md) next, which `next` suggests.

The project slug comes from the host, never from the arguments. Writes are
atomic (write to a temporary file, then rename), and every write is logged
with the calling tool and turn, so the ledger can say which call produced
the file a verdict was given for.

## Errors

`INVALID_INPUT` for a bad path, non-UTF-8 content, or a forbidden import;
`CONFLICT` when `if_sha256` does not match; `BUDGET_EXCEEDED` over the size
cap.
