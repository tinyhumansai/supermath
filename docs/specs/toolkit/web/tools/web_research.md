# `web_research`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-web` | [Web](../README.md) | agentic (Gemini) | deferred | read | P3 |

**Given → returns:** a research question → a sourced brief across several searches.

The intake tool for a problem: what is known, who worked on it, what was
tried, what is open. It runs several grounded Gemini turns, one per
facet, and assembles them into one brief whose every sentence carries its
sources. Use [`web_search`](web_search.md) for a single fact; use this
before starting on a conjecture
([`strategies/01-intake.md`](../../../../research/strategies/01-intake.md)).

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
  the first five below. One Gemini request per facet, at most six requests
  per call.
- `prefer` names [source classes](../README.md#source-classes) listed first
  in the merged `sources`. Unlike `web_search`, it only ranks: the facet
  prompts are fixed and do not mention it, it filters nothing, and it
  decides which sources survive the 30-source limit.

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

## Turn

| | |
| --- | --- |
| Model | `SUPERMATH_WEB_MODEL`, temperature 0 |
| What the model does | one grounded turn per facet; in each, it runs the Google searches it chooses and answers that facet |
| Tools in the turn | Google Search grounding only |
| Budget | at most 6 Gemini requests, 180 s |

The facets and their prompts are fixed by the tool. The tool merges
sources, detects disagreements by comparing numbers and status words, and
drops unsupported sentences; no model plans the research or adjudicates
between sources.

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
    ],
    "search_entry_points": ["<html …>"]
  },
  "coverage": { "searched": ["…every query Google ran…"], "not_covered": ["paywalled full text"] },
  "unknowns": [],
  "next": ["lit_get_paper {\"arxiv_id\": \"…\"}"]
}
```

- Sources are shared across facets and numbered once.
- **Disagreements** are surfaced, not resolved: when two findings of the
  same facet give different statuses, dates, or bounds for the same thing,
  both are listed. Findings in different facets are not compared. The tool
  detects them by comparing the findings' numbers and status words; it
  does not ask a model to adjudicate.
- A facet with no grounded finding says "no source found", which is
  `unknown` for that facet, not evidence of absence. The call's outcome is
  `ok` when at least one facet found something, `unknown` otherwise.
- At most eight findings per facet and 30 sources in all, and the
  findings' text totals at most 1,500 words. Nothing is cut silently:
  - findings past the eighth of a facet are counted in `unknowns`
    (`facet status: 4 findings omitted by the limit of 8 per facet`);
  - a finding whose every source fell outside the 30 kept is dropped and
    counted (`2 findings omitted by the source limit`);
  - the word cap cuts whole findings, round by round across facets (every
    facet's first finding, then every facet's second, …); the first that
    does not fit ends the brief, and the cut is counted
    (`15 findings omitted to keep the brief under 1,500 words`).
  A facet left with no findings by a cut says so in its `note`
  (`findings omitted by the source limit` or `… by the word limit`)
  instead of "no source found".
- It records nothing in the ledger itself. `next` suggests `lit_get_paper`
  for the arXiv sources it found; the agent, or a sub-agent, decides what is
  worth keeping and records it.
- `search_entry_points` lists the distinct Google search-suggestion widgets
  of the facet turns, passed through unchanged (see
  [terms](../README.md#terms)).
- Disagreements are found within one facet: a status word ("open",
  "unsolved") against a settled one ("proved", "refuted"), or the same
  sentence with different figures. A finding that uses both kinds of word
  is not compared.

## Errors

As [`web_search`](web_search.md#errors), with a 180 s timeout. A facet that
times out or is rate-limited is reported in `unknowns` and the others are
returned; only a call where every facet failed is an error.
