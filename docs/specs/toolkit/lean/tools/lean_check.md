# `lean_check`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-lean` | [Lean](../README.md) | primitive | direct | read | P2 |

**Given → returns:** a Lean file in the project → a [verdict](../README.md#verdicts) with provenance.

Checks a file in the project's Lean package as it stands on disk. It never
takes Lean source as an argument: source gets into the package through
[`file_write`](../../files/tools/file_write.md) and [`file_edit`](../../files/tools/file_edit.md), so every
verdict names a file and a digest that anyone can re-check
([`lean/`](../README.md#files-not-source)).

```json
{
  "path": "lean/Supermath/Sendov/Main.lean",
  "targets": ["Sendov.main"]
}
```

- `path` is a module file in the package
  ([path rules](../README.md#files-not-source)). A path that does not exist is
  `NOT_FOUND`.
- `targets` names the declarations whose axioms are reported and whose
  status the verdict speaks for. Omitted, every theorem and definition the
  file declares is a target.
- The file is elaborated in the package's environment, so it may import
  Mathlib and other `Supermath.*` modules of the same project; those are
  built first if their digests changed.
- `data.errors` lists at most 20 errors as `{line, column, message}`, the
  messages trimmed to 400 characters. `data.sorries` lists each `sorry` with
  its position and goal, so the next call can be
  [`lean_goals`](lean_goals.md) or [`lean_try_tactics`](lean_try_tactics.md)
  on it.
- The verdict is cached by the file's digest and the digests of its
  imports: checking an unchanged file returns the cached verdict, marked as
  such in `coverage`.
- Budget: 60 s per file by default, and 1 MB of source.

A statement with a `sorry` proof is the normal first step of formalizing,
and its verdict is `checked_with_sorry`.

## Errors

`INVALID_INPUT` for a path outside the package rules; `NOT_FOUND` for a
missing file or target; `TIMEOUT`, with `outcome: "unknown"`;
`BACKEND_UNAVAILABLE` when the Lean service is down.
