# Lean toolkit: implementation plan

- **Spec:** [`../specs/toolkit/README.md`](../specs/toolkit/README.md), with
  one file per tool in [`../specs/toolkit/tools/`](../specs/toolkit/tools/README.md)
- **Decisions:** [ADR 0003](../adr/0003-run-lean-as-a-pinned-service.md),
  [ADR 0004](../adr/0004-split-the-toolkit-into-an-mcp-crate.md),
  [ADR 0005](../adr/0005-search-the-web-with-gemini-grounding.md)
- **Status:** Not started

## Goal

Split the toolkit into the `supermath-mcp` crate and keep `supermath-agent`
as the OpenHuman runner ([`crates.md`](../specs/toolkit/crates.md)), then
grow the toolkit in phases. Each phase is a pull request or a short series
that leaves `main` green and both binaries usable. The split and the
envelope come first, because every tool reports through them and lives in
one of the two crates.

## Non-goals

- The solving loop and its prompt.
- Hosting a natural-language Mathlib index ourselves.

## Assumptions

- The ladder gains a `formalize` rung before P4; until then the formalizer
  runs on `flash`.
- CortexDB's label filtering is settled before P2's `ledger_claims`
  ([ledger open question 1](../specs/toolkit/ledger.md#open-questions)).

## Conventions

- Each tool is built against its file in `specs/toolkit/tools/`; a change to
  behavior updates that file in the same commit.
- Every test runs against loopback stubs (the pattern in `memory/stub.rs`
  and `tests/support/`), never the network. Live tests are named `live_*`
  and gated by an environment variable.
- `supermath-mcp` never depends on OpenHuman; a CI step runs
  `cargo tree -p supermath-mcp` and fails on any `openhuman` crate.

## P1: split the crates, envelope, MCP server

1. **Create `crates/supermath-mcp`**, inheriting the workspace fields and
   lints, with `config/` and `error/` of its own.
2. **Envelope.** `envelope/test.rs` first: the serde form of each outcome,
   that `error` requires a code, that a list over its cap is refused. Then
   `envelope/mod.rs`.
3. **`Tool` trait** in `tool/`, with `Exposure` and `Permission`; `Toolkit`
   in `toolkit/`.
4. **Move** `memory/` (the CortexDB client and its stub) from
   `supermath-agent` to `supermath-mcp`, and move `factor_integer` and
   `gcd_lcm` to `calc/` as `calc_factor` and `calc_gcd_lcm`, answering in
   the envelope. Move their tests with them.
5. **Host adapter.** `supermath-agent/src/host/` wraps each toolkit tool as
   an OpenHuman `Tool`. Test first: names, schemas, exposure, and
   permission pass through; `execute` returns the envelope.
6. **MCP server.** Add `rmcp` (server and stdio features only, with a
   comment). `server/` serves `tools/list` and `tools/call`.
   `crates/supermath-mcp/tests/serve.rs` performs the handshake, lists
   tools, and calls `calc_factor`.
7. **Binaries.** `supermath-mcp serve [project]` and `supermath-mcp check`;
   `supermath-agent check` keeps its output, now listing `calc_*` names.
   A `supermath-agent` test compares the adapted tools with the MCP
   server's list.
8. Update `AGENTS.md`'s project structure, both crate READMEs, `README.md`
   with the `claude mcp add` line, and the CI coverage gate to cover both
   crates.

## P2: Lean service, Lean files and check, ledger

1. **Lean service.** `docker/lean/Dockerfile`, `lakefile.toml`,
   `lean-toolchain`, and the HTTP wrapper (kimina-lean-server if the
   evaluation in ADR 0003 holds). Add the `lean` service to `compose.yaml`
   on an `internal: true` network the agent shares, so it has no egress,
   and `LEAN_URL` to `supermath-mcp`'s config and `.env.example`. Mount a
   `lean-projects` volume in both `lean` and `agent`, and add
   `SUPERMATH_LEAN_PROJECTS`.
2. **Verdicts.** Tests first in `lean/test.rs`, one per row of the
   [verdict table](../specs/toolkit/lean.md#verdicts), against a stub that
   replays recorded REPL answers. Then `lean/`.
3. **Files.** Tests first: the [path rules](../specs/toolkit/lean.md#files-not-source)
   (`..`, absolute paths, symlinks out, forbidden imports), atomic writes,
   `if_sha256` conflicts, an edit matching zero or two times, the write
   log. Then [`lean_write`](../specs/toolkit/tools/lean_write.md),
   [`lean_edit`](../specs/toolkit/tools/lean_edit.md),
   [`lean_read`](../specs/toolkit/tools/lean_read.md),
   [`lean_files`](../specs/toolkit/tools/lean_files.md). A test walks every
   `lean_*` schema and asserts no Lean-source argument outside these two
   write tools.
4. **Scope.** The write tools are denied under `HostOnly`, so move the
   top-level agent to `ToolScopeSpec::Named` here (a test pins the exact
   list) and hold the runtime as `Arc<Runtime>`.
5. **Tools:** [`lean_check`](../specs/toolkit/tools/lean_check.md) on files,
   cached by digest,
   [`lean_search`](../specs/toolkit/tools/lean_search.md) (name mode against
   the REPL; Loogle behind a stub),
   [`lean_decl`](../specs/toolkit/tools/lean_decl.md).
6. **Ledger.** Tests first: every allowed and refused transition, a caller
   sending `formalised` gets `FORBIDDEN`, a `lean` source is promoted only
   by the server's own check, a claim whose file changed falls to
   `asserted`, effective status over a chain. Then
   `ledger/`, [`ledger_record`](../specs/toolkit/tools/ledger_record.md) and
   [`ledger_claims`](../specs/toolkit/tools/ledger_claims.md).
7. Both `check` commands report the Lean service's toolchain and Mathlib
   revision. `live_lean_check`, gated by `LEAN_URL`.

## P3: literature and web

1. Record fixtures in `crates/supermath-mcp/tests/fixtures/literature/`: an
   Atom search feed, an e-print tarball with `\input` and `\newtheorem`
   variants, a PDF-only paper, an OEIS answer, a 429.
2. Rate limiter and cache, tested with a fake clock.
3. [`lit_search_arxiv`](../specs/toolkit/tools/lit_search_arxiv.md),
   [`lit_get_paper`](../specs/toolkit/tools/lit_get_paper.md),
   [`lit_extract_statements`](../specs/toolkit/tools/lit_extract_statements.md),
   [`lit_oeis`](../specs/toolkit/tools/lit_oeis.md), each test-first.
4. `live_arxiv_search`, gated by `SUPERMATH_LIVE_ARXIV`.
5. **Web.** Record Gemini fixtures in
   `crates/supermath-mcp/tests/fixtures/web/`: a grounded answer, one with
   unsupported sentences, an ungrounded answer, a 429, a safety block; and
   a loopback redirect chain. Then `web/`: the Gemini client, redirect
   resolution, source classes, the daily cap, and the cache.
6. [`web_search`](../specs/toolkit/tools/web_search.md),
   [`web_research`](../specs/toolkit/tools/web_research.md),
   [`web_read`](../specs/toolkit/tools/web_read.md), each test-first. Add
   `GEMINI_API_KEY` and `SUPERMATH_WEB_MODEL` to `.env.example` and pass
   them to the agent in `compose.yaml`.
7. `live_web_search`, gated by `GEMINI_API_KEY` and `SUPERMATH_LIVE_WEB`.

## P4: goal tools and sub-agents

1. In `supermath-mcp`: [`lean_goals`](../specs/toolkit/tools/lean_goals.md),
   [`lean_try_tactics`](../specs/toolkit/tools/lean_try_tactics.md),
   [`ledger_link`](../specs/toolkit/tools/ledger_link.md), test-first.
2. **Agentic tools.** The `Kind` on the `Tool` trait, the `usage` field in
   the envelope, and a test that every child's tool list holds primitives
   only.
3. `subagents/`: child specs with their models (`SUPERMATH_*_MODEL`),
   scopes, and budgets, tested against a scripted provider
   (`Provider::custom`) that plays a fixed sequence of tool calls.
4. [`agent_digest_paper`](../specs/toolkit/tools/agent_digest_paper.md),
   [`agent_formalize`](../specs/toolkit/tools/agent_formalize.md),
   [`agent_review_alignment`](../specs/toolkit/tools/agent_review_alignment.md).
   Each has a test where the scripted child claims success and the Lean
   stub says otherwise; the tool must report the stub's verdict.
5. `bench/formalize/` with 30 statements and `live_formalize_bench`.

## P5: compute, refutation, blueprint

1. Enable `openhuman-embed`'s `mcp` feature; connect Jacobian and
   lean-lsp-mcp when their variables are set; `supermath-agent check`
   lists them.
2. [`calc_search_counterexample`](../specs/toolkit/tools/calc_search_counterexample.md)
   and [`agent_refute`](../specs/toolkit/tools/agent_refute.md).
3. [`ledger_bound`](../specs/toolkit/tools/ledger_bound.md),
   [`lit_citations`](../specs/toolkit/tools/lit_citations.md).
4. [`lean_build`](../specs/toolkit/tools/lean_build.md) and
   [`lean_blueprint`](../specs/toolkit/tools/lean_blueprint.md).

## Verification, every phase

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
.github/scripts/check-file-coverage.sh 90 coverage.json
docker compose config --quiet
```

Plus `cargo test -p supermath-mcp` alone, which must not compile
OpenHuman, and, for a phase that changes the stack, `docker compose up
--build` with `supermath-agent check` exiting 0.

## Checklist

- [ ] P1: `supermath-mcp` crate, envelope, `Tool` trait, memory and calc
      moved, host adapter, `supermath-mcp serve`
- [ ] P2: Lean service, `lean_write`, `lean_edit`, `lean_read`,
      `lean_files`, `lean_check`, `lean_search`, `lean_decl`, `Named`
      scope, `ledger_record`, `ledger_claims`
- [ ] P3: `lit_search_arxiv`, `lit_get_paper`, `lit_extract_statements`,
      `lit_oeis`, `web_search`, `web_research`, `web_read`
- [ ] P4: goal and tactic tools, agentic-tool contract, sub-agents,
      formalize bench
- [ ] P5: Jacobian and lean-lsp-mcp, counterexample search, `agent_refute`,
      bounds, citations, blueprint
- [ ] Specs moved to Implemented; `agent-runner.md` toolkit table updated
