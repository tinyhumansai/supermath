# `lean_check`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | direct | read | P2 |

**Given → returns:** Lean source → a [verdict](../lean.md#verdicts) with provenance.

Given Lean source and the names of its target declarations, returns a
verdict.

```json
{
  "source": "theorem two_add_two : 2 + 2 = 4 := by norm_num",
  "targets": ["two_add_two"],
  "depth": "proof"
}
```

- `source` starts after `import Mathlib`; the import is implicit and other
  imports are refused (`INVALID_INPUT`), so every check sees the same
  environment.
- `depth: "statement"` checks only that the statement elaborates: the proof
  is replaced by `sorry`, and the verdict is `checked_with_sorry` at best.
  Formalizing a statement before proving it is the common case.
- `data.errors` lists at most 20 errors as `{line, column, message}`, the
  messages trimmed to 400 characters. `data.sorries` lists each `sorry` with
  its position and goal.
- Budget: 60 s and 200 KB of source by default.
