# 6. One crate per tool domain

- **Status:** Proposed
- **Date:** 2026-10-22

## Context

[ADR 0004](0004-split-the-toolkit-into-an-mcp-crate.md) put every primitive
tool in one crate, `supermath-mcp`. The toolkit has since grown to eight
domains (files, Lean, Python, literature, web, ledger, arithmetic, and
sub-agents), each with its own backend service, configuration, failure
modes, and live tests. In one crate, a change to the web client rebuilds
and retests the Lean client, one domain's dependencies are every domain's,
and nothing stops one domain from reaching into another's internals.

## Decision

- Give each tool domain its own crate, with its own spec folder under
  `docs/specs/toolkit/<domain>/` (a `README.md` and one file per tool), its
  own tests, and its own coverage gate: `supermath-files`,
  `supermath-lean`, `supermath-python`, `supermath-literature`,
  `supermath-web`, `supermath-ledger`, `supermath-calc`, and
  `supermath-subagents`.
- Put the shared contract in a small core crate, `supermath-tool`: the
  `Tool` trait, the envelope, error codes, kinds, budgets, the call log, the
  tool registry, and a contract checker every domain runs over its tools.
- Forbid dependencies between domain crates, except that `lean` and
  `python` depend on `supermath-files`. Where one domain needs another's
  evidence, it defines a port (a trait), and the composition wires it.
- Keep `supermath-mcp` as the composition and the MCP server, owning no
  tools, and `supermath-agent` as the OpenHuman runner.
- Move `agent_*` tools into `supermath-subagents`, which reaches primitives
  only through the registry it is handed.
- Enforce the dependency rules in CI with a script over `cargo metadata`.

This supersedes ADR 0004's placement of the tools in `supermath-mcp`; its
other decisions (serve over MCP with `rmcp`, link the toolkit in process
in the runner, move the top-level agent to a `Named` scope) stand.

## Consequences

- Each domain is built, tested, and reviewed on its own:
  `cargo test -p supermath-lean` compiles only what Lean needs.
- Ports make cross-domain trust explicit: the ledger can be tested with a
  fake Lean checker, and the real one is wired in exactly one place.
- More crates to keep in step: each inherits the workspace's package
  fields and lints, and the root `Cargo.toml` lists none of them by hand,
  since `members = ["crates/*"]` picks them up.
- A tool's name prefix names its crate, which the contract checker
  enforces.
