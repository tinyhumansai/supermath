# Conjecture toolkit

- **Status:** Draft
- **Owner:** Maintainers
- **Decisions:** [`../../adr/0003-run-lean-as-a-pinned-service.md`](../../adr/0003-run-lean-as-a-pinned-service.md),
  [`../../adr/0004-split-the-toolkit-into-an-mcp-crate.md`](../../adr/0004-split-the-toolkit-into-an-mcp-crate.md),
  [`../../adr/0005-search-the-web-with-gemini-grounding.md`](../../adr/0005-search-the-web-with-gemini-grounding.md)
- **Plan:** [`../../plans/lean-toolkit.md`](../../plans/lean-toolkit.md)
- **Research:** [`../../research/rea-and-jacobian/`](../../research/rea-and-jacobian/README.md),
  [`../../research/strategies/`](../../research/strategies/README.md),
  [`../../research/math-superagent/`](../../research/math-superagent/README.md)

## Problem

The [agent runner](../agent-runner.md) is wired, but its toolkit has three
tools: memory recall, factorization, and gcd. An agent working on a
conjecture needs to find what is known, read it, state it precisely, check
what it claims, look for counterexamples, and remember all of it across
turns. Each of those needs a tool the agent can find, call cheaply, and
trust no further than the evidence behind its answer.

## Goals

- A toolkit an OpenHuman agent can use to work on a conjecture end to end:
  literature, web research, Lean formalization and checking, computation, a claim ledger in
  CortexDB, and sub-agents for the long tasks.
- Lean is the arbiter. A claim is `formalised` only when this harness ran the
  Lean kernel on it and saw no `sorry` and no axiom outside the allowlist.
- Two crates ([`crates.md`](crates.md)): `supermath-mcp`, the toolkit and
  an MCP server any agent can use, with no OpenHuman dependency; and
  `supermath-agent`, the OpenHuman runner, which links the toolkit as host
  tools and adds the model-driven `agent_*` tools.
