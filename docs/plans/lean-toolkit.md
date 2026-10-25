# Conjecture toolkit: implementation plan

- **Spec:** [`../specs/toolkit/README.md`](../specs/toolkit/README.md); one
  folder per crate, each with one file per tool
- **Decisions:** [ADR 0003](../adr/0003-run-lean-as-a-pinned-service.md),
  [ADR 0004](../adr/0004-split-the-toolkit-into-an-mcp-crate.md),
  [ADR 0005](../adr/0005-search-the-web-with-gemini-grounding.md),
  [ADR 0006](../adr/0006-one-crate-per-tool-domain.md),
  [ADR 0007](../adr/0007-run-python-solvers-as-a-pinned-service.md)
- **Status:** Implemented; the live tests (Lean image, Gemini, formalize bench) are not yet run

> **Note.** This plan is kept as written. `supermath-agent`, `supermath-subagents`
> (the `agent_*` tools), the formalize bench test, and the OpenHuman MCP
> connection of P6 were later removed by
> [ADR 0010](../adr/0010-run-on-rsi-ralph.md); the agents now run in
> rsi-ralph ([`loops.md`](../specs/loops.md)). The specs it names for them
> (`agent-runner.md`, `toolkit/subagents/`) no longer exist.

## Goal

Build the toolkit as one crate per tool domain
([`crates.md`](../specs/toolkit/crates.md)), each specified, tested, and
gated on its own, composed by `supermath-mcp` and linked by the
`supermath-agent` runner. Each phase is a pull request or a short series
that leaves `main` green and both binaries usable. Phases add crates; a
crate, once added, is complete for the tools its phase lists.

## Non-goals

- The solving loop and its prompt.
- Hosting a natural-language Mathlib index ourselves.
- SageMath (a separate image and ADR if it is needed).

## Assumptions

- The ladder gains a `formalize` rung before P5; until then the formalizer
  runs on `flash`.
