# math-superagent research

Research notes copied from
[senamakel/math-superagent](https://github.com/senamakel/math-superagent), the
earlier Rust math research agent this project follows on from. The notes study
how working mathematicians, and one public AI-mathematics site, actually reach
results, and ask what a runtime would have to do to work the same way.

## Source

| Repository | Revision | Committed | Copied |
| --- | --- | --- | --- |
| [senamakel/math-superagent](https://github.com/senamakel/math-superagent) | `f0b3505` | 2026-08-31 | 2026-10-22 |

The files are copied verbatim from `research/` at that revision. They have not
been edited, so they still talk about that repository's runtime (its roles,
ledgers, schools, and `src/orchestrator/` modules). Read those references as
history: links that point outside this folder resolve against the source
repository at the pinned revision, not against supermath.

Two files are longer than this repository's 500-line Markdown limit
(`proofatlas/04-known-theorem-bench.md` and
`proofatlas/05-open-workspace-shape.md`). They are kept whole because a copy is
only useful if it matches its source.

## Contents

- **[`tao/`](tao/01-meta-method.md)**: Terence Tao's stated problem-solving
  method, the anatomy of his proofs, his tools, machine-assisted mathematics,
  and conjecture generation. This was the first study and the others descend
  from its schema.
- **[`mathematicians/`](mathematicians/00-conventions.md)**: ten more
  mathematicians studied the same way (Grothendieck, Erdős, Gowers, Wiles,
  Thurston, Zeilberger, Ramanujan, Scholze, Arnold, Perelman), plus a harness
  inventory and the cross-cutting conclusions. Start with
  `00-conventions.md` for the tags.
- **[`proofatlas/`](proofatlas/00-README.md)**: proofatlas.ai read as a
  harness: what a finished proof bundle contains, how disproofs are witnessed,
  how a conjecture is decomposed, and what moves a result between trust grades.

## What supermath takes from it

These notes are inputs, not decisions. The first scaffold of the solving
harness (`crates/supermath-agent`) takes three things from them, recorded in
[`../../specs/solving-harness.md`](../../specs/solving-harness.md):

- a run ends in a three-valued verdict (proved, refuted, not established),
  never a bare yes or no, as both this study and
  [`../rea-and-jacobian/`](../rea-and-jacobian/README.md) recommend;
- memory is scoped per problem and the scope is never built from model
  arguments (`proofatlas/06-trust-model.md` and math-superagent's CortexDB
  notes);
- what a run learns is written back only after the run says what it
  established, so provisional work does not pollute durable recall.
