# Roadmap

## Shipped

- the conjecture toolkit, one crate per tool domain
  ([spec](docs/specs/toolkit/README.md), [plan](docs/plans/lean-toolkit.md)),
  served over MCP and as one process per call
- conjectures as rsi-ralph loops ([spec](docs/specs/loops.md),
  [ADR 0010](docs/adr/0010-run-on-rsi-ralph.md)): `loop new`, the
  `score proof` scorer, and prover, refuter and lead seats on Surplus
- `docker compose up`: CortexDB and the pinned Lean and Python services
- CI: format, clippy, tests, per-file coverage, rustdoc, and the compose file

## Next

- a meta-loop that tunes the loop template's seats and brief across problems,
  on the cases in `bench/formalize/`
- seats for the roles the removed `agent_*` tools played: a paper reader, a
  formalizer, and an alignment reviewer on a different model

## Out Of Scope

- anything that cannot be tested deterministically
- a provider client or agent loop of our own: those are rsi-ralph's
