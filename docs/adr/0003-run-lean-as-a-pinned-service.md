# 3. Run Lean as a pinned service

- **Status:** Proposed
- **Date:** 2026-10-22

## Context

The toolkit makes Lean the arbiter of what is proved
([`specs/toolkit/`](../specs/toolkit/README.md)). A `formalised` claim must
mean the same thing tomorrow as today, so the toolchain and Mathlib revision
behind every verdict have to be fixed and recorded. A check must also be
fast enough to call dozens of times per turn, which rules out elaborating
`import Mathlib` from scratch on each call.

Lean source written by a model is untrusted: elaboration can run code
through `#eval` and macros, and can loop.

Three ways to run it were considered:

- **The operator's own `elan` and `lake`.** No container, but no pin: two
  operators get two verdicts, and a check runs with the operator's
  privileges.
- **[lean-lsp-mcp](https://github.com/oOo0oOo/lean-lsp-mcp) over MCP.**
  Little code, but the verdict would be our parse of a language server's
  diagnostics, and its process runs wherever the agent runs.
- **A service of our own, in compose.** A pinned image with Mathlib's cache
  prefetched and warm
  [Lean REPL](https://github.com/leanprover-community/repl) processes behind
  HTTP.

Jacobian removed its own `lean.check` in 0.14.0 and advises keeping Lean in
a separate server the caller composes
([`research/rea-and-jacobian/projects.md`](../research/rea-and-jacobian/projects.md)).

## Decision

- Run Lean as the `lean` compose service, built from `docker/lean/`, with one
  `lean-toolchain` and one Mathlib revision pinned in its Lake manifest and
  the Mathlib cache fetched at build time.
- Serve checks through pooled REPLs with `import Mathlib` preloaded, behind
  a small HTTP API. Evaluate
  [kimina-lean-server](https://github.com/project-numina/kimina-lean-server)
  first; write our own only if it cannot report what the verdicts need
  (messages, sorries, proof states, `#print axioms`).
- Isolate the service: no network at runtime, an unprivileged user, a
  read-only root except the project volume, and per-request limits on time,
  memory, and output.
- Record the toolchain, Mathlib revision, and source digest with every
  verdict. Move the Mathlib pin only in its own commit, and re-check every
  `formalised` claim when it moves.
- lean-lsp-mcp may be connected for interactive exploration, but never sets
  a status.

## Evaluation of kimina-lean-server

[kimina-lean-server](https://github.com/project-numina/kimina-lean-server)
already pools REPLs behind REST and was considered first, as above. It was
assessed from its documented API, not run. We wrote our own small wrapper
(`docker/lean/server.py`, standard library only) because the verdicts need
things its documented API does not return in one call: the
axioms of named targets after elaboration in the same environment, `sorry`
goals with positions, a tactic run against a given `sorry`'s proof state,
and a range search in chunks with the largest value covered. Wrapping
Kimina would have meant a second layer of calls per check, a heavier image,
and an API of its own to track. The wrapper is about 350 lines over the
REPL's JSON protocol; if it outgrows that, revisit adopting Kimina's pool
and keep only the verdict-specific commands. The API is specified in
[`specs/toolkit/lean/README.md`](../specs/toolkit/lean/README.md#the-http-api).

## Consequences

- Verdicts are reproducible across operators and over time.
- The image is large (Mathlib's cache is several gigabytes) and its first
  build is slow; CI builds it only when `docker/lean/` changes.
- A Mathlib bump is a deliberate, reviewable event with a re-check, not
  drift.
- The agent depends on one more service; when it is down, `lean_*` tools
  return `BACKEND_UNAVAILABLE` and nothing is checked.
