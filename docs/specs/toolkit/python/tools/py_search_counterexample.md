# `py_search_counterexample`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-python` | [Python](../README.md) | primitive | deferred | read | P4 |

**Given → returns:** a predicate function in a project file and a frame → a witness, or none in frame.

Searches a stated frame for a value where a predicate fails, and records
the frame either way, so "nothing found" always says how far it looked
(the `search_frame` of
[`proofatlas/02-refutation-path.md`](../../../../research/math-superagent/proofatlas/02-refutation-path.md)).
It is the fast, broad search; a witness it finds is then confirmed in Lean
with [`lean_search_counterexample`](../../lean/tools/lean_search_counterexample.md).

```json
{
  "path": "python/work/euler_poly.py",
  "predicate": "is_prime_value",
  "frame": { "kind": "range", "from": 0, "to": 1000000 },
  "timeout_s": 300
}
```

- `predicate` names a function of one argument returning `bool`; the
  search looks for an input where it returns `False`.
- `frame` is one of:
  - `{"kind": "range", "from", "to"}`: integers, in order;
  - `{"kind": "product", "ranges": [...]}`: tuples over a box, in
    lexicographic order;
  - `{"kind": "generator", "entry"}`: values yielded by another function in
    the same file, which must be finite and is reported by count.
- The search runs in the solver environment in chunks, under the same
  limits as [`py_run`](py_run.md).

## Returns

- **Found:** `outcome: "ok"` with the first witness, how many values were
  checked before it, and `next` suggesting the Lean confirmation. The
  witness is also written to a generated file under
  `python/work/witnesses/` so it can be re-run.
- **Not found:** `outcome: "unknown"` with the frame searched in full.
  Recording it with `ledger_record` as a `bound` gives "no counterexample
  for n ≤ 10^6", with this call as its source.
- **Stopped by a limit:** `outcome: "unknown"` with the part of the frame
  covered.

A predicate that raises on some input stops the search and reports that
input and the exception: a crash is not a counterexample.

## Errors

As [`py_run`](py_run.md#errors), plus `INVALID_INPUT` for an empty or
unbounded frame, and `BUDGET_EXCEEDED` for a frame over the admission cap
(10^8 values by default).
