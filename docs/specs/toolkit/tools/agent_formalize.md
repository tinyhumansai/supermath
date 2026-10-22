# `agent_formalize`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-agent` | [Sub-agents](../agents.md) | agentic (OpenHuman) | deferred | read | P4 |

**Given → returns:** a statement or paper locator → Lean files and their verdicts.

Runs an OpenHuman agent turn that writes a statement, and optionally its
proof, as Lean files in the project's package. The caller gets paths and
verdicts the tool computed itself, not the child's account.

```json
{
  "source": { "arxiv_id": "2012.04125", "label": "thm:main" },
  "depth": "statement",
  "path": "Supermath/Sendov/Main.lean"
}
```

- `source` is `{arxiv_id, label}`, `{statement}` (informal text), or
  `{claim_id}`.
- `depth: "statement"` produces the theorem statement with a `sorry`
  proof, plus any definitions it needs. `depth: "proof"` also attempts the
  proof.
- `path` is the file to create or extend, under the
  [path rules](../lean.md#files-not-source). The child may add helper
  modules under the same directory.

## Turn

| | |
| --- | --- |
| Model | `SUPERMATH_FORMALIZER_MODEL` (default rung `formalize`) |
| Tools in the turn | `lit_get_paper`, `lean_search`, `lean_decl`, `lean_files`, `lean_read`, `lean_write`, `lean_edit`, `lean_check`, `lean_goals`, `lean_try_tactics`, `recall_memory` |
| Budget | 24 iterations, 200,000 tokens, 600 s by default; `depth: "proof"` raises iterations to 48 |
| Prompt | find the Mathlib names first; reuse Mathlib definitions rather than redefining them; write the statement with a `sorry` proof and check it; then work each `sorry` with `lean_goals`, `lean_try_tactics`, and `lean_edit` |

If the statement cannot be written from Mathlib primitives, the child says
which definition is missing and stops: that is a `formalisation_depth`
finding, not a failure
([`proofatlas/02-refutation-path.md`](../../../research/math-superagent/proofatlas/02-refutation-path.md)).

The child does not get `ledger_record`. Recording is the tool's job, from
what it verified.

## Returns

- **Verified after the turn:** the tool lists the files the child wrote
  (from the write log, not the child's reply), runs
  [`lean_check`](lean_check.md) on each, and returns `{path, sha256,
  declarations, verdict}` per file. Each target declaration is recorded in
  the ledger as a `result` with that verdict, its `lean` source
  `{path, declaration, file_sha256}`, alignment `not_reviewed`, and a
  `uses` edge to the fact it formalizes.
- **Passed through, as untrusted prose:** the child's note on what it could
  not do, such as a missing definition, at most 80 words.
- **Discarded:** the child's own claims about verdicts.
- `usage` with the turn's model, tool calls, tokens, and time.

## Errors

`INVALID_INPUT` for a bad path or source; `NOT_FOUND` for an unknown paper,
label, or claim (checked before the turn); `CONFLICT` when `path` changed
during the turn under another writer. A turn that runs out of budget is
`outcome: "unknown"`, with the files it wrote and their verdicts.
