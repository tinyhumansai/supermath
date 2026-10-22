# Literature tools

- **Status:** Draft
- **Parent:** [`README.md`](README.md)

Before attacking a conjecture, find out what is known: who stated it, what
is proved, what was tried and failed, and whether the gap has a name
([`strategies/01-intake.md`](../../research/strategies/01-intake.md)). These
tools find and read papers. They never summarize with a model; reading for
meaning is [`agent_digest_paper`](agents.md#agent_digest_paper)'s job.

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
- Responses are cached on disk under the workspace, keyed by URL, for 24
  hours for searches and indefinitely for a versioned paper (`2310.01234v2`
  does not change).
- An HTTP 429 or 503 becomes `RATE_LIMITED` with `retry_after_ms`. The tool
  never retries silently past its budget.

## Tools

### `lit_search_arxiv`

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

### `lit_get_paper`

Given an arXiv id, returns its metadata and section index, or one section.

```json
{ "arxiv_id": "2310.01234", "section": "3.2" }
```

- Without `section`: metadata, the abstract, and the section tree as
  `{number, title, chars}`.
- With `section`: that section's text, at most 12,000 characters, with a
  `cursor` for the rest.
- The source is the LaTeX e-print, flattened: `\input` followed, macros from
  the preamble expanded where they are simple `\newcommand`s, comments
  removed. Math stays as LaTeX.
- If the e-print is PDF-only, the text comes from the PDF, and `unknowns`
  says so: formulas from PDF text are unreliable, and section numbers may be
  guessed.

### `lit_extract_statements`

Given an arXiv id, returns its numbered statements: every `theorem`,
`lemma`, `proposition`, `corollary`, `conjecture`, `definition`, and
`remark`-like environment the preamble declares with `\newtheorem`.

Each row: `{label, kind, number, section, statement, refs}`. `refs` lists the
labels the statement and its proof `\ref`, which gives a dependency graph
of the paper without reading it. The extraction is a parse, not a model:
statements the parser could not attach to a number are listed in
`unknowns`, not guessed.

This is the input to formalization: a locator `{arxiv_id, label}` names one
statement exactly, and the ledger cites it.

### `lit_citations`

Given an arXiv id and a direction (`references` or `cited_by`), returns
papers from Semantic Scholar, each with its arXiv id when it has one.
`cited_by` answers "has anyone built on this, or refuted it?".

### `lit_oeis`

Given either a list of at least four integers or a query, returns matching
OEIS sequences: `{id, name, terms, formula, references}` with terms capped at
30 and formulas at five lines. `coverage` notes that OEIS matches a
subsequence, so a match is a lead, not an identification.

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
- `live_arxiv_search`, gated by `SUPERMATH_LIVE_ARXIV`, searches arXiv once.

## Open questions

1. Should search add a semantic index of abstracts in CortexDB, so related
   papers surface by meaning as well as words?
2. Do we need full-text search across a project's downloaded papers?
