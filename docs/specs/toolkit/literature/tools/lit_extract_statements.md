# `lit_extract_statements`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-literature` | [Literature](../README.md) | primitive | deferred | read | P3 |

**Given → returns:** an arXiv id → numbered statements and their `\ref` edges.

Given an arXiv id, returns its numbered statements: every `theorem`,
`lemma`, `proposition`, `corollary`, `conjecture`, `definition`, and
`remark`-like environment the preamble declares with `\newtheorem`.

Each row: `{label, kind, number, section, statement, refs}`. `kind` is the
environment's title lowercased (`theorem`, `lemma`). `label` is the first
`\label` in the statement (`null` if unlabeled); `number` is what LaTeX
prints (`2.3`, `A.1`), or `null` for a starred (`\newtheorem*`) environment.
`refs` lists the labels the statement and its proof `\ref`, `\eqref`,
`\cref`, `\Cref`, `\autoref` or `\vref`, which gives a dependency graph
of the paper without reading it. The proof is the `proof` environment that
directly follows; one titled `[Proof of \ref{x}]` for another label is not
attached. The extraction is a parse, not a model:
statements the parser could not attach to a number are listed in
`unknowns`, not guessed.

Numbering follows `\newtheorem{env}{Title}`, `\newtheorem{env}[shared]{Title}`
(a shared counter, followed through chains), `\newtheorem{env}{Title}[section]`
and `[subsection]`, and the starred form. Counters reset per section,
`\appendix` switches to letters. Listed in `unknowns` instead of numbered:
a `\begin{theorem}` (or lemma, proposition, corollary, conjecture,
definition, remark) with no `\newtheorem` in the source, a counter the paper
renumbers with `\renewcommand{\the<counter>}` or `\def`, a `within` other
than section or subsection, a counter shared with a non-theorem counter, and
an environment never closed. `limit` (default 10, maximum 50) and `cursor`
page the rows; `data.total` counts them all, and `data.locator` is
`{arxiv_id, version}`. A PDF-only paper is `unknown` with no rows.

Errors: `INVALID_INPUT`, `NOT_FOUND`, `RATE_LIMITED`, `TIMEOUT`,
`BACKEND_UNAVAILABLE`.

This is the input to formalization: a locator `{arxiv_id, label}` names one
statement exactly, and the ledger cites it.
