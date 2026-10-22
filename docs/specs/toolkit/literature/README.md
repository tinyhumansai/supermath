# Literature tools

- **Status:** Implemented
- **Crate:** `supermath-literature`
- **Parent:** [`README.md`](../README.md)

Before attacking a conjecture, find out what is known: who stated it, what
is proved, what was tried and failed, and whether the gap has a name
([`strategies/01-intake.md`](../../../research/strategies/01-intake.md)). These
tools find and read papers. They never summarize with a model; reading for
meaning is [`agent_digest_paper`](../subagents/tools/agent_digest_paper.md)'s job.

## Sources

| Source | Used for | Access |
| --- | --- | --- |
| arXiv | search, metadata, LaTeX source | the [arXiv API](https://info.arxiv.org/help/api/index.html) and `https://arxiv.org/e-print/<id>` |
| Semantic Scholar | references and citations | the Graph API, with `SEMANTIC_SCHOLAR_API_KEY` if set |
| OEIS | integer sequences | `https://oeis.org/search?fmt=json` |

Not covered, and named in every search's `coverage.not_covered`: zbMATH,
MathSciNet, journals outside arXiv, and books. A search that finds nothing
says so only for the sources it covered.

### Politeness

- arXiv asks for no more than one request every three seconds. The client
  queues requests to keep that rate for the whole process, not per tool
  call, and sends a `User-Agent` naming supermath and a contact address
  (`SUPERMATH_CONTACT`).
- Responses are cached on disk under `$SUPERMATH_CACHE/literature`, keyed by
  the SHA-256 of the URL, for 24 hours for searches (and for the metadata of
  an unversioned id) and indefinitely for a versioned paper (`2310.01234v2`
  does not change). `get_paper` and `extract_statements` pin the e-print to
  the version the metadata names, so the source is fetched once. With
  `SUPERMATH_CACHE` unset there is no cache. `coverage.cached_age` is the
  age of a cached answer in seconds.
- An HTTP 429 or 503 becomes `RATE_LIMITED` with `retry_after_ms`, from the
  `Retry-After` header (seconds) or a per-source default (3 s arXiv, 1 s the
  others). The tool never retries silently past its budget.
- A timeout is `TIMEOUT`; a refused connection, any other bad status, or an
  undecodable answer is `BACKEND_UNAVAILABLE`.

### Reading the source

An e-print is a gzipped tarball, a single gzipped `.tex`, a bare tarball, or
a PDF; the reader tells them apart by magic bytes. `\input` and `\include`
are followed (cycles and missing files go to `unknowns`), `%` comments are
removed, and argument-free macros (`\newcommand`, `\providecommand`,
`\def`, `\DeclareMathOperator`) are expanded. Macros with arguments and
`\renewcommand` are left as written. **PDF text is not extracted**: no
parser for it ships here, so a PDF-only paper (or an e-print arXiv does not
serve) returns its metadata and an `unknowns` entry, and `unknown` when a
section or statements were asked for.

## Crate

| | |
| --- | --- |
| Name | `supermath-literature` |
| Depends on | `supermath-tool`, `reqwest`, `quick-xml` (the Atom feed), `flate2` and `tar` for e-prints, `serde_json`, `tokio` |
| Public API | `Literature` (builds the clients and `tools()`), `LiteratureConfig`, `ArxivClient`, `SemanticScholarClient`, `OeisClient`, `RateLimiter`, `ResponseCache`, `Clock`, `flatten_latex`, `extract_statements`, and the tools below |
| Configuration | `SUPERMATH_CONTACT`, `SEMANTIC_SCHOLAR_API_KEY`, `SUPERMATH_CACHE`; base URLs `SUPERMATH_ARXIV_API_URL`, `SUPERMATH_ARXIV_EPRINT_URL`, `SUPERMATH_SCHOLAR_URL`, `SUPERMATH_OEIS_URL` (tests point these at loopback stubs) |
| Tests | recorded fixtures under `tests/fixtures/` served by a loopback stub; the rate limiter on a fake clock; `live_literature_arxiv` gated by `SUPERMATH_LIVE_ARXIV` |

## Tools

Each tool is specified in its own file in this crate's `tools/` folder, in
the shape [`../tools.md`](../tools.md) sets out.

| Tool | Given → returns |
| --- | --- |
| [`lit_search_arxiv`](tools/lit_search_arxiv.md) | a query → paged papers |
| [`lit_get_paper`](tools/lit_get_paper.md) | an arXiv id → metadata, section index, a section's text |
| [`lit_extract_statements`](tools/lit_extract_statements.md) | an arXiv id → numbered statements and their `\ref` edges |
| [`lit_citations`](tools/lit_citations.md) | an arXiv id → references or citing papers |
| [`lit_oeis`](tools/lit_oeis.md) | terms or a query → matching sequences |

## Invariants

- No `lit_*` tool calls a model.
- Every row carries its source and, for arXiv, the version read.
- A fact recorded from a paper cites `{arxiv_id, version, label or section}`.

## Acceptance criteria

- Unit tests run each tool against recorded responses served by a loopback
  stub: an Atom feed, a LaTeX e-print tarball, a PDF-only paper, an OEIS
  JSON answer, a 429.
- `lit_extract_statements` is tested on a fixture with `\newtheorem` variants,
  shared counters, `\input` files, and an unlabeled theorem.
- The rate limiter is tested with a fake clock, never wall time.
- `live_literature_arxiv`, gated by `SUPERMATH_LIVE_ARXIV`, searches arXiv once.

## Open questions

1. Should search add a semantic index of abstracts in CortexDB, so related
   papers surface by meaning as well as words?
2. Do we need full-text search across a project's downloaded papers?
