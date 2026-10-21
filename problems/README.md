# Problems

One Markdown file per problem. The file stem is the problem's slug, and the
slug names its memory in CortexDB (`{CORTEX_SCOPE_ROOT}/project:{slug}/app:brain`),
so renaming a file starts that problem with empty memory.

```sh
docker compose run --rm agent solve /problems/sum-of-odd-numbers.md
```

`sum-of-odd-numbers.md` is a smoke test: a statement every capable model
should prove, so a run that does not end `VERDICT: proved` points at the
harness rather than the mathematics.
