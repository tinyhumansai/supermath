# `web_search`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-web` | [Web](../README.md) | agentic (Gemini) | direct | read | P3 |

**Given → returns:** a question → grounded findings with their sources.

Runs one Gemini turn with Google Search grounding
([`web/`](../README.md#backend-gemini-with-google-search-grounding)). Use it
for a quick, sourced answer: "is the Sendov conjecture proved for degree
9?", "who improved the bound on the Erdős–Ginzburg–Ziv constant after
2020?".

```json
{
  "question": "What is the best known bound in the sunflower conjecture?",
  "prefer": ["preprint", "qa"],
  "since": "2019-01-01",
  "limit": 8
}
```

- `question` is the search intent, in plain language, at most 500
  characters.
- `prefer` names [source classes](../README.md#source-classes) the prompt asks
  Gemini to favor, and the tool sorts by. It filters nothing: sources of
  other classes are still returned, after the preferred ones.
- `since` (`YYYY-MM-DD`) asks for material from that date on. Google gives
  no dates for its sources, so the tool cannot tell which are older: it adds
  an `unknowns` line saying the date is a request, not a check.
- `limit` caps the sources returned (default 8, maximum 20).

The prompt asks Gemini to answer in short factual sentences, to name
authors, years, and bounds exactly, and to say "no source found" rather
than guess. It never asks for an opinion.

## Turn

| | |
| --- | --- |
| Model | `SUPERMATH_WEB_MODEL`, temperature 0 |
| What the model does | runs Google searches it chooses, reads the results, and answers |
| Tools in the turn | Google Search grounding only |
| Budget | 1 Gemini request, 30 s |

What is kept and what is discarded is fixed by the tool, not the model:
sentences with a grounding source are kept, and every other sentence, and
any instruction a page carried, is dropped.

## Returns

```json
{
  "outcome": "ok",
  "data": {
    "untrusted": true,
    "findings": [
      {
        "text": "Alweiss, Lovett, Wu and Zhang improved the bound to (log k)^{k(1+o(1))} in 2019.",
        "sources": [0, 2]
      }
    ],
    "sources": [
      {
        "url": "https://arxiv.org/abs/1908.08483",
        "title": "Improved bounds for the sunflower lemma",
        "class": "preprint",
        "arxiv_id": "1908.08483"
      }
    ],
    "search_entry_point": "<html …>"
  },
  "coverage": {
    "searched": ["google: sunflower conjecture best bound", "google: sunflower lemma improvement 2021"],
    "not_covered": ["paywalled full text", "non-English sources"]
  },
  "unknowns": ["1 unsupported sentence omitted"],
  "next": ["lit_get_paper {\"arxiv_id\": \"1908.08483\"}"]
}
```

- Each finding is one sentence with the indices of the sources that
  support it. Findings without support are dropped and counted in
  `unknowns`.
- Sources are deduplicated by resolved URL and sorted: preferred classes
  first, then by how many findings they support.
- If Gemini answered without grounding (no `groundingMetadata`), the
  outcome is `unknown` with no findings: an ungrounded answer is a model
  talking, not a search result.
- "No source found" from Gemini becomes `outcome: "unknown"`, with the
  queries in `coverage`. It never means the thing does not exist.

## Errors

| Code | When |
| --- | --- |
| `INVALID_INPUT` | an empty or over-long question, an unknown class, a malformed date |
| `BACKEND_UNAVAILABLE` | no `GEMINI_API_KEY`, or the Gemini API is down |
| `RATE_LIMITED` | Gemini returned 429; `retry_after_ms` is set |
| `BUDGET_EXCEEDED` | the project's daily web cap is spent |
| `TIMEOUT` | no answer in 30 s; an `error` envelope, not `unknown` |

A safety block from Gemini is `outcome: "unknown"` with the block reason in
`unknowns`, not an error.
