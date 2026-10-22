# `lit_citations`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-literature` | [Literature](../README.md) | primitive | deferred | read | P6 |

**Given → returns:** an arXiv id → references or citing papers.

Given an arXiv id and a direction (`references` or `cited_by`), returns
papers from Semantic Scholar, each with its arXiv id when it has one.
`cited_by` answers "has anyone built on this, or refuted it?".

```json
{ "arxiv_id": "2310.01234", "direction": "cited_by", "limit": 10 }
```

- Rows: `{source, paper_id, arxiv_id, doi, title, year, venue, authors (three),
  author_count, citation_count}`; `arxiv_id` has no version and is `null` when
  Semantic Scholar has none. `limit` defaults to 10 (maximum 50); `cursor` is
  Semantic Scholar's offset.
- `SEMANTIC_SCHOLAR_API_KEY` is sent as `x-api-key` when set; without it the
  shared public rate limit applies.
- `coverage.searched` is `["semantic_scholar"]`. Its graph is incomplete, so
  `unknowns` always says an absent paper may still cite or be cited.
- `next` suggests `lit_get_paper` on the first row that has an arXiv id.

Errors: `INVALID_INPUT`, `NOT_FOUND` (Semantic Scholar does not know the
paper), `RATE_LIMITED`, `TIMEOUT`, `BACKEND_UNAVAILABLE`.
