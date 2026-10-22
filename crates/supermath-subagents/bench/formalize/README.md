# Formalization benchmark

Thirty informal statements, each a Mathlib theorem with a known
formalization, for comparing rungs of the model ladder on `agent_formalize`.
`statements.json` is an array of `{id, area, statement, expected}`:

- `id` is a unique slug; the benchmark writes `lean/Supermath/Bench/<Id>.lean`.
- `statement` is the informal text the tool is given as `{statement}`.
- `expected` is the Mathlib name of the theorem the statement formalizes. The
  benchmark asks only for the statement with a `sorry` proof (`depth:
  "statement"`) and counts a case `checked` when the tool's own `lean_check`
  verdict is `checked` or `checked_with_sorry`. `expected` is for the alignment
  review and for a later proof-depth run.

The set covers arithmetic, number theory, algebra, group theory, linear
algebra, combinatorics, topology, set theory, and real analysis. It does not
yet hold statements from arXiv: those need labels checked against real
e-prints, and are added as `{arxiv_id, label}` cases once the literature
backend can resolve them.

Run it against a real stack with
`cargo test -p supermath-agent --test live live_subagents_formalize_bench`
(see the variables in that test's header); without them it is skipped.
