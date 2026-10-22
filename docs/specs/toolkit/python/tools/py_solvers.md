# `py_solvers`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-python` | [Python](../README.md) | primitive | deferred | read | P4 |

**Given → returns:** nothing, or an area → the solvers in the pinned environment, with versions.

Answers "what can I import?" before the agent writes a file that imports
something missing.

```json
{ "area": "sat" }
```

- `area` is one of `symbolic`, `numeric`, `sat`, `smt`, `optimization`,
  `graphs`, or omitted for all.
- Each row: `{package, version, import_name, area, summary}`, where
  `summary` is one line on what the package is for, kept beside the lock
  in `docker/python/solvers.toml` and checked in a test against the lock.
- `coverage.searched` names the lock digest the list was read from, and
  `data.lock_sha256` repeats it. An area with no solver is `ok` with an
  `unknowns` note.
- The tool reads the service's `GET /solvers`, which serves the rows of
  `solvers.toml` whose package is in the lock, with the locked versions.

The list comes from the service's lock, never from a hand-kept table in
the crate, so it cannot drift from what a run can import.

## Errors

`INVALID_INPUT` for an unknown area; `BACKEND_UNAVAILABLE` when the Python
service is down.
