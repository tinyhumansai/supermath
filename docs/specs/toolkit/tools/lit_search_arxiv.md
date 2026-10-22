# `lit_search_arxiv`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | [Literature](../literature.md) | primitive | direct | read | P3 |

**Given → returns:** a query → paged papers.

Given a query, returns papers.

```json
{ "query": "Sendov conjecture", "category": "math.CV", "since": "2015-01-01", "limit": 10 }
```

- `query` is free text, sent as arXiv's `all:` field; `title` and `author`
  may be given separately.
- `category` restricts to an arXiv category such as `math.NT`.
- Each row: `{arxiv_id, version, title, authors, primary_category,
  published, abstract}`. `authors` is capped at three plus a count; the
  abstract is trimmed to 600 characters.
- Sorted by relevance unless `sort: "recent"`.
- `next` suggests `lit_get_paper` on the top row.
