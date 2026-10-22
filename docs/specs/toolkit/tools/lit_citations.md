# `lit_citations`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | [Literature](../literature.md) | primitive | deferred | read | P5 |

**Given → returns:** an arXiv id → references or citing papers.

Given an arXiv id and a direction (`references` or `cited_by`), returns
papers from Semantic Scholar, each with its arXiv id when it has one.
`cited_by` answers "has anyone built on this, or refuted it?".