- Every tool follows the [contract](#tool-contract) below.

## Non-goals

- The solving loop: how the top-level agent sequences these tools, its
  prompt, and its final report. That is a separate spec.
- A proof search engine of our own. Search is Lean tactics, Mathlib search,
  and models on the ladder.
- Accepting a result as published mathematics. A checked proof is evidence
  that a Lean statement holds, not that it says what the paper says; that is
  [alignment](ledger.md#alignment), a separate axis.

## Documents

| Spec | Covers |
| --- | --- |
| [`lean.md`](lean.md) | the pinned Lean service and the `lean_*` tools |
| [`literature.md`](literature.md) | arXiv, Semantic Scholar, OEIS: the `lit_*` tools |
| [`web.md`](web.md) | web research through Gemini with Google Search grounding: the `web_*` tools |
| [`ledger.md`](ledger.md) | claims, facts, and edges in CortexDB: the `ledger_*` tools |
| [`agents.md`](agents.md) | sub-agents behind the `agent_*` tools |
| [`tools/`](tools/README.md) | one file per tool: arguments, behavior, errors |
| [`crates.md`](crates.md) | the `supermath-mcp` and `supermath-agent` crates, host tools, and the MCP server |

## Tool contract

These rules come from morluto's REA and Jacobian
([`principles.md`](../../research/rea-and-jacobian/principles.md)). They are
what makes a tool cheap for an agent to choose, call, and believe.

### Names answer a question

A tool is named `<area>_<verb>_<object>` for the question it answers, not the
engine behind it: `lean_check`, `lit_search_arxiv`, `ledger_record`. Areas are
`lit`, `web`, `lean`, `calc`, `ledger`, and `agent`. A tool establishes exactly one
postcondition, stated in its description as "given X, returns Y such that P".
Workflows are composed by the agent or by an `agent_*` tool, never hidden in
a primitive.

### Progressive disclosure

Five tools are **direct**, always in the model's catalogue: `recall_memory`,
`lit_search_arxiv`, `web_search`, `lean_check`, and `ledger_record`. Every other tool is
**deferred** and found through `tool_search`, which Jev ranks. A description's
first sentence is what Jev ranks on, so it names the question.

### One envelope

Every tool answers with the same JSON shape:

```json
{
  "outcome": "ok",
  "data": {},
  "coverage": { "searched": ["arxiv"], "not_covered": ["zbmath"] },
  "unknowns": [],
  "next": ["lit_get_paper {\"arxiv_id\": \"2310.01234\"}"],
  "cursor": null
}
```

- `outcome` is `ok`, `unknown`, or `error`. **`unknown` is never a negative
  answer**: a timeout, an exhausted budget, or a source not searched says
  nothing about the mathematics.
- `coverage` is present on every search: what was searched and what was not.
  Absence of a result outside coverage is unknown, not absence.
- `unknowns` lists anything the answer could not establish, such as a paper
  read from PDF text because its LaTeX source was unavailable.
- `next` suggests at most three follow-up calls, with arguments.
- `cursor` pages a list. Every list is capped (default 10, maximum 50), and
  each row is the smallest record that lets the agent decide whether to fetch
  more.

An `error` carries a stable `code` and a one-line `message`:

| Code | Meaning |
| --- | --- |
| `INVALID_INPUT` | arguments failed the schema or a semantic check |
| `NOT_FOUND` | the named paper, declaration, or claim does not exist |
| `BACKEND_UNAVAILABLE` | a service is down; no fallback was tried |
| `BUDGET_EXCEEDED` | admission refused the call before running it |
| `TIMEOUT` | the call ran out of time; the result is unknown |
| `RATE_LIMITED` | an upstream asked us to slow down; retry after `retry_after_ms` |
| `FORBIDDEN` | the caller may not make this transition (see the ledger) |

Tests assert codes, never message text.

### Boundaries

- Arguments are typed JSON, parsed once at the boundary. No tool evaluates a
  string as code in the host. Lean source is sent to the Lean service, which
  is sandboxed ([`lean.md`](lean.md#isolation)).
- No silent fallback. If a backend is missing, the tool says so; it never
  answers from a different engine than the one it names.
- Every call is admitted against a budget (time, size, rows) before it runs.
- Provenance only where it carries trust: `lean_check` records the
  toolchain, Mathlib revision, source digest, and elapsed time; a literature
  row records its source and version. Arithmetic does not.

### Permissions

Every tool declares `Read` or `Write` honestly; the runner maps it to
OpenHuman's permission levels and the MCP server to `readOnlyHint`
([`crates.md`](crates.md)). `lean_write` and the ledger writes are `Write`.
`agent_*` tools are `Read` themselves, because the writing happens inside
the child agent, which has its own scope ([`agents.md`](agents.md#scopes)).

## Claim status

Every claim the ledger holds has a status. Statuses only rise on evidence:

| Status | Means | Set by |
| --- | --- | --- |
| `asserted` | someone said it; the default | anyone |
| `cited` | a source says it, with a locator | a `lit_*` result |
| `checked` | it held on a stated finite range | a `calc_*` result with its frame |
| `formalised` | the Lean kernel accepted a statement of it, in this run | `supermath-mcp`, from a `lean_check` it ran |

`evidence_origin` is `collected` (a tool in this `supermath-mcp` process produced it) or
`supplied` (it arrived as text). Only `collected` evidence may set `checked`
or `formalised`. A transcript someone pasted is a claim, not evidence.

Alignment, whether a Lean statement says what the informal statement says,
is a separate field: `not_reviewed`, `accepted`, or `disputed`
([`ledger.md`](ledger.md#alignment)).

## Catalogue

Every tool lives in `supermath-mcp` except the `agent_*` tools, which need
models and live in `supermath-agent`; MCP clients get everything but those.

| Tool | Exposure | Permission | Phase | Given → returns |
| --- | --- | --- | --- | --- |
| [`recall_memory`](tools/recall_memory.md) | direct | read | shipped | a query → this project's notes |
| [`lit_search_arxiv`](tools/lit_search_arxiv.md) | direct | read | P3 | a query → paged papers |
| [`lit_get_paper`](tools/lit_get_paper.md) | deferred | read | P3 | an arXiv id → metadata, section index, a section's text |
| [`lit_extract_statements`](tools/lit_extract_statements.md) | deferred | read | P3 | an arXiv id → numbered statements and their `\ref` edges |
| [`lit_citations`](tools/lit_citations.md) | deferred | read | P5 | an arXiv id → references or citing papers |
| [`lit_oeis`](tools/lit_oeis.md) | deferred | read | P3 | terms or a query → matching sequences |
| [`web_search`](tools/web_search.md) | direct | read | P3 | a question → grounded findings with their sources |
| [`web_research`](tools/web_research.md) | deferred | read | P3 | a research question → a sourced brief across several searches |
| [`web_read`](tools/web_read.md) | deferred | read | P3 | a URL → the page's main text, paged |
| [`lean_check`](tools/lean_check.md) | direct | read | P2 | Lean source → a [verdict](lean.md#verdicts) with provenance |
| [`lean_search`](tools/lean_search.md) | deferred | read | P2 | a name, pattern, or phrase → Mathlib declarations |
| [`lean_decl`](tools/lean_decl.md) | deferred | read | P2 | a declaration name → signature, docstring, module |
| [`lean_goals`](tools/lean_goals.md) | deferred | read | P4 | source and a position → the goal state there |
| [`lean_try_tactics`](tools/lean_try_tactics.md) | deferred | read | P4 | source, a `sorry`, tactics → per-tactic outcome |
| [`lean_write`](tools/lean_write.md) | deferred | write | P4 | a module path and source → the file, checked |
| [`lean_build`](tools/lean_build.md) | deferred | read | P5 | the project package → per-module verdicts |
| [`lean_blueprint`](tools/lean_blueprint.md) | deferred | read | P5 | the project → the declaration DAG with status |
| [`calc_factor`](tools/calc_factor.md) | deferred | read | P1 | an integer → its factorization |
| [`calc_gcd_lcm`](tools/calc_gcd_lcm.md) | deferred | read | P1 | two integers → gcd and lcm |
| [`calc_search_counterexample`](tools/calc_search_counterexample.md) | deferred | read | P5 | a decidable Lean predicate and a range → a witness, or none in range |
| [`ledger_record`](tools/ledger_record.md) | direct | write | P2 | a claim or fact → its id and status |
| [`ledger_claims`](tools/ledger_claims.md) | deferred | read | P2 | filters → paged claims |
| [`ledger_link`](tools/ledger_link.md) | deferred | write | P4 | two claim ids → a `uses` edge |
| [`ledger_bound`](tools/ledger_bound.md) | deferred | write | P5 | a quantity and a bound → the best bound so far |
| [`agent_digest_paper`](tools/agent_digest_paper.md) | deferred | read | P4 | an arXiv id → facts recorded from it |
| [`agent_formalize`](tools/agent_formalize.md) | deferred | read | P4 | a statement or paper locator → Lean declarations and their verdict |
| [`agent_review_alignment`](tools/agent_review_alignment.md) | deferred | read | P4 | a declaration and its informal statement → alignment |
| [`agent_refute`](tools/agent_refute.md) | deferred | read | P5 | a statement → a counterexample, or a repaired statement |

`factor_integer` and `gcd_lcm` move to `supermath-mcp` as `calc_factor` and
`calc_gcd_lcm` in P1. Jacobian's ~1,400 operations are reached over MCP, not
reimplemented ([`crates.md`](crates.md#consuming-other-mcp-servers)).

## Invariants

- No tool reports `false`, "not found", or "no counterexample" for something
  outside its coverage or budget; it reports `unknown`.
- Only `supermath-mcp` sets `formalised` or `collected`, and only from a call
  it executed itself, whoever the caller is.
- A sub-agent's own account of its work is never a verdict. An `agent_*` tool
  re-runs `lean_check` on what the child produced and reports that.
- No memory scope comes from model output ([`agent-runner.md`](../agent-runner.md#memory-scopes)).
- Tool names, argument schemas, and error codes are the same over OpenHuman
  and over MCP.
- `supermath-mcp` depends on no OpenHuman crate and calls no model, except
  Gemini as the web search backend ([ADR 0005](../../adr/0005-search-the-web-with-gemini-grounding.md)).
- No web source raises a claim above `asserted`.

## Acceptance criteria

- Every tool has unit tests for its answer, each error code it can return,
  and its envelope's serde form.
- A test asserts the direct/deferred split and that every description's first
  sentence names a question.
- An end-to-end test drives `supermath-mcp serve` and gets the same envelope
  as the runner's host tool for the same arguments.
- `live_*` tests, gated by environment variables, exercise the real Lean
  service, arXiv, and CortexDB.

## Open questions

1. Should Jev rank by description only, or also by tags per area?
2. Does `ledger_record` belong among the direct tools, or only inside
   `agent_*` tools, so the top-level agent never writes claims itself?
3. Which tactic set is the default for `lean_try_tactics`, and is it per
   project?
