# Roadmap

## Shipped

- the agent scaffold on OpenHuman: the ladder as provider (`flash`, medium
  effort), Jev ranking `tool_search` through OpenRouter, a three-tool toolkit,
  and CortexDB memory scoped per project
- `docker compose up`: the ladder, CortexDB, and `supermath-agent check`
- CI: format, clippy, tests, per-file coverage, rustdoc, and the compose file

## Next

- design how the agent works on a problem, drawing on `docs/research/`
- grow the toolkit (Jacobian over MCP, a Lean checker, or a sandboxed Python)

## Out Of Scope

- anything that cannot be tested deterministically
- a provider client or agent loop of our own: those are OpenHuman's
