# Conjecture toolkit

- **Status:** Implemented
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
  literature, web research, Lean formalization and checking, computation
  with Python solvers, a claim ledger in CortexDB, and sub-agents for the
  long tasks.
- Lean is the arbiter. A claim is `formalised` only when this process ran the
  Lean kernel on it and saw no `sorry` and no axiom outside the allowlist.
- One crate per tool domain, each specified and tested on its own
  ([`crates.md`](crates.md)): files, Lean, Python, literature, web, ledger,
  calc, and sub-agents, over a small core contract crate.
  `supermath-mcp` composes them and serves them to any MCP client;
  `supermath-agent`, the OpenHuman runner, links the same composition.
- The project workspace holds Lean and Python side by side, and every tool
  that runs code runs a file in it ([`files/`](files/README.md)).
- Every tool follows the [contract](#tool-contract) below.

## Non-goals

- The solving loop: how the top-level agent sequences these tools, its
  prompt, and its final report. That is a separate spec.
- A proof search engine of our own. Search is Lean tactics, Mathlib search,
  and models on Surplus Intelligence.
- Accepting a result as published mathematics. A checked proof is evidence
  that a Lean statement holds, not that it says what the paper says; that is
  [alignment](ledger/README.md#alignment), a separate axis.

## Documents

| Spec | Crate | Covers |
| --- | --- | --- |
| [`crates.md`](crates.md) | all | the crate layout, dependency rules, per-crate testing, the runner, compose |
| [`tools.md`](tools.md) | all | the shape of a tool spec file |
| [`tool/`](tool/README.md) | `supermath-tool` | the `Tool` trait, envelope, budgets, and contract tests |
| [`files/`](files/README.md) | `supermath-files` | the project workspace: Lean and Python files, path rules, the write log |
| [`lean/`](lean/README.md) | `supermath-lean` | the pinned Lean service and the `lean_*` tools |
| [`python/`](python/README.md) | `supermath-python` | the pinned Python solver service and the `py_*` tools |
| [`literature/`](literature/README.md) | `supermath-literature` | arXiv, Semantic Scholar, OEIS: the `lit_*` tools |
| [`web/`](web/README.md) | `supermath-web` | Gemini with Google Search grounding: the `web_*` tools |
| [`ledger/`](ledger/README.md) | `supermath-ledger` | claims, facts, and edges in CortexDB: the `ledger_*` tools |
| [`calc/`](calc/README.md) | `supermath-calc` | small exact arithmetic |
| [`subagents/`](subagents/README.md) | `supermath-subagents` | the agentic `agent_*` tools |
| [`mcp/`](mcp/README.md) | `supermath-mcp` | composition, ports, and the MCP server |

## Tool contract

These rules come from morluto's REA and Jacobian
([`principles.md`](../../research/rea-and-jacobian/principles.md)). They are
what makes a tool cheap for an agent to choose, call, and believe.

### Names answer a question

A tool is named `<area>_<verb>_<object>` for the question it answers, not the
engine behind it: `lean_check`, `lit_search_arxiv`, `ledger_record`. Areas are
`file`, `lean`, `py`, `lit`, `web`, `ledger`, `calc`, and `agent`, one per
crate. A tool establishes exactly one
postcondition, stated in its description as "given X, returns Y such that P".
Workflows are composed by the agent or by an `agent_*` tool, never hidden in
a primitive.

### Primitive and agentic tools

Every tool is one of two kinds, declared by the tool and shown in the
[catalogue](#catalogue):

- **Primitive.** Code over a backend: a parser, an HTTP API, the Lean
  kernel, CortexDB. No model runs. The same arguments against the same
  backend state give the same answer, in roughly bounded time and cost.
- **Agentic.** It looks like a tool call, but behind it **a model takes a
  turn**: it decides what to search or which tools to call, possibly many
  times, and the tool returns what that turn produced. There are two
  families:
  - **`agentic (Gemini)`**, the `web_*` search tools: one or more Gemini
    turns with Google Search grounding, where Gemini chooses the queries
    ([`web/`](web/README.md)). Lives in `supermath-web`.
  - **`agentic (OpenHuman)`**, the `agent_*` tools: a full OpenHuman agent
    turn, on its own Surplus model, with its own prompt and its own subset
    of primitives, running up to dozens of tool calls
    ([`subagents/`](subagents/README.md)). Lives in `supermath-subagents`.

An agentic tool is a different thing to call, and the contract says so:

| | Primitive | Agentic |
| --- | --- | --- |
| Deterministic | yes, given backend state | no; the model chooses |
| Cost | backend time | model tokens, reported in `usage` |
| Latency | seconds | tens of seconds to minutes |
| Budget | admission caps | iterations, tokens, wall time; exhausting it is `unknown` |
| Description starts | "Given X, returns Y" | "Runs a <model> turn that …", so the caller knows it is delegating |
| Trust in its output | the backend's | none on its own; see below |
| May call | — | primitives only; never another agentic tool |

Rules for agentic tools:

- **Never the source of a verdict.** What a model turn says it did is a
  claim. An agentic tool reports verdicts and statuses only from
  primitives it ran *after* the turn ended: `agent_formalize` re-runs
  [`lean_check`](lean/tools/lean_check.md) on the files the child wrote; `web_*`
  tools keep only sentences that a grounding source supports.
- **Usage is part of the answer.** The envelope carries
  `usage: {model, turns, tool_calls, input_tokens, output_tokens,
  elapsed_ms}`, so the calling agent can account for what it spent.
- **No nesting.** A turn inside an agentic tool sees primitives only, so a
  call is one level of delegation, never a tree.
- **Progress is visible.** OpenHuman agentic tools forward their child's
  tool events to the caller's progress stream; Gemini tools report the
  queries Google ran in `coverage`.
- **Each tool file has a "Turn" section**: the model, the tools the turn
  may call, its budget, what is returned, and what is discarded.

### Progressive disclosure

Six tools are **direct**, always in the model's catalogue: `recall_memory`,
`lit_search_arxiv`, `web_search`, `lean_check`, `py_run`, and
`ledger_record`. Every other tool is
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
  nothing about the mathematics. A counterexample search that covers its
  whole stated frame and finds no witness is the one complete negative: it
  ends `ok` with `data.result: "exhausted"` and the frame, and means nothing
  outside that frame.
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
| `CONFLICT` | a file changed since the caller last saw it (`if_sha256` mismatch) |

Tests assert codes, never message text.

### Boundaries

- Arguments are typed JSON, parsed once at the boundary. No tool evaluates a
  string as code in the host. Lean source is sent to the Lean service, which
  is sandboxed ([`lean/`](lean/README.md#isolation)).
- No silent fallback. If a backend is missing, the tool says so; it never
  answers from a different engine than the one it names.
- Every call is admitted against a budget (time, size, rows) before it runs.
- Tools that run code (Lean and Python) take files in the project's
  workspace, never source as an argument; only `file_write` and
  `file_edit` put source on disk ([`files/`](files/README.md)).
- Provenance only where it carries trust: `lean_check` records the
  toolchain, Mathlib revision, file digest, and elapsed time; a literature
  row records its source and version. Arithmetic does not.

### Permissions

Every tool declares `Read` or `Write` honestly; the runner maps it to
OpenHuman's permission levels and the MCP server to `readOnlyHint`
([`crates.md`](crates.md)). `file_write`, `file_edit`, and the ledger
writes are `Write`.
`agent_*` tools are `Read` themselves, because the writing happens inside
the child agent, which has its own scope ([`subagents/`](subagents/README.md#scopes)).

## Claim status

Every claim the ledger holds has a status. Statuses only rise on evidence:

| Status | Means | Set by |
| --- | --- | --- |
| `asserted` | someone said it; the default | anyone |
| `cited` | a source says it, with a locator | a `lit_*` result |
| `checked` | it held on a stated finite range | a `calc_*` result with its frame |
| `formalised` | the Lean kernel accepted a statement of it, in this run | the ledger, from a `lean_check` this process ran |

`evidence_origin` is `collected` (a tool in this process produced it) or
`supplied` (it arrived as text). Only `collected` evidence may set `checked`
or `formalised`. A transcript someone pasted is a claim, not evidence.

Alignment, whether a Lean statement says what the informal statement says,
is a separate field: `not_reviewed`, `accepted`, or `disputed`
([`ledger/`](ledger/README.md#alignment)).

## Catalogue

Every tool, by crate. MCP clients get every tool except the `agent_*`
tools, which need OpenHuman ([`mcp/`](mcp/README.md)).

| Tool | Crate | Kind | Exposure | Permission | Phase | Given → returns |
| --- | --- | --- | --- | --- | --- | --- |
| [`file_edit`](files/tools/file_edit.md) | `supermath-files` | primitive | deferred | write | P1 | a file and exact replacements → the edited file's digest and changed lines |
| [`file_list`](files/tools/file_list.md) | `supermath-files` | primitive | deferred | read | P1 | the project, optionally a directory → its files with their last results |
| [`file_read`](files/tools/file_read.md) | `supermath-files` | primitive | deferred | read | P1 | a file and a line range → its numbered lines and digest, or its outline |
| [`file_write`](files/tools/file_write.md) | `supermath-files` | primitive | deferred | write | P1 | a path and its content → the file written, with its digest |
| [`calc_factor`](calc/tools/calc_factor.md) | `supermath-calc` | primitive | deferred | read | P1 | an integer → its factorization |
| [`calc_gcd_lcm`](calc/tools/calc_gcd_lcm.md) | `supermath-calc` | primitive | deferred | read | P1 | two integers → gcd and lcm |
| [`ledger_bound`](ledger/tools/ledger_bound.md) | `supermath-ledger` | primitive | deferred | write | P6 | a quantity and a bound → the best bound so far |
| [`ledger_claims`](ledger/tools/ledger_claims.md) | `supermath-ledger` | primitive | deferred | read | P2 | filters → paged claims |
| [`ledger_link`](ledger/tools/ledger_link.md) | `supermath-ledger` | primitive | deferred | write | P5 | two claim ids → a `uses` edge |
| [`ledger_record`](ledger/tools/ledger_record.md) | `supermath-ledger` | primitive | direct | write | P2 | a claim or fact → its id and status |
| [`recall_memory`](ledger/tools/recall_memory.md) | `supermath-ledger` | primitive | direct | read | P1 | a query → this project's notes |
| [`lean_blueprint`](lean/tools/lean_blueprint.md) | `supermath-lean` | primitive | deferred | read | P6 | the project → the declaration DAG with status |
| [`lean_build`](lean/tools/lean_build.md) | `supermath-lean` | primitive | deferred | read | P6 | the project package → per-file verdicts |
| [`lean_check`](lean/tools/lean_check.md) | `supermath-lean` | primitive | direct | read | P2 | a Lean file in the project → a [verdict](lean/README.md#verdicts) with provenance |
| [`lean_decl`](lean/tools/lean_decl.md) | `supermath-lean` | primitive | deferred | read | P2 | a declaration name → signature, docstring, module, without the proof |
| [`lean_goals`](lean/tools/lean_goals.md) | `supermath-lean` | primitive | deferred | read | P5 | a file and a position → the goal state there |
| [`lean_search`](lean/tools/lean_search.md) | `supermath-lean` | primitive | deferred | read | P2 | a name, pattern, or phrase → Mathlib declarations |
| [`lean_search_counterexample`](lean/tools/lean_search_counterexample.md) | `supermath-lean` | primitive | deferred | read | P6 | a decidable predicate declared in a project file, and a range → a witness, or `exhausted` over the whole range |
| [`lean_try_tactics`](lean/tools/lean_try_tactics.md) | `supermath-lean` | primitive | deferred | read | P5 | a file, one of its `sorry`s, and tactics → per-tactic outcome |
| [`py_run`](python/tools/py_run.md) | `supermath-python` | primitive | direct | read | P4 | a Python file, an entry point, and arguments → its returned value, output, and provenance |
| [`py_search_counterexample`](python/tools/py_search_counterexample.md) | `supermath-python` | primitive | deferred | read | P4 | a predicate function in a project file and a frame → a witness, or `exhausted` over the whole frame |
| [`py_solvers`](python/tools/py_solvers.md) | `supermath-python` | primitive | deferred | read | P4 | nothing, or an area → the solvers in the pinned environment, with versions |
| [`lit_citations`](literature/tools/lit_citations.md) | `supermath-literature` | primitive | deferred | read | P6 | an arXiv id → references or citing papers |
| [`lit_extract_statements`](literature/tools/lit_extract_statements.md) | `supermath-literature` | primitive | deferred | read | P3 | an arXiv id → numbered statements and their `\ref` edges |
| [`lit_get_paper`](literature/tools/lit_get_paper.md) | `supermath-literature` | primitive | deferred | read | P3 | an arXiv id → metadata, section index, a section's text |
| [`lit_oeis`](literature/tools/lit_oeis.md) | `supermath-literature` | primitive | deferred | read | P3 | terms or a query → matching sequences |
| [`lit_search_arxiv`](literature/tools/lit_search_arxiv.md) | `supermath-literature` | primitive | direct | read | P3 | a query → paged papers |
| [`web_read`](web/tools/web_read.md) | `supermath-web` | primitive | deferred | read | P3 | a URL → the page's main text, paged |
| [`web_research`](web/tools/web_research.md) | `supermath-web` | agentic (Gemini) | deferred | read | P3 | a research question → a sourced brief across several searches |
| [`web_search`](web/tools/web_search.md) | `supermath-web` | agentic (Gemini) | direct | read | P3 | a question → grounded findings with their sources |
| [`agent_digest_paper`](subagents/tools/agent_digest_paper.md) | `supermath-subagents` | agentic (OpenHuman) | deferred | read | P5 | an arXiv id → facts recorded from it |
| [`agent_formalize`](subagents/tools/agent_formalize.md) | `supermath-subagents` | agentic (OpenHuman) | deferred | read | P5 | a statement or paper locator → Lean files and their verdicts |
| [`agent_refute`](subagents/tools/agent_refute.md) | `supermath-subagents` | agentic (OpenHuman) | deferred | read | P6 | a statement → a counterexample, or a repaired statement |
| [`agent_review_alignment`](subagents/tools/agent_review_alignment.md) | `supermath-subagents` | agentic (OpenHuman) | deferred | read | P5 | a declaration and its informal statement → alignment |

`factor_integer` and `gcd_lcm` move to `supermath-calc` as `calc_factor`
and `calc_gcd_lcm` in P1. Jacobian's ~1,400 operations are reached over MCP, not
reimplemented ([`crates.md`](crates.md#consuming-other-mcp-servers)).

## Invariants

- No tool reports `false`, "not found", or "no counterexample" for something
  outside its coverage or budget; it reports `unknown`.
- Only the ledger sets `formalised`, and only `collected` evidence counts,
  from a call this process executed itself, whoever the caller is.
- A sub-agent's own account of its work is never a verdict. An `agent_*` tool
  re-runs `lean_check` on what the child produced and reports that.
- No memory scope comes from model output ([`agent-runner.md`](../agent-runner.md#memory-scopes)).
- Tool names, argument schemas, and error codes are the same over OpenHuman
  and over MCP.
- Only `supermath-subagents` and `supermath-agent` depend on OpenHuman, and
  no domain crate depends on another except through `supermath-files`
  ([`crates.md`](crates.md#dependency-rules)).
- Only `supermath-web` calls a model outside the sub-agents: Gemini, as
  its search backend ([ADR 0005](../../adr/0005-search-the-web-with-gemini-grounding.md)).
- A Python result can make a claim `checked` on a stated frame, never
  `formalised` ([`python/`](python/README.md#status-rules)).
- No web source raises a claim above `asserted`.

## Acceptance criteria

- Every domain crate passes the [contract checker](tool/README.md#contract-tests)
  over its tools, and its own tests run with `cargo test -p <crate>`.
- A test asserts the direct/deferred split and that every description's first
  sentence names a question.
- An end-to-end test drives `supermath-mcp serve` and gets the same envelope
  as the runner's host tool for the same arguments.
- `live_*` tests, gated by environment variables, exercise the real Lean
  service, the Python service, arXiv, Gemini, and CortexDB.

## Open questions

1. Should Jev rank by description only, or also by tags per area?
2. Does `ledger_record` belong among the direct tools, or only inside
   `agent_*` tools, so the top-level agent never writes claims itself?
3. Which tactic set is the default for `lean_try_tactics`, and is it per
   project?
