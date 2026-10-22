# `lean_search_counterexample`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-lean` | [Lean](../README.md) | primitive | deferred | read | P6 |

**Given → returns:** a decidable predicate declared in a project file, and a range → a witness, or none in range.

Exhaustive search over a finite frame, run by the Lean service so the
predicate is the same object a later proof talks about. Like every Lean
tool it takes a file, not source: the predicate is declared in the project
package with [`file_write`](../../files/tools/file_write.md) or [`file_edit`](../../files/tools/file_edit.md)
first.

```json
{
  "path": "lean/Supermath/Euler/Search.lean",
  "predicate": "Euler.notPrime",
  "range": { "from": 0, "to": 100 }
}
```

The range is inclusive at both ends.

- `predicate` names a declaration of type `ℕ → Prop` (or `ℤ → Prop`) with a
  `DecidablePred` instance in that file; anything else is `INVALID_INPUT`
  with Lean's error.
- The search is `decide`-checked per value in chunks, under the
  [Lean service](../README.md#the-lean-service)'s budgets.
- **Found:** `outcome: "ok"` with the first witness, and the tool writes
  `theorem <predicate>_witness : ∃ n, <predicate> n := ⟨w, by decide⟩` as
  the whole of a generated file `lean/Supermath/Witnesses/<Name>.lean`
  (`<Name>` is the predicate's dotted name with each segment capitalised and
  joined, so `Euler.notPrime` is `EulerNotPrime`; the file imports the
  predicate's module) and checks it, so the refutation can be recorded as
  `checked` with `evidence_origin: collected`. There is one file per
  predicate: an identical file is left alone and a different one (an earlier
  witness) is replaced. Nothing is lost by replacing it, because the
  theorem's statement does not depend on the witness — appending would only
  redeclare the same name. `data` carries the witness, the frame, the
  witness file, and the check's verdict, provenance, and errors.
- The witness stands on the service's search, so it is reported even when
  the theorem recording it is not evidence; each case adds a note to
  `unknowns` and says the theorem is not evidence yet:
  - the theorem checks to anything but `checked` (the note names the
    verdict);
  - the check could not run — the service is down, does not find the file,
    or answers an error: `verdict: "unknown"`, `provenance: null`;
  - the file could not be written: `witness_file: null`,
    `verdict: "unknown"`, `provenance: null`, and no check is attempted.
- **Exhausted:** the whole range was decided and no value satisfies the
  predicate: `outcome: "ok"` with `data.found: false`,
  `data.result: "exhausted"`, `data.frame` (`covered_to` equals `to`), and
  `coverage` naming the range. The answer is complete for that range and
  says nothing outside it; a ledger `tool` source with the frame earns
  `checked` on it.
- A budget hit mid-range — the service reports `timeout`, `memory`, or
  `output_limit` — returns `unknown` (`data.result: "limit"`) with the
  largest value covered (`data.frame.covered_to`, `null` when none was), and
  a note naming what ran out. A service that does not answer in time is also
  `unknown`, with no `covered_to`. `unknown` is only for a search cut short,
  never a negative answer.
- `data.frame.domain` is `nat` or `int`, as the service decided by
  elaborating the predicate; it is `null` when the search stopped before the
  service reported one (a client-side timeout, or a limit hit while
  elaborating).

The frame is the `search_frame` of
[`proofatlas/02-refutation-path.md`](../../../../research/math-superagent/proofatlas/02-refutation-path.md).

## Errors

`INVALID_INPUT` for a predicate that is not a decidable predicate on
integers, or an empty range; `NOT_FOUND` for a missing file or
declaration; `BUDGET_EXCEEDED` for a range over the admission cap (10^6
values by default); `BACKEND_UNAVAILABLE` when the Lean service is down.
