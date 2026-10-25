# Web tools

- **Status:** Implemented
- **Crate:** `supermath-web`
- **Parent:** [`README.md`](../README.md)
- **Decision:** [`../../../adr/0005-search-the-web-with-gemini-grounding.md`](../../../adr/0005-search-the-web-with-gemini-grounding.md)

Much of what is known about a conjecture never reaches arXiv: MathOverflow
answers, Tao's and Kalai's blogs, course notes, Polymath wikis, talk slides,
errata, and the comment thread where someone spotted the gap. These tools
surface well-sourced material from the open web, through Gemini with
Google Search grounding, and hand the agent the **sources**, not just
Gemini's prose about them.

## Backend: Gemini with Google Search grounding

One request is a `generateContent` call to the Gemini API with the Google
Search tool enabled:

```http
POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent
x-goog-api-key: $GEMINI_API_KEY

{
  "contents": [{ "role": "user", "parts": [{ "text": "<research prompt>" }] }],
  "tools": [{ "google_search": {} }],
  "generationConfig": { "temperature": 0 }
}
```

Gemini decides which searches to run, reads the results, and answers, so
`web_search` and `web_research` are **agentic (Gemini)** tools: a model turn
behind a tool interface
([`README.md`](../README.md#primitive-and-agentic-tools)). `web_read` is a
primitive. The
response's `candidates[0].groundingMetadata` carries what we keep:

| Field | Becomes |
| --- | --- |
| `webSearchQueries` | `coverage.searched`: the queries Google actually ran |
| `groundingChunks[].web.{uri, title}` | the source list |
| `groundingSupports[]` | which span of the answer each source supports, with `groundingChunkIndices` |
| `searchEntryPoint.renderedContent` | Google's search-suggestions widget, passed through in `data.search_entry_point` (see [terms](#terms)) |

The model is `SUPERMATH_WEB_MODEL` (default: the current Gemini Flash
model, pinned by name in config, never an alias that moves). Google
retires model names; a 404 from Gemini is reported as `BACKEND_UNAVAILABLE`
with a message that names the model, says it may be retired, and points at
`SUPERMATH_WEB_MODEL`. The key is
`GEMINI_API_KEY`; without it, `web_search` and `web_research` return
`BACKEND_UNAVAILABLE`, and no other engine stands in. `web_read` is a plain
fetch and needs no key.

### Sources, not prose

Gemini's answer is a model's summary, so the tools treat it as a guide to
the sources and nothing more:

- Every sentence returned is attached to the sources that support it, from
  `groundingSupports`. A sentence with no support is dropped from
  `data.findings` and counted in `unknowns` ("2 unsupported sentences
  omitted").
- Grounding URIs arrive as `vertexaisearch.cloud.google.com` redirects. The
  tool resolves each one with `HEAD` requests to the redirect host (no body,
  five redirects at most, 5 s) and takes the real URL from the last
  `Location`; it does not request the destination itself. A redirect that
  loops, dead-ends (a 404, a 5xx, or no `Location`), or times out is
  reported as unresolved in `unknowns`, and its source is dropped along with
  any sentence it alone supported. A source the agent cannot open is not a
  source.
- Ledger entries from the web cite the resolved URL, never the redirect and
  never Gemini.

### Source classes

Each resolved source is classified by its domain, so the agent can weigh it
without opening it:

| Class | Domains (examples) |
| --- | --- |
| `preprint` | arxiv.org, hal.science, eprint.iacr.org |
| `journal` | DOI landing pages, ams.org, springer.com, sciencedirect.com, projecteuclid.org |
| `qa` | mathoverflow.net, math.stackexchange.com |
| `reference` | oeis.org, zbmath.org, mathworld.wolfram.com, wikipedia.org, leanprover-community.github.io |
| `notes` | `*.edu`, `*.ac.*`, personal academic pages, course notes, slides |
| `blog` | terrytao.wordpress.com, gilkalai.wordpress.com, other blogs |
| `other` | everything else |

The table lives in code as data, with a unit test per row. An arXiv source
also carries its arXiv id, and `next` suggests
[`lit_get_paper`](../literature/tools/lit_get_paper.md) on it: the web tools find, the
literature tools read.

### Untrusted content

Web text is data, never instructions. Anything a page says reaches the
agent inside `data`, marked `"untrusted": true`, and tool descriptions say
so. `web_read` strips scripts, styles, and hidden elements before
returning text, and never follows instructions or links on its own. A page
that tells the agent to do something is reporting what the page says,
nothing more.

### Budgets and cache

| Limit | Default |
| --- | --- |
| Gemini requests per `web_search` | 1 |
| Gemini requests per `web_research` | 6 |
| `web_*` calls per project per day | 200 (`SUPERMATH_WEB_DAILY_CAP`) |
| Wall time per call | 30 s (`web_search`), 180 s (`web_research`), 20 s (`web_read`) |

Responses are cached under `$SUPERMATH_CACHE/web` for 24 hours, keyed by
model and prompt (which holds the arguments), so a repeated question costs
no Gemini request. A cached answer says so in `coverage.cached_age` (for
`web_research`, the age of the oldest cached facet). The daily cap counts
every admitted `web_*` call, cached or not, in memory per UTC day: a call
is admitted once its arguments have parsed and passed validation (and, for
`web_search` and `web_research`, a key is set). A call refused as
`INVALID_INPUT` before admission, including `web_read` on an arXiv URL,
spends nothing; one that fails after admission (a timeout, a 404, a
redirect into a private address) does.

### Terms

Google's terms for grounding with Google Search require showing the search
suggestions (`searchEntryPoint`) to an end user alongside grounded results.
The tools pass it through unchanged in `data.search_entry_point`; a
front end that shows web results to a person must render it. An agent that
only reads the sources does not.

## Status rules

Web sources sit below the literature in the ledger's
[status lattice](../README.md#claim-status):

- A `web` source can record a claim as `asserted`, never `cited`. A
  MathOverflow answer is a strong lead, not a citation.
- A web source that is an arXiv paper is re-read through
  [`lit_get_paper`](../literature/tools/lit_get_paper.md) before it is cited, and the
  ledger cites the arXiv id.
- `web` sources are recorded as `{url, title, class, retrieved_at}`
  ([`ledger/`](../ledger/README.md#sources)).

## Crate

| | |
| --- | --- |
| Name | `supermath-web` |
| Depends on | `supermath-tool`, `reqwest`, `scraper` (the HTML parser), `serde_json`, `tokio` |
| Public API | `WebToolkit` (builds the tools), `WebConfig`, `GeminiClient`, `Grounding`, `SourceClass`, `resolve_redirect`, `extract_main_text`, and the tools below |
| Configuration | `GEMINI_API_KEY`, `SUPERMATH_WEB_MODEL`, `SUPERMATH_WEB_DAILY_CAP`, `SUPERMATH_CACHE`, `SUPERMATH_CONTACT`, and `SUPERMATH_WEB_BASE_URL` (the Gemini API root, for stubs) |
| Tests | recorded Gemini responses and pages from a loopback stub; redirect chains; a test per source class; `live_web_search` gated by `GEMINI_API_KEY` and `SUPERMATH_LIVE_WEB` |

## Tools

Each tool is specified in its own file in this crate's `tools/` folder, in
the shape [`../tools.md`](../tools.md) sets out.

| Tool | Given → returns |
| --- | --- |
| [`web_search`](tools/web_search.md) | a question → grounded findings with their sources |
| [`web_research`](tools/web_research.md) | a research question → a sourced brief across several searches |
| [`web_read`](tools/web_read.md) | a URL → the page's main text, paged |

## Invariants

- No `web_*` tool returns a sentence without a source supporting it.
- No `web_*` tool raises a claim above `asserted`.
- No `web_*` tool falls back to another search engine or to Surplus
  when Gemini is unavailable.
- Web content is never executed, and never treated as an instruction.

## Acceptance criteria

- Unit tests replay recorded Gemini responses from a loopback stub: a
  grounded answer, an answer with unsupported sentences, an ungrounded
  answer (no `groundingMetadata`), a 429, and a safety block.
- Redirect resolution is tested against a loopback redirect chain,
  including a loop and a dead end.
- Source classification has a test per class.
- `live_web_search`, gated by `GEMINI_API_KEY` and `SUPERMATH_LIVE_WEB`,
  runs one grounded search and asserts at least one resolved source.

## Open questions

1. Should `web_research` run on the Gemini Pro model by default, trading
   cost for depth?
2. Should we add Gemini's URL context tool to `web_read`, or keep reading
   deterministic?
3. Is a per-project daily cap enough, or do we need a spend cap in dollars?
