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
  "seed": 0,
  "timeout_s": 300
}
```

- `predicate` names a function of one argument returning `bool`; the
  search looks for an input where it returns `False`.
- `frame` is one of:
  - `{"kind": "range", "from", "to"}`: the integers `from..=to`, in order
    (both ends inclusive, so the frame above supports "n <= 10^6");
  - `{"kind": "product", "ranges": [{"from", "to"}, ...]}`: tuples over a
    box of inclusive ranges (1 to 8 of them), in lexicographic order, each
    passed to the predicate as one tuple;
  - `{"kind": "generator", "entry"}`: values yielded by another function in
    the same file, which must be finite and is reported by count.
- `seed` (default 0) seeds `random` and `numpy`; `timeout_s` is as in
  [`py_run`](py_run.md).
- The search runs in the solver environment under the same limits as
  [`py_run`](py_run.md), and records its progress as it goes, so a limit
  reports how many values were covered.

## Returns

- **Found:** `outcome: "ok"` with the first witness, how many values were
  checked before it, and `next` suggesting the Lean confirmation. The
  witness is also written to a generated file
  `python/work/witnesses/witness_<predicate>_<12 hex>.py` (named by its
  content, so a repeat writes nothing new) whose `check()` re-runs the
  predicate on the witness, so it can be re-run with `py_run`. If the file
  cannot be written, the witness is still returned, with an `unknowns` note.
- **Exhausted:** the whole stated frame was searched and held no witness:
  `outcome: "ok"` with `data.result: "exhausted"`, `data.checked`, the frame
  in `data.frame` and `data.frame_description`, and `coverage` naming it.
  The answer is complete for that frame and says nothing outside it. `next`
  suggests a `ledger_record` of kind `bound` with this call as a `tool`
  source and the frame, which the ledger accepts (the call ended `ok`) and
  earns `checked` on that frame, never `formalised`.
- **Stopped by a limit:** `outcome: "unknown"` (a search cut short is never a
  negative answer) with `data.checked` and, for a
  range, `data.covered: {from, to}`. A generator that reaches the admission
  cap is `capped`, also unknown, with the count covered.

A predicate that raises on some input, or returns something other than a
bool, stops the search and reports that input and the exception
(`outcome: "unknown"`, `data.result: "predicate_raised"`): a crash is not a
counterexample.

## Errors

As [`py_run`](py_run.md#errors), plus `INVALID_INPUT` for an empty or
unbounded frame, and `BUDGET_EXCEEDED` for a frame over the admission cap
(10^8 values by default, `SUPERMATH_PY_MAX_FRAME`; checked up front for
ranges and products, by the service for generators).
