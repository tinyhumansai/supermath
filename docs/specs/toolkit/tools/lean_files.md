# `lean_files`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | primitive | deferred | read | P2 |

**Given → returns:** the project, optionally a directory → its Lean files with their last verdicts.

```json
{ "prefix": "Supermath/Sendov/" }
```

Each row: `{path, sha256, lines, last_verdict, checked_sha256}`.
`last_verdict` is the verdict of the last [`lean_check`](lean_check.md) on
the file, and `checked_sha256` the digest it was given for; when the two
digests differ, the verdict is stale and the row says so. Paged, 50 rows at
most.
