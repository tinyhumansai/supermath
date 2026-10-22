# `calc_search_counterexample`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | Compute | deferred | read | P5 |

**Given → returns:** a decidable Lean predicate and a range → a witness, or none in range.

Exhaustive search over a finite frame, run by the Lean service so the
predicate is the same object a later proof talks about.

```json
{
  "binder": "n : ℕ",
  "predicate": "¬ (n ^ 2 + n + 41).Prime",
  "range": { "from": 0, "to": 100 }
}
```

- `predicate` is Lean source for a `Decidable` proposition in the bound
  variable; a predicate that is not decidable is `INVALID_INPUT` with
  Lean's error.
- The search is `decide`-checked per value in chunks, under the
  [Lean service](../lean.md#the-lean-service)'s budgets.
- **Found:** `outcome: "ok"` with the first witness, and a `lean_check`ed
  term `⟨w, by decide⟩` proving the existential, so the refutation can be
  recorded as `checked` with `evidence_origin: collected`.
- **Not found:** `outcome: "unknown"` with the frame searched, recorded so
  "nothing found" always says how far it looked. It is never `false`.
- A budget hit mid-range returns `unknown` with the largest value covered.

The frame is the `search_frame` of
[`proofatlas/02-refutation-path.md`](../../../research/math-superagent/proofatlas/02-refutation-path.md).

## Errors

`INVALID_INPUT` for an undecidable or ill-typed predicate or an empty
range; `BUDGET_EXCEEDED` for a range over the admission cap (10^6 values by
default); `BACKEND_UNAVAILABLE` when the Lean service is down.
