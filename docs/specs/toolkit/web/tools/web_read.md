# `web_read`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-web` | [Web](../README.md) | primitive | deferred | read | P3 |

**Given → returns:** a URL → the page's main text, paged.

Reads a source that [`web_search`](web_search.md) or
[`web_research`](web_research.md) surfaced, so the agent can check what it
actually says. It is a plain HTTP fetch; no model reads the page.

```json
{ "url": "https://mathoverflow.net/questions/12345", "cursor": null }
```

- `url` must be `https`, or `http` upgraded to `https`, on a public host.
  Private, loopback, and link-local addresses are refused after DNS
  resolution, so a page cannot point the tool at the operator's network.
  The check runs on every redirect hop, not only the URL given, and the
  connection is pinned to the addresses checked.
- arXiv URLs are refused with `INVALID_INPUT` and a `next` hint to
  [`lit_get_paper`](../../literature/tools/lit_get_paper.md), which reads the LaTeX source.
  A redirect to arXiv is refused the same way, with the same code and
  message, before arXiv is requested. A URL refused before the fetch (a
  malformed URL, a direct arXiv URL) spends none of the daily cap.
- The response must be HTML or plain text, at most 5 MB. A PDF is refused
  with `INVALID_INPUT`, as an error and not an `unknowns` entry; the error
  message suggests finding an HTML or arXiv version.

## Returns

```json
{
  "outcome": "ok",
  "data": {
    "untrusted": true,
    "url": "https://mathoverflow.net/questions/12345",
    "title": "…",
    "class": "qa",
    "retrieved_at": "2026-10-22T14:03:00Z",
    "text": "…",
    "math": "tex"
  },
  "cursor": "c2"
}
```

- The main text is extracted deterministically: scripts, styles,
  navigation, hidden elements, and comments are removed; headings,
  paragraphs, lists, and code are kept as Markdown.
- Math is returned as TeX where the page carries it (MathJax and KaTeX
  sources, `<annotation encoding="application/x-tex">`), and `math` says
  whether it was `tex`, `rendered` (lost), or `none`.
- At most 12,000 characters per page of text, with a `cursor` for the
  rest.
- On MathOverflow and Math Stack Exchange, answers are returned with their
  score and acceptance, highest first.

## Errors

| Code | When |
| --- | --- |
| `INVALID_INPUT` | a malformed URL, a private address or a redirect to one, an arXiv URL or a redirect to one, a PDF or other unsupported content type, a page over 5 MB |
| `NOT_FOUND` | 404 or 410 |
| `FORBIDDEN` | 401 or 403, or `robots.txt` disallows the path |
| `RATE_LIMITED` | 429; `retry_after_ms` is set |
| `TIMEOUT` | no response in 20 s |
| `BACKEND_UNAVAILABLE` | the site answered 5xx, was unreachable, or redirected more than five times |
| `BUDGET_EXCEEDED` | the project's daily web cap is spent |

A `cursor` is `c2`, `c3`, and so on; each page is a fresh fetch of the URL,
so a page that changed between calls can shift the break.

The fetch sends the same `User-Agent` as the literature tools, naming
supermath and `SUPERMATH_CONTACT`, and honours `robots.txt`.
