# Formalization benchmark

Thirty informal statements, each a Mathlib theorem with a known
formalization, and twelve labelled results from arXiv papers. They were
written for comparing Surplus models on `agent_formalize`, which was removed
with `supermath-subagents` ([ADR 0010](../../docs/adr/0010-run-on-rsi-ralph.md)),
and are kept for a later rsi-ralph meta-loop that tunes the loop template
across problems.
`statements.json` is an array of `{id, area, statement, expected}`:

- `id` is a unique slug; the benchmark writes `lean/Supermath/Bench/<Id>.lean`.
- `statement` is the informal text the tool is given as `{statement}`.
- `expected` is the Mathlib name of the theorem the statement formalizes. The
  benchmark asks only for the statement with a `sorry` proof (`depth:
  "statement"`) and counts a case `checked` when the tool's own `lean_check`
  verdict is `checked` or `checked_with_sorry`. `expected` is for the alignment
  review and for a later proof-depth run.

The set covers arithmetic, number theory, algebra, group theory, linear
algebra, combinatorics, topology, set theory, and real analysis.

`arxiv.json` is an array of `{id, area, arxiv_id, label}`. Each case names a
theorem, lemma, or proposition by its `\label` in a version-pinned paper, so
the tool is given `{arxiv_id, label}` and has to fetch the e-print and find
the statement itself before the child formalizes it. Every label was checked
against `lit_extract_statements` on that version. The cases are self-contained
results (they need no definition from earlier in the paper) in number theory,
analysis, additive combinatorics, group theory, and graph theory, from
Legendre's formula, which Mathlib already has, to spectral and homomorphism
inequalities, which it does not. There is no `expected` name: most have no
Mathlib counterpart.

Nothing in the workspace runs them at the moment. The live runner that did
(`cargo test -p supermath-agent --test live live_subagents_formalize_bench`)
is in the git history before ADR 0010.
