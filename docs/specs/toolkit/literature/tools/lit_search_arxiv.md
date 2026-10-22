# `lit_search_arxiv`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-literature` | [Literature](../README.md) | primitive | direct | read | P3 |

**Given → returns:** a query → paged papers.

Given a query, returns papers.

```json
{ "query": "Sendov conjecture", "category": "math.CV", "since": "2015-01-01", "limit": 10 }
```

- `query` is free text: each word becomes an `all:` clause, ANDed; `title`
  and `author` may be given separately (`ti:`, `au:`). At least one of the
  three is required, else `INVALID_INPUT`.
- `category` restricts to an arXiv category such as `math.NT`; `since`
  (`YYYY-MM-DD`) to papers submitted on or after that date.
- Each row: `{arxiv_id, version, title, authors, primary_category,
  published, abstract}`. `authors` is capped at three plus a count; the
  abstract is trimmed to 600 characters.
- Sorted by relevance unless `sort: "recent"`.
- `limit` defaults to 10 (maximum 50); `cursor` is the offset of the next
  page and is returned while more results remain.
- `next` suggests `lit_get_paper` on the top row. A search with no result
  says so in `unknowns`, naming the sources it did not cover.
- `coverage.searched` is `["arxiv"]`; `not_covered` names zbMATH, MathSciNet,
  journals outside arXiv, and books.

Errors: `INVALID_INPUT`, `RATE_LIMITED`, `TIMEOUT`, `BACKEND_UNAVAILABLE`.
