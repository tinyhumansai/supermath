# Roadmap

## Shipped

- the agent scaffold on OpenHuman: the ladder as provider (`flash`, medium
  effort), Jev ranking `tool_search` through OpenRouter, a three-tool toolkit,
  and CortexDB memory scoped per project
- `docker compose up`: the ladder, CortexDB, and `supermath-agent check`
- CI: format, clippy, tests, per-file coverage, rustdoc, and the compose file

## Next

- the conjecture toolkit, in five phases
  ([spec](docs/specs/toolkit/README.md), [plan](docs/plans/lean-toolkit.md)):
  the toolkit split into its own `supermath-mcp` crate and MCP server, with
  `supermath-agent` as the OpenHuman runner; a pinned Lean service and
  `lean_check`; a claim ledger in CortexDB; arXiv and OEIS; sub-agents that
  digest papers and formalize statements in Lean; Jacobian, counterexample
  search, and refutation
- design how the agent works on a problem, drawing on `docs/research/`

## Out Of Scope

- anything that cannot be tested deterministically
- a provider client or agent loop of our own: those are OpenHuman's
