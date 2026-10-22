# `web_research`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Web](../web.md) | deferred | read | P3 |

**Given → returns:** a research question → a sourced brief across several searches.

The intake tool for a problem: what is known, who worked on it, what was
tried, what is open. It runs several grounded Gemini requests, one per
facet, and assembles them into one brief whose every sentence carries its
sources. Use [`web_search`](web_search.md) for a single fact; use this
before starting on a conjecture
([`strategies/01-intake.md`](../../../research/strategies/01-intake.md)).

```json
{
  "question": "The Sendov conjecture",
  "facets": ["status", "partial_results", "approaches", "failed_attempts", "related_problems"],
  "prefer": ["preprint", "journal", "qa"]
}
```

- `question` names the problem or asks the question, at most 500
  characters.
- `facets` picks which searches to run, from a fixed set; the default is
  all five below. One Gemini request per facet, at most six requests per
  call.

| Facet | Asks |
| --- | --- |
| `status` | is it open, proved, or refuted, and as of when |
| `partial_results` | the special cases and bounds proved, by whom, with the strongest first |
| `approaches` | the techniques applied to it, and which made progress |
| `failed_attempts` | claimed proofs that were withdrawn, gaps found, approaches known not to work |
| `related_problems` | equivalent forms, generalizations, and what it would imply |
| `formalization` | whether any of it is in Mathlib or another Lean project (opt-in) |

The facets are deterministic: the tool writes each facet's prompt from a
template, and Gemini runs the searches. The tool does not ask a model to
plan, so the same arguments run the same searches.

## Returns

```json
{
  "outcome": "ok",
  "data": {
    "untrusted": true,
    "facets": {
      "status": {
        "findings": [{ "text": "…", "sources": [0, 3] }]
      },
      "failed_attempts": {
        "findings": [],
        "note": "no source found"
      }
    },
    "sources": [{ "url": "…", "title": "…", "class": "preprint", "arxiv_id": "…" }],
    "disagreements": [
      { "facet": "status", "claims": [{ "text": "…", "sources": [1] }, { "text": "…", "sources": [4] }] }
    ]
  },
  "coverage": { "searched": ["…every query Google ran…"], "not_covered": ["paywalled full text"] },
  "unknowns": [],
  "next": ["lit_get_paper {\"arxiv_id\": \"…\"}", "ledger_record {…}"]
}
```

- Sources are shared across facets and numbered once.
- **Disagreements** are surfaced, not resolved: when two facets or two
  sources give different statuses, dates, or bounds for the same thing,
  both are listed. The tool detects them by comparing the findings'
  numbers and status words; it does not ask a model to adjudicate.
- A facet with no grounded finding says "no source found", which is
  `unknown` for that facet, not evidence of absence. The call's outcome is
  `ok` when at least one facet found something, `unknown` otherwise.
- At most eight findings per facet and 30 sources in all; the brief stays
  under about 1,500 words.
- It records nothing in the ledger itself. `next` suggests
  `ledger_record` for the leads it found; the agent, or a sub-agent,
  decides what is worth keeping.

## Errors

As [`web_search`](web_search.md#errors), with a 180 s timeout. A facet that
times out or is rate-limited is reported in `unknowns` and the others are
returned; only a call where every facet failed is an error.
