# `py_run`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-python` | [Python](../README.md) | primitive | direct | read | P4 |

**Given → returns:** a Python file, an entry point, and arguments → its returned value, output, and provenance.

Runs one function from a file in the project's `python/work/` package, in
the sandboxed solver environment ([`python/`](../README.md)). It never
takes Python source: the file is written first with
[`file_write`](../../files/tools/file_write.md) or
[`file_edit`](../../files/tools/file_edit.md).

```json
{
  "path": "python/work/sunflower_search.py",
  "entry": "largest_free_family",
  "args": { "n": 6, "k": 3 },
  "seed": 0,
  "timeout_s": 120
}
```

- `entry` names a top-level function in the file.
- `args` is passed as keyword arguments; it is a JSON object (default `{}`), at most 64 KB. Arguments that do not fit the function's signature are `INVALID_INPUT`.
- `seed` (default 0) is passed to the function as `seed` when it declares
  that parameter, and seeds `random` and `numpy` before the call.
- `timeout_s` defaults to `SUPERMATH_PY_DEFAULT_TIMEOUT` (60), shortened to
  the call's wall-time budget when that is tighter, and is capped at
  `SUPERMATH_PY_MAX_TIMEOUT` (900). Asking for more than the cap, or more
  than the call's budget, is `BUDGET_EXCEEDED`.

## Returns

```json
{
  "outcome": "ok",
  "data": {
    "result": "returned",
    "value": { "size": 20, "family": ["…"] },
    "exact": true,
    "stdout": "searched 1,048,576 families\n",
    "stdout_truncated": false,
    "stderr": "",
    "stderr_truncated": false,
    "provenance": { "path": "…", "file_sha256": "…", "imports_sha256": "…", "entry": "largest_free_family", "args_sha256": "…", "seed": 0, "python": "3.12.15", "lock_sha256": "…", "elapsed_ms": 4210, "peak_rss_mb": 312 }
  },
  "next": ["ledger_record {\"kind\": \"bound\", …, \"source\": {\"tool\": {\"tool\": \"py_run\", \"call_id\": \"…\"}}}"]
}
```

- `result` is `returned` or `raised`. `raised` is `outcome: "ok"` too: the
  run happened, and the exception is the answer
  (`data.exception: {type, message, traceback}`).
- A run that exceeded a limit is `outcome: "unknown"`, with `data.limit`
  (`time`, `cpu`, `memory`, `output`, or `crashed`) and the output so far.
- A return value that is not JSON (a set, a non-finite float, a non-string
  key) is a `raised` result of type `NotJsonSerializable`.
- A value holding floating-point numbers that does not declare
  `"exact": true` carries an `unknowns` note: it cannot back an exact claim
  as `checked`.
- `exact` is what the function declared about its arithmetic
  ([status rules](../README.md#status-rules)); absent means inexact.
- The call is recorded in this process's call log under its `call_id`, so
  `ledger_record` can cite it as a `tool` source.

## Errors

| Code | When |
| --- | --- |
| `INVALID_INPUT` | a path outside `python/work/`, no such entry point, arguments that are not a JSON object |
| `NOT_FOUND` | the file does not exist |
| `BUDGET_EXCEEDED` | a timeout over the cap, or the project's compute budget is spent |
| `BACKEND_UNAVAILABLE` | `PYTHON_URL` is unset, the Python service is down, answered something unexpected, or ran a different version of the file than the workspace holds |
