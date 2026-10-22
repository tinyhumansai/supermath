# `file_list`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-files` | [Files](../README.md) | primitive | deferred | read | P1 |

**Given → returns:** the project, optionally a directory → its files with their last results.

```json
{ "prefix": "lean/Supermath/Sendov/" }
```

Each row: `{path, language, sha256, lines, last_result, result_sha256}`.
`last_result` is the last [`lean_check`](../../lean/tools/lean_check.md)
verdict for a Lean file, or the last [`py_run`](../../python/tools/py_run.md)
outcome for a Python file, reported through each domain's
[observer](../README.md#observers); `result_sha256` is the digest it was
for. When the digests differ, the result is stale and the row says so.
Paged, 50 rows at most.
