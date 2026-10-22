# Lean toolkit: implementation plan

- **Spec:** [`../specs/toolkit/README.md`](../specs/toolkit/README.md)
- **Decisions:** [ADR 0003](../adr/0003-run-lean-as-a-pinned-service.md),
  [ADR 0004](../adr/0004-serve-the-toolkit-over-mcp.md)
- **Status:** Not started

## Goal

Deliver the toolkit in five phases, each a pull request or a short series,
each leaving `main` green and the agent usable. Phase order follows what
the later phases rest on: the envelope and the Lean verdict first, because
every other tool reports through them.

## Non-goals

- The solving loop and its prompt.
- Hosting a natural-language Mathlib index ourselves.

## Assumptions

- The ladder gains a `formalize` rung before P3; until then the formalizer
  runs on `flash`.
- CortexDB's label filtering is settled before P1's `ledger_claims`
  ([ledger open question 1](../specs/toolkit/ledger.md#open-questions)).

## Module layout

New feature modules in `crates/supermath-agent/src/`, each with `mod.rs`,
`types.rs` where types are substantial, and `test.rs`:

```text
envelope/     # Outcome, Envelope, ErrorCode, Coverage; serde pinned
lean/         # the Lean service client and verdict reduction
literature/   # arXiv, Semantic Scholar, OEIS clients; LaTeX flattening
ledger/       # entries, status transitions, effective status
subagents/    # child AgentSpecs, budgets, re-check of child output
mcp/          # the rmcp server over Toolkit
toolkit/      # one file per area: calc.rs, lean.rs, lit.rs, ledger.rs, agent.rs
```

Every test runs against loopback stubs (the pattern in `memory/stub.rs` and
`tests/support/`), never the network. Live tests are named `live_*` and
gated by an environment variable.

## P1: envelope, Lean check, ledger

1. **Envelope.** Write `envelope/test.rs` first: the serde form of each
   outcome, that `error` requires a code, that a list over its cap is
   refused. Then `envelope/mod.rs`.
2. **Rename** `factor_integer` and `gcd_lcm` to `calc_factor` and
   `calc_gcd_lcm`, moving them to `toolkit/calc.rs` and answering in the
   envelope. Update the integration test's expected `check` output.
3. **Lean service.** `docker/lean/Dockerfile`, `lakefile.toml`,
   `lean-toolchain`, and the HTTP wrapper (kimina-lean-server if the
   evaluation in ADR 0003 holds). Add the `lean` service to `compose.yaml`
   on an `internal: true` network the agent shares, so it has no egress,
   and add `LEAN_URL` to `Config` and `.env.example`.
4. **Verdicts.** Tests first in `lean/test.rs`, one per row of the verdict
   table in [`lean.md`](../specs/toolkit/lean.md#verdicts), against a stub
   that replays recorded REPL answers. Then `lean/mod.rs`.
5. **Tools.** `lean_check`, `lean_search` (name mode against the REPL;
   Loogle behind a stub), `lean_decl`, in `toolkit/lean.rs`.
6. **Ledger.** Tests first: every allowed and refused transition, a model
   sending `formalised` gets `FORBIDDEN`, effective status over a chain.
   Then `ledger/`, and `ledger_record` and `ledger_claims` in
   `toolkit/ledger.rs`.
7. **Check.** `supermath-agent check` reports the Lean service's toolchain
   and Mathlib revision.
8. `live_lean_check`, gated by `LEAN_URL`.

## P2: literature

1. Record fixtures in `crates/supermath-agent/tests/fixtures/literature/`:
   an Atom search feed, an e-print tarball with `\input` and `\newtheorem`
   variants, a PDF-only paper, an OEIS answer, a 429.
2. Rate limiter and cache, tested with a fake clock.
3. `lit_search_arxiv`, `lit_get_paper`, `lit_extract_statements`,
   `lit_oeis`, each test-first against the fixtures.
4. `live_arxiv_search`, gated by `SUPERMATH_LIVE_ARXIV`.

## P3: sub-agents

1. Move the top-level agent to `ToolScopeSpec::Named`; a test pins the
   exact list. Hold the runtime as `Arc<Runtime>`.
2. `lean_goals`, `lean_try_tactics`, `lean_write`, `ledger_link`, test-first.
3. `subagents/`: build child specs with their models
   (`SUPERMATH_*_MODEL`), scopes, and budgets. Test against a scripted
   provider (`Provider::custom`) that plays a fixed sequence of tool calls.
4. `agent_digest_paper`, `agent_formalize`, `agent_review_alignment`. Each
   has a test where the scripted child claims success and the Lean stub
   says otherwise; the tool must report the stub's verdict.
5. `bench/formalize/` with 30 statements and `live_formalize_bench`.

## P4: MCP server

1. Add `rmcp` with a comment, server and stdio features only.
2. `mcp/`: `tools/list` and `tools/call` over `Toolkit`.
3. `tests/mcp.rs`: handshake, list, call `calc_factor`, compare to the host
   tool's envelope.
4. `supermath-agent mcp [project]`, documented in `README.md` with the
   `claude mcp add` line.

## P5: compute, refutation, blueprint

1. Enable `openhuman-embed`'s `mcp` feature; connect Jacobian and
   lean-lsp-mcp when their variables are set; `check` lists them.
2. `calc_search_counterexample`, over a decidable Lean predicate through the
   Lean service.
3. `agent_refute`, `ledger_bound`, `lit_citations`.
4. `lean_build` and `lean_blueprint`.

## Verification, every phase

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
.github/scripts/check-file-coverage.sh 90 coverage.json
docker compose config --quiet
```

Plus, for a phase that changes the stack, `docker compose up --build` with
`supermath-agent check` exiting 0.

## Checklist

- [ ] P1: envelope, renamed calc tools, Lean service, `lean_check`,
      `lean_search`, `lean_decl`, `ledger_record`, `ledger_claims`
- [ ] P2: `lit_search_arxiv`, `lit_get_paper`, `lit_extract_statements`,
      `lit_oeis`
- [ ] P3: `Named` scope, goal and tactic tools, `lean_write`, sub-agents,
      formalize bench
- [ ] P4: `supermath-agent mcp`
- [ ] P5: Jacobian and lean-lsp-mcp, counterexample search, `agent_refute`,
      bounds, citations, blueprint
- [ ] Specs moved to Implemented; `agent-runner.md` toolkit table updated
