# `calc_search_counterexample`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | Compute | primitive | deferred | read | P5 |

**Given → returns:** a decidable predicate declared in a project file, and a range → a witness, or none in range.

Exhaustive search over a finite frame, run by the Lean service so the
predicate is the same object a later proof talks about. Like every Lean
tool it takes a file, not source: the predicate is declared in the project
package with [`lean_write`](lean_write.md) or [`lean_edit`](lean_edit.md)
first.

```json
{
  "path": "Supermath/Euler/Search.lean",
  "predicate": "Euler.notPrime",
  "range": { "from": 0, "to": 100 }
}
```

- `predicate` names a declaration of type `ℕ → Prop` (or `ℤ → Prop`) with a
  `DecidablePred` instance in that file; anything else is `INVALID_INPUT`
  with Lean's error.
- The search is `decide`-checked per value in chunks, under the
  [Lean service](../lean.md#the-lean-service)'s budgets.
- **Found:** `outcome: "ok"` with the first witness, and the tool appends
  `theorem <predicate>_witness : ∃ n, <predicate> n := ⟨w, by decide⟩` to a
  generated file `Supermath/Witnesses/<Name>.lean` and checks it, so the
  refutation can be recorded as `checked` with `evidence_origin:
  collected`.
- **Not found:** `outcome: "unknown"` with the frame searched, recorded so
  "nothing found" always says how far it looked. It is never `false`.
- A budget hit mid-range returns `unknown` with the largest value covered.

The frame is the `search_frame` of
[`proofatlas/02-refutation-path.md`](../../../research/math-superagent/proofatlas/02-refutation-path.md).

## Errors

`INVALID_INPUT` for a predicate that is not a decidable predicate on
integers, or an empty range; `NOT_FOUND` for a missing file or
declaration; `BUDGET_EXCEEDED` for a range over the admission cap (10^6
values by default); `BACKEND_UNAVAILABLE` when the Lean service is down.
