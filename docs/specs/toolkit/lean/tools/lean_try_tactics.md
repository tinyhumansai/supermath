# `lean_try_tactics`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-lean` | [Lean](../README.md) | primitive | deferred | read | P5 |

**Given → returns:** a file, one of its `sorry`s, and tactics → per-tactic outcome.

```json
{
  "path": "lean/Supermath/Sendov/Main.lean",
  "sorry": { "line": 53, "column": 2 },
  "tactics": ["nlinarith [sq_nonneg (a - b)]", "positivity", "exact?"]
}
```

- `sorry` is the position of one `sorry` in the file, as
  [`lean_check`](lean_check.md) listed it.
- Up to 12 tactics, each reported as `closes`, `progress` (with the
  remaining goals), or `fails` (with the error). The default list, when
  none is given, is `rfl`, `norm_num`, `simp`, `omega`, `decide`,
  `linarith`, `nlinarith`, `positivity`, `aesop`, and `exact?`. A tactic
  that closes the goal is returned with the term `exact?` found, when it
  found one.
- Tactics run in the REPL's tactic mode against that `sorry`'s proof state,
  in parallel, each under its own timeout (10 s by default).
- **It does not change the file.** A tactic that closes the goal comes with
  a ready [`file_edit`](../../files/tools/file_edit.md) call in `next`, replacing the `sorry`
  with it.

## Errors

`INVALID_INPUT` when there is no `sorry` at that position; `NOT_FOUND` for
a missing file; `CONFLICT` when the file changed since the position was
reported (`if_sha256` may be passed to make that explicit).
