# Math solving harness

- **Status:** Draft (scaffold implemented)
- **Owner:** Maintainers
- **Research:** [`../research/math-superagent/`](../research/math-superagent/README.md),
  [`../research/rea-and-jacobian/`](../research/rea-and-jacobian/README.md)

## Problem

supermath needs an agent that takes one mathematical problem, works on it, and
says what it established. Earlier work in
[senamakel/math-superagent](https://github.com/senamakel/math-superagent)
built that on its own agent loop. That loop now exists, maintained, as
[OpenHuman](https://github.com/tinyhumansai/openhuman)'s embeddable runtime,
so this harness should be a thin layer over OpenHuman rather than another
agent loop.

The harness also needs memory that outlives one run, so a second attempt at a
problem starts from what the first one learned. math-superagent settled on
CortexDB for that after Cognee lost findings silently; this harness keeps
CortexDB.

## Goals

- `crates/supermath-agent`: a library and a `supermath-agent` binary that
  embed OpenHuman through `openhuman-embed`.
- One run reads a problem statement from a Markdown file, recalls what memory
  holds for that problem, runs an OpenHuman agent turn, and reports a verdict.
- The verdict is three-valued: **proved**, **refuted**, or **not
  established**, with the reason. A run that ran out of budget is never
  reported as a refutation.
- What the run concluded is written back to CortexDB, scoped to the problem.
- `docker compose up` brings up CortexDB and runs the harness against a
  problem in `problems/`.

## Non-goals

- The roles, ledgers, schools, and Lean pipeline of math-superagent. They are
  documented in the research notes and come later, one spec each.
- Formal proof checking. A proved verdict is the model's claim and is labeled
  as such.
- Multi-tenant memory. One deployment owns one scope root.

## Proposed behavior

```sh
supermath-agent solve problems/sum-of-odd-numbers.md
```

1. **Problem.** The slug is the file stem, lowercased, with every character
   outside `[a-z0-9-]` replaced by `-`. The statement is the file's contents.
   An empty file is `Error::EmptyProblem`.
2. **Recall.** `POST /v1/recall` at the problem's brain scope with the
   statement as the query. An empty pack, or a `404` for a scope nothing has
   been written to, is "nothing recorded", not an error.
3. **Turn.** One OpenHuman agent turn with the solver prompt, the statement,
   and the recalled notes. The agent's provider is an OpenAI-compatible route
   (`SUPERMATH_API_BASE_URL`, `SUPERMATH_API_KEY`, `SUPERMATH_MODEL`).
4. **Verdict.** The reply must end with a line `VERDICT: proved`,
   `VERDICT: refuted`, or `VERDICT: not-established`. A reply with no verdict
   line, or an unknown verdict word, is read as not established with reason
   `NoVerdict`; the harness never guesses a positive answer.
5. **Remember.** `POST /v1/experience?wait=indexed` with the verdict and the
   reply's summary. The write's `stages_completed` must include `indexed`; a
   write that stored bytes it has not indexed is an error, not a success.

### Memory scopes

```text
{CORTEX_SCOPE_ROOT}/project:{slug}/app:brain
```

`CORTEX_SCOPE_ROOT` defaults to `ws:supermath`. The scope is built only from
configuration and the problem slug. No model output or tool argument reaches a
scope, so one problem cannot read or write another's memory through the agent.

### Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `SUPERMATH_API_BASE_URL` | `https://openrouter.ai/api/v1` | OpenAI-compatible API root |
| `SUPERMATH_API_KEY` | required | Bearer token for that API |
| `SUPERMATH_MODEL` | required | Model id sent to that API |
| `SUPERMATH_WORKSPACE` | `./.supermath` | OpenHuman workspace and agent home |
| `CORTEX_API_URL` | `http://localhost:3141` | CortexDB base URL |
| `CORTEX_API_KEY` | required | CortexDB operator key |
| `CORTEX_SCOPE_ROOT` | `ws:supermath` | Scope subtree this deployment owns |

A required variable that is unset or blank is `Error::MissingConfig` naming
the variable.

## Invariants and constraints

- "Not established" is never collapsed into "refuted".
- A memory write is reported as stored only when CortexDB reports it indexed.
- No memory scope is derived from model output.
- The harness does not evaluate or execute caller-supplied text itself; any
  computation happens inside OpenHuman's sandboxed tools.

## Acceptance criteria

- Unit tests cover slug derivation, verdict parsing (each verdict, a missing
  line, an unknown word, trailing whitespace), configuration errors, recall
  rendering, and the indexed-barrier check.
- Memory client tests run against a loopback stub of the CortexDB endpoints,
  never the network.
- `docker compose config --quiet` passes, and `docker compose up` starts
  CortexDB healthy before the agent runs.

## Open questions

1. Should memory move behind OpenHuman's own memory-engine seam (so every
   OpenHuman agent sees CortexDB), instead of the harness calling CortexDB
   around the turn?
2. Which math tools come first: Jacobian over MCP, a Lean checker, or a
   sandboxed Python?
3. Does the harness become a TinyBus module, or stay a standalone binary that
   other modules call?
