# supermath-web

The `web_*` tools: Gemini with Google Search grounding for sourced answers
from the open web, and a plain page reader. The spec is
[`docs/specs/toolkit/web/`](../../docs/specs/toolkit/web/README.md); the
decision is [ADR 0005](../../docs/adr/0005-search-the-web-with-gemini-grounding.md).
Depends on `supermath-tool` only.

## Tools

| Tool | Kind | Exposure | Does |
| --- | --- | --- | --- |
| `web_search` | agentic (Gemini) | direct | one grounded turn; findings tied to their sources |
| `web_research` | agentic (Gemini) | deferred | one grounded turn per facet, merged into a brief with disagreements |
| `web_read` | primitive | deferred | a URL's main text as Markdown, math as TeX, paged |

Build them with `WebToolkit::new(WebConfig::from_env()?)`; `tools()` returns
the three, `register(&mut registry)` adds them to a `ToolRegistry`.

## Design

- **Sources, not prose.** Gemini's answer is split into sentences; a sentence
  survives only when a `groundingSupports` span covers it, and it carries the
  sources that support it. Other sentences are dropped and counted in
  `unknowns`. An answer with no `groundingMetadata` is `unknown`.
- **Redirects.** Grounding URIs are redirects on `vertexaisearch.cloud.google.com`.
  The resolver `HEAD`s that host (five hops, 5 s) and reads the `Location`; it
  never requests the destination. A redirect that loops, dead-ends, or does not
  resolve drops its source, and any sentence left without one.
- **Classes.** Each source is classified by domain from a data table
  (`CLASS_TABLE`); arXiv sources carry their id and `next` suggests
  `lit_get_paper`.
- **No fallback.** Without `GEMINI_API_KEY` the Gemini tools answer
  `BACKEND_UNAVAILABLE`. The key is read once into a redacted field, sent only
  in the `x-goog-api-key` header, and never logged or returned.
- **Cache and cap.** Responses are cached 24 h under `$SUPERMATH_CACHE/web`,
  keyed by model and prompt. `SUPERMATH_WEB_DAILY_CAP` calls per project per
  UTC day, counted in memory.
- **`web_read` safety.** `https` only (`http` is upgraded); every address a host
  resolves to must be public, and the connection is pinned to the addresses
  that were checked, on every redirect hop; `robots.txt` is honoured; 5 MB and
  20 s limits; no PDFs; arXiv URLs are sent to `lit_get_paper`. Page text is
  returned inside `data` marked `"untrusted": true`.

## Configuration

| Variable | Default |
| --- | --- |
| `GEMINI_API_KEY` | unset |
| `SUPERMATH_WEB_MODEL` | `gemini-2.5-flash` |
| `SUPERMATH_WEB_DAILY_CAP` | `200` |
| `SUPERMATH_CACHE` | no cache |
| `SUPERMATH_CONTACT` | none (goes in the `User-Agent`) |
| `SUPERMATH_WEB_BASE_URL` | Google's API root (tests point it at a stub) |

## Tests

`cargo test -p supermath-web`. Unit tests sit in each module's `test.rs`;
`tests/web/` drives the tools end to end against loopback stubs with recorded
Gemini responses in `tests/fixtures/` (grounded, unsupported sentences,
ungrounded, no source, 429, safety block, a redirect chain). Nothing touches
the network. `live_web_search` runs one real grounded search and skips unless
`GEMINI_API_KEY` and `SUPERMATH_LIVE_WEB` are set.
