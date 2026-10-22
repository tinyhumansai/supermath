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
  one after another in one REPL (proof states are per process), each under
  its own timeout (10 s by default).
- **It does not change the file.** A tactic that closes the goal comes with
  a ready [`file_edit`](../../files/tools/file_edit.md) call in `next`, replacing the `sorry`
  with a tactic: `exact <term>` when `exact?` found a term, otherwise the
  tactic as tried. (The `sorry` sits in a `by` block, so a bare term would
  not elaborate.) Only an `exact` suggestion yields a `term`; another
  tactic's `Try this:` suggestion, such as `simp?`'s, is not reported.

## Errors

`INVALID_INPUT` when there is no `sorry` at that position (checked against
the last `lean_check` of the same file digest when there is one, otherwise by
the service) or the tactic list is empty, over 12, or has a blank or
500-character-plus entry; `NOT_FOUND` for a missing file; `CONFLICT` when
`if_sha256` is passed and is not the file's current digest;
`BUDGET_EXCEEDED` for a file over the byte budget; `TIMEOUT` and
`BACKEND_UNAVAILABLE` as for `lean_check`.

When a closing tactic is found, `next` holds the `file_edit` call. It is
omitted, with a note in `unknowns` naming the reason, when the `sorry`'s line
is past the end of the file, when that line holds no `sorry`, or when the
line is not unique in the file (an edit's `old` must occur exactly once).
