# `file_list`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-files` | [Files](../README.md) | primitive | deferred | read | P1 |

**Given → returns:** the project, optionally a directory → its files with their last results.

```json
{ "prefix": "lean/Supermath/Sendov/" }
```

Each row: `{path, language, sha256, lines, last_result, result_sha256,
stale}`; `last_result`, `result_sha256`, and `stale` are `null` when no
observer has a result for the file.
`last_result` is the last [`lean_check`](../../lean/tools/lean_check.md)
verdict for a Lean file, or the last [`py_run`](../../python/tools/py_run.md)
outcome for a Python file, reported through each domain's
[observer](../README.md#observers); `result_sha256` is the digest it was
for. When the digests differ, the result is stale and the row says so
(`stale: true`). Paged with `limit` (default and maximum 50, overriding
the [envelope](../../README.md#one-envelope)'s general default of 10; a
larger `limit` is clamped to 50) and `cursor`; the reply also carries
`total`, the number of files under the prefix.

- `cursor` is the row offset of the next page, as a decimal string (`"50"`);
  a reply carries one only while rows remain. A cursor past the end is an
  empty `ok` page.
- A file that was listed but cannot be read as a source file (it vanished,
  grew past the 1 MB cap, or is no longer UTF-8) has no row, but `total`
  still counts it, so each one is named in `unknowns`
  (`"<path> was skipped: <why>"`).

## Errors

`INVALID_INPUT` for a bad prefix, or a cursor that is not a non-negative
row offset (`"x"`, `"-1"`, `""`); `BACKEND_UNAVAILABLE` when the workspace
disk fails, including a listed file the disk refuses to read.
