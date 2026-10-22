# 2. Build the agent on OpenHuman, routed through the LLM ladder

- **Status:** Proposed; the ladder routing is superseded by
  [ADR 0008](0008-call-surplus-intelligence-directly.md)
- **Date:** 2026-10-22

## Context

supermath needs an agent loop, a tool runtime, provider routing, and
sessions. [OpenHuman](https://github.com/tinyhumansai/openhuman) provides all
of them behind `openhuman-embed`, a library facade meant for exactly this kind
of host. math-superagent built its own loop on TinyAgents; OpenHuman is where
that loop is now maintained.

The repository started from the TinyBus module template. supermath is not a
TinyBus module, and the template's bus crates and release workflow did not
apply.

OpenHuman is not published to crates.io. Its graph resolves only with the
`[patch]` table in its root `Cargo.toml`, and Cargo applies `[patch]` only at
a workspace root.

## Decision

- Remove the TinyBus template: `crates/template`, `crates/template-bus`, the
  `vendor/tinybus` submodule, and the module release workflow.
- Vendor OpenHuman as the `vendor/openhuman` git submodule, tracking `main`,
  and depend on `openhuman-embed` by path. Take `openhuman-tinyhumans` with
  only its `jev` feature, for the Jev `tool_search` ranker; nothing hosted is
  installed.
- Carry OpenHuman's `[patch]` table in the root `Cargo.toml`, with paths
  re-rooted at `vendor/openhuman`, and start `Cargo.lock` from OpenHuman's.
- Route every chat completion and embedding through the LLM ladder; call
  OpenRouter directly only for Jev.

## Consequences

- One workspace and one lockfile, pinned close to OpenHuman's.
- The patch table must move with the OpenHuman gitlink. Cargo warns on an
  unused patch and fails on a missing one, so drift is loud.
- Rust 1.96 or later, OpenHuman's minimum.
- A cold build compiles all of OpenHuman, which takes minutes. The Dockerfile
  caches the target directory between image builds.
- Model choice and price live in the ladder's config, not here: `flash` and
  `vectors` are ladder names.
