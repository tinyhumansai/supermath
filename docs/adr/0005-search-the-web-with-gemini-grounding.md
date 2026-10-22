# 5. Search the web with Gemini and Google Search grounding

- **Status:** Proposed; amended by 6 (the web tools live in `supermath-web`)
- **Date:** 2026-10-22

## Context

The literature tools read arXiv, Semantic Scholar, and OEIS. Much of what
an agent needs before attacking a conjecture is elsewhere: MathOverflow,
research blogs, course notes, errata, withdrawn claims. The toolkit needs a
web search that returns sources an agent can check, not a model's
recollection.

The maintainers will provide a Gemini API key. Gemini's Google Search
grounding runs real Google searches and returns, beside its answer, the
queries it ran, the pages it used, and which span of the answer each page
supports.

[ADR 0004](0004-split-the-toolkit-into-an-mcp-crate.md) says `supermath-mcp`
calls no model, so that its answers are deterministic and its tests need no
provider.

## Decision

- Add `web_search`, `web_research`, and `web_read` to `supermath-mcp`
  ([`specs/toolkit/web/`](../specs/toolkit/web/README.md)), backed by the Gemini
  API's `generateContent` with the `google_search` tool, keyed by
  `GEMINI_API_KEY`, on a pinned model named in `SUPERMATH_WEB_MODEL`.
- Treat Gemini here as a search backend, not a reasoner. This amends ADR
  0004's rule to: `supermath-mcp` calls no model **except Gemini as the
  web search backend**. Its prose is kept only where a grounding support
  ties it to a source, at temperature 0, behind a 24-hour cache, and is
  never evidence on its own.
- Cap web-sourced claims at `asserted` in the ledger.
- Read pages with a deterministic fetch and extraction, not a model.
- Pass Google's search suggestions through to callers, as the grounding
  terms require for results shown to end users.

## Consequences

- The agent gets current, sourced web material that the literature tools
  cannot reach, with every sentence traceable to a page.
- A second paid API and a second key. Without the key, the web tools say
  `BACKEND_UNAVAILABLE` and the rest of the toolkit is unaffected.
- `supermath-mcp` is no longer model-free. Its tests still are: they replay
  recorded Gemini responses from a loopback stub.
- Results depend on Google's index on the day; the cache and the recorded
  queries make a run's web evidence inspectable, not reproducible.
