# 2. Build the solving harness on OpenHuman, in its own Cargo workspace

- **Status:** Proposed
- **Date:** 2026-10-22

## Context

The solving harness ([`../specs/solving-harness.md`](../specs/solving-harness.md))
needs an agent loop, a tool runtime, provider routing, and sessions.
[OpenHuman](https://github.com/tinyhumansai/openhuman) provides all of them
behind `openhuman-embed`, a library facade meant for exactly this kind of host.
math-superagent built its own loop on TinyAgents; OpenHuman is where that loop
is now maintained.

OpenHuman is not published to crates.io. Its graph resolves only with the
`[patch]` table in its root `Cargo.toml`, which points git dependencies at the
crates it vendors, and Cargo applies `[patch]` only at a workspace root. It
also vendors its own TinyBus, at a different revision from the one this
repository pins at `vendor/tinybus`; two path packages called `tinybus` cannot
share one lockfile.

## Decision

- Vendor OpenHuman as the `vendor/openhuman` git submodule, tracking `main`,
  and depend on `openhuman-embed` by path.
- Make `crates/supermath-agent` its own Cargo workspace (an empty
  `[workspace]` table in its manifest), excluded from the root workspace. Its
  manifest carries OpenHuman's `[patch]` table with paths re-rooted at
  `vendor/openhuman`, and its `Cargo.lock` starts from OpenHuman's.
- Keep the template crates (`template`, `template-bus`) and their CI as they
  are. Whether the harness becomes a TinyBus module is an open question in the
  spec.

## Consequences

- The repository has two lockfiles. The root rule "one lockfile" holds for
  the root workspace; the agent's lockfile is pinned to OpenHuman's.
- The root contract commands (`cargo fmt`, `clippy`, `build`, `test` from the
  root) do not build the agent. Its commands run from
  `crates/supermath-agent`, and CI needs a job for it.
- The patch table must be updated when the OpenHuman gitlink moves and
  OpenHuman's own table changes. Cargo warns on an unused patch, and a missing
  one fails resolution, so drift is loud.
- The agent requires Rust 1.96 or later, OpenHuman's minimum, while the
  template crates keep their own MSRV.
- A cold build compiles all of OpenHuman, which takes minutes. The Dockerfile
  caches the target directory between image builds.
