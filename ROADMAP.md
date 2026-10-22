# Roadmap

## Shipped

- the agent scaffold on OpenHuman: Surplus Intelligence as provider
  (`deepseek-v4.1-flash`, medium effort), Jev ranking `tool_search` through OpenRouter, a three-tool toolkit,
  and CortexDB memory scoped per project
- `docker compose up`: CortexDB and `supermath-agent check`
- CI: format, clippy, tests, per-file coverage, rustdoc, and the compose file

## Next

- the conjecture toolkit, one crate per tool domain, in six phases
  ([spec](docs/specs/toolkit/README.md), [plan](docs/plans/lean-toolkit.md)):
  the tool contract, the project workspace, and the MCP server; a pinned
  Lean service and the ledger's evidence; arXiv, OEIS, and web research
  through Gemini with Google Search grounding; a pinned Python solver
  service; sub-agents that digest papers and formalize statements in Lean;
  counterexample search, refutation, and the blueprint
- design how the agent works on a problem, drawing on `docs/research/`

## Out Of Scope

- anything that cannot be tested deterministically
- a provider client or agent loop of our own: those are OpenHuman's
