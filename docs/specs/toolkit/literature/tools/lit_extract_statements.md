# `lit_extract_statements`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-literature` | [Literature](../README.md) | primitive | deferred | read | P3 |

**Given → returns:** an arXiv id → numbered statements and their `\ref` edges.

Given an arXiv id, returns its numbered statements: every `theorem`,
`lemma`, `proposition`, `corollary`, `conjecture`, `definition`, and
`remark`-like environment the preamble declares with `\newtheorem`.

Each row: `{label, kind, number, section, statement, refs}`. `refs` lists the
labels the statement and its proof `\ref`, which gives a dependency graph
of the paper without reading it. The extraction is a parse, not a model:
statements the parser could not attach to a number are listed in
`unknowns`, not guessed.

This is the input to formalization: a locator `{arxiv_id, label}` names one
statement exactly, and the ledger cites it.