- CortexDB's label filtering is settled before P2's `ledger_claims`
  ([ledger open question 1](../specs/toolkit/ledger/README.md#open-questions)).

## Conventions, every crate

- The crate's spec folder is the contract: build each tool against its
  file, and change the file in the same commit as the behavior.
- Tests first. Unit tests in `src/<module>/test.rs`; integration tests in
  the crate's `tests/`, against loopback stubs of its backend and fakes of
  its ports; never the network. Live tests are `live_<domain>_*`, gated by
  environment variables.
- Every domain crate runs the
  [contract checker](../specs/toolkit/tool/README.md#contract-tests) over
  its tools.
- `cargo test -p <crate>` passes alone, and at least 90% line coverage per
  file.
- New variables go in `.env.example` and the crate's spec in the same
  commit.

## P1: the contract, the workspace, the composition

1. **`supermath-tool`.** Tests first for the envelope's serde form, `error`
   requiring a code, list caps, `Budget::admit`, and the contract checker
   against a passing and a failing fake tool. Then the crate.
2. **Dependency rules.** `scripts/check-crate-deps.sh` over `cargo
   metadata`, with fixture workspaces that break each rule; wired into CI.
3. **`supermath-files`.** The [path rules](../specs/toolkit/files/README.md#path-rules)
   for Lean and Python, atomic guarded writes, the write log, `FileObserver`;
   then [`file_write`](../specs/toolkit/files/tools/file_write.md),
   [`file_edit`](../specs/toolkit/files/tools/file_edit.md),
   [`file_read`](../specs/toolkit/files/tools/file_read.md),
   [`file_list`](../specs/toolkit/files/tools/file_list.md).
4. **`supermath-calc`.** Move `factor_integer` and `gcd_lcm` from
   `supermath-agent` as [`calc_factor`](../specs/toolkit/calc/tools/calc_factor.md)
   and [`calc_gcd_lcm`](../specs/toolkit/calc/tools/calc_gcd_lcm.md), with
   their tests.
5. **`supermath-ledger`, first part.** Move the CortexDB client and its stub
   from `supermath-agent`, and [`recall_memory`](../specs/toolkit/ledger/tools/recall_memory.md)
   with it.
6. **`supermath-mcp`.** `Toolkit::compose`, the call log, and `serve` over
   stdio with `rmcp`; `tests/serve.rs` does the handshake, lists tools, and
   calls `calc_factor` and `file_write`. `supermath-mcp check`.
7. **`supermath-agent`.** The host adapter; the top-level agent moves to
   `ToolScopeSpec::Named` (the file tools are `Write`, which `HostOnly`
   denies), with a test pinning the list; the runtime as `Arc<Runtime>`.
   `check` lists the composed tools.
8. Update `AGENTS.md`'s project structure, every crate's `README.md`,
   `README.md` with the `claude mcp add` line, and the CI coverage gate to
   cover every crate.

## P2: Lean and the ledger's evidence

1. **Lean service.** `docker/lean/` (toolchain, Mathlib pin, cache, and the
   HTTP wrapper; kimina-lean-server if ADR 0003's evaluation holds). Add
   `lean` to `compose.yaml` on an `internal: true` network, mount the
   `projects` volume, add `LEAN_URL`.
2. **`supermath-lean`.** Tests first, one per row of the
   [verdict table](../specs/toolkit/lean/README.md#verdicts), against a stub
   replaying recorded REPL answers; staleness by digest; the observer. Then
   [`lean_check`](../specs/toolkit/lean/tools/lean_check.md),
   [`lean_search`](../specs/toolkit/lean/tools/lean_search.md),
   [`lean_decl`](../specs/toolkit/lean/tools/lean_decl.md).
3. **`supermath-ledger`, second part.** Tests first, against a fake
   `LeanEvidence`: every allowed and refused transition, a caller sending
   `formalised` gets `FORBIDDEN`, a changed file drops its claim to
   `asserted`, effective status over a chain. Then
   [`ledger_record`](../specs/toolkit/ledger/tools/ledger_record.md) and
   [`ledger_claims`](../specs/toolkit/ledger/tools/ledger_claims.md).
4. Wire `LeanEvidence` and the Lean observer in `supermath-mcp`, with a
   composition test. Both `check` commands report the Lean toolchain and
   Mathlib revision. `live_lean_check`.

## P3: literature and web

1. **`supermath-literature`.** Fixtures (an Atom feed, an e-print tarball
   with `\input` and `\newtheorem` variants, a PDF-only paper, an OEIS
   answer, a 429); the rate limiter on a fake clock; then
   [`lit_search_arxiv`](../specs/toolkit/literature/tools/lit_search_arxiv.md),
   [`lit_get_paper`](../specs/toolkit/literature/tools/lit_get_paper.md),
   [`lit_extract_statements`](../specs/toolkit/literature/tools/lit_extract_statements.md),
   [`lit_oeis`](../specs/toolkit/literature/tools/lit_oeis.md).
   `live_literature_arxiv`.
2. **`supermath-web`.** Gemini fixtures (grounded, unsupported sentences,
   ungrounded, 429, safety block) and a loopback redirect chain; source
   classes; the daily cap and cache; then
   [`web_search`](../specs/toolkit/web/tools/web_search.md),
   [`web_research`](../specs/toolkit/web/tools/web_research.md),
   [`web_read`](../specs/toolkit/web/tools/web_read.md). `GEMINI_API_KEY`
   and `SUPERMATH_WEB_MODEL` in `.env.example` and `compose.yaml`.
   `live_web_search`.

## P4: Python solvers

1. **Python service.** `docker/python/` with `pyproject.toml`, `uv.lock`,
   `solvers.toml`, and the run server; `python` in `compose.yaml` on the
   internal network, mounting `projects` read-only; `PYTHON_URL`.
2. **`supermath-python`.** Tests first for every
   [outcome](../specs/toolkit/python/README.md#outcomes), provenance, and
   the observer, against a stub replaying recorded runs. Then
   [`py_run`](../specs/toolkit/python/tools/py_run.md),
   [`py_search_counterexample`](../specs/toolkit/python/tools/py_search_counterexample.md),
   [`py_solvers`](../specs/toolkit/python/tools/py_solvers.md).
3. Wire the Python observer and record `py_run` calls in the call log, so
   a run can be a ledger `tool` source; a ledger test that a Python run
   reaches `checked` and never `formalised`.
4. `live_python_run` and `live_python_isolation` (no socket, no write to
   the project, killed at its timeout).

## P5: proof work and sub-agents

1. [`lean_goals`](../specs/toolkit/lean/tools/lean_goals.md),
   [`lean_try_tactics`](../specs/toolkit/lean/tools/lean_try_tactics.md),
   [`ledger_link`](../specs/toolkit/ledger/tools/ledger_link.md),
   test-first in their crates.
2. **`supermath-subagents`.** Child specs with their models
   (`SUPERMATH_*_MODEL`), `Named` scopes built from the registry, budgets,
   and `usage`; a test that every child's list holds primitives only.
   Tests run against a scripted provider (`Provider::custom`) and fake
   primitives.
3. `agent_digest_paper`,
   `agent_formalize`,
   `agent_review_alignment`.
   Each has a test where the scripted child claims success and the fake
   `lean_check` says otherwise; the tool must report the fake's verdict.
4. `supermath-agent` registers the sub-agents. `bench/formalize/` with 30
   statements and `live_subagents_formalize_bench`.

## P6: refutation, blueprint, and outside servers

1. [`lean_search_counterexample`](../specs/toolkit/lean/tools/lean_search_counterexample.md),
   then `agent_refute`
   over the Python and Lean searches.
2. [`ledger_bound`](../specs/toolkit/ledger/tools/ledger_bound.md),
   [`lit_citations`](../specs/toolkit/literature/tools/lit_citations.md).
3. [`lean_build`](../specs/toolkit/lean/tools/lean_build.md),
   [`lean_blueprint`](../specs/toolkit/lean/tools/lean_blueprint.md).
4. Enable `openhuman-embed`'s `mcp` feature in `supermath-agent`; connect
   Jacobian and lean-lsp-mcp when their variables are set.

## Verification, every phase

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
scripts/check-crate-deps.sh
.github/scripts/check-file-coverage.sh 90 coverage.json
docker compose config --quiet
```

Plus `cargo test -p <crate>` for each crate the phase touched, and, for a
phase that changes the stack, `docker compose up --build` with
`supermath-agent check` exiting 0.

## Checklist

- [x] P1: `supermath-tool`, dependency rules, `supermath-files`,
      `supermath-calc`, ledger memory move, `supermath-mcp serve`, host
      adapter and `Named` scope
- [x] P2: Lean service, `supermath-lean` (`lean_check`, `lean_search`,
      `lean_decl`), `ledger_record`, `ledger_claims`
- [x] P3: `supermath-literature`, `supermath-web`
- [x] P4: Python service, `supermath-python`
- [x] P5: goal and tactic tools, `ledger_link`, `supermath-subagents`,
      formalize bench
- [x] P6: counterexample search in Lean, `agent_refute`, bounds,
      citations, build, blueprint, Jacobian and lean-lsp-mcp
- [x] Specs moved to Implemented; `agent-runner.md` toolkit table updated
