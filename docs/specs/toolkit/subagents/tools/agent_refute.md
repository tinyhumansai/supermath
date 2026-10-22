# `agent_refute`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-subagents` | [Sub-agents](../README.md) | agentic (OpenHuman) | deferred | read | P6 |

**Given → returns:** a statement → a counterexample, or a repaired statement.

Runs an OpenHuman agent turn that looks for a counterexample before anyone
tries to prove the statement.

```json
{ "claim_id": "clm_7f3a…", "budget": { "wall_secs": 900 } }
```

## Turn

| | |
| --- | --- |
| Model | `SUPERMATH_FORMALIZER_MODEL` |
| Tools in the turn | `file_write`, `file_edit`, `file_read`, `py_solvers`, `py_run`, `py_search_counterexample`, `lean_check`, `lean_search_counterexample`, `lit_oeis` |
| Budget | 24 iterations, 200,000 tokens, 600 s by default |
| Prompt | state the search frame first; search broadly in Python (exhaustive small cases, then SAT, CP-SAT, or numeric search), then confirm any witness in Lean as a decidable predicate |

## Returns

- **Verified after the turn:** a witness found in Python is only a lead,
  recorded as a `lead` (source `supplied`, naming the call) and never as a
  refutation. A witness counts only if a
  [`lean_search_counterexample`](../../lean/tools/lean_search_counterexample.md)
  call the child made answered `ok` with `data.found: true`, a `witness`, the
  generated `witness_file` (a path), and its `theorem`, and that file
  [`lean_check`](../../lean/tools/lean_check.md)s as `checked` for that
  theorem, re-run by the tool (`{path, targets: [theorem]}`; the search's own
  `data.verdict` is not trusted). It is recorded as a `refutation` with
  `refutes` set and the `lean` source
  `{path: witness_file, declaration: theorem, file_sha256}`, the digest
  `lean_check` reported in `data.provenance.file_sha256`; the ledger runs
  its own check and earns the status.
- **Not found:** `outcome: "unknown"` with every frame searched to the end,
  by `py_search_counterexample` (`outcome: "ok"`, `data.result:
  "exhausted"`, the frame in `data.frame_description`) or in Lean
  (`outcome: "ok"`, `data.result: "exhausted"`, `data.found: false` and
  `data.frame` `{predicate, from, to, covered_to, domain}`, recorded as
  `decide <predicate> over ℕ|ℤ in <from>..=<covered_to>`), recorded as a
  `bound` with its `frame`. The bound's source is the `tool` source naming
  the search tool and the call id the belt logged: the call ended `ok`, so
  the ledger earns `checked` on that frame only (never `formalised`). A
  search cut short (a limit, a timeout, a crash in the predicate) ends
  `unknown`, states no finished frame, and is not recorded.
- **Passed through, as a `lead`:** when a counterexample is confirmed, the
  child's proposed nearest repaired statement (its `REPAIRED:` line, at most
  80 words, recorded as a `lead` and returned as untrusted prose)
  ([`strategies/03-failure.md`](../../../../research/strategies/03-failure.md)).
- `usage` with the turn's model, tool calls, tokens, and time.

## Errors

`NOT_FOUND` for an unknown claim; `BUDGET_EXCEEDED` when the project's
budget cannot admit a turn.
