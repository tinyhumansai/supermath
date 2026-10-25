# 10. Run the agents in rsi-ralph, not in this repository

- **Status:** Proposed
- **Date:** 2026-10-25
- **Supersedes:** [ADR 0002](0002-build-on-openhuman.md)
- **Amends:** [0004](0004-split-the-toolkit-into-an-mcp-crate.md),
  [0006](0006-one-crate-per-tool-domain.md),
  [0008](0008-call-surplus-intelligence-directly.md),
  [0009](0009-settle-conjectures-by-a-prover-refuter-duel.md)
- **Spec:** [`specs/loops.md`](../specs/loops.md)

## Context

[ADR 0002](0002-build-on-openhuman.md) embedded OpenHuman in two crates:
- `supermath-agent`, the runner: a turn, then a continue prompt, until the
  model wrote `STATUS: DONE`;
- `supermath-subagents`, the `agent_*` tools and the duel of
  [ADR 0009](0009-settle-conjectures-by-a-prover-refuter-duel.md).

They brought the `vendor/openhuman` submodule and its nested submodules, a
`[patch]` table that had to move with every gitlink bump, Rust 1.96, X11 and
audio libraries in CI, a Node stage in the image, and OpenRouter for Jev.
Every `cargo test` compiled all of OpenHuman.

The runner's loop never measured progress. A turn could end "done" with
nothing checked, and nothing compared one attempt with the next.

[`rsi-ralph`](https://github.com/tinyhumansai/rsi-ralph) is a binary that
already embeds OpenHuman. It runs hives of agents over epochs against a
subject, scores every candidate with commands the loop folder names, and
promotes only a measured improvement
([research note](../research/rsi-ralph.md)).

Since tinyhumansai/rsi-ralph#75 (`e40c7e3`), its seats call the executable
tools their `hive.json` declares.

## Decision

- **One conjecture is one rsi-ralph loop folder.**
  - `supermath-mcp loop new` writes it, with the statement frozen.
  - `supermath-mcp score proof` is its scorer, and accepts a side only
    through a theorem Lean checks at exactly the pinned type.
  - Running it is `rsi-ralph run`. supermath does not fork rsi-ralph, vendor
    it, or link it.
- **The toolkit reaches the agents as programs.**
  - Each tool call is one `supermath-mcp call` process.
  - `supermath-mcp hive-tools` writes the `hive.json` tool list from the
    registry.
  - The MCP server stays for clients that speak MCP.
- **The `agent_*` roles become seats.** The prover and the refuter each get
  their own candidate checkout, and the lead reads the incumbent. The
  scorer's verdict replaces the duel's "first verified win".
- **OpenHuman leaves the repository.**
  - Removed: `supermath-agent`, `supermath-subagents`, `vendor/openhuman`,
    the `[patch]` table, the agent image and its compose service.
  - `scripts/check-crate-deps.sh` now fails on any `openhuman*` dependency.
  - The formalization benchmark data moves to `bench/formalize/` for a later
    meta-loop.

## Consequences

- **Builds.** A cold build and test of the workspace compiles nine small
  crates. CI checks out no submodule and installs no system package.
  `docker/mcp/Dockerfile` is the only Rust image.
- **Models.** They are set per loop in `rsi.json`, and rsi-ralph's gateway
  sends them to `TINYRSI_UPSTREAM`, which is Surplus Intelligence. The
  `SUPERMATH_*_MODEL` and `SUPERMATH_DUEL_*` variables are gone, and so are
  Jev and `OPENROUTER_API_KEY`: `tool_search` was OpenHuman's. CortexDB
  still calls Surplus directly, as ADR 0008 decided.
- **Running.** A loop needs an `rsi-ralph` built from `e40c7e3` or later.
  There is no tagged release yet, so the binary is built from source.
- **State across processes.** Each call is a fresh process. The call log is
  therefore persisted per checkout (`.log/calls.jsonl`), so `ledger_record`
  can still cite earlier calls. The Lean verdict cache and the web daily cap
  stay per process.
- **Unused pieces removed.** `Backend::OpenHuman` and
  `Ledger::set_alignment` lost their callers with the crates and are removed
  as well. An entry's `alignment` field stays: it is `not_reviewed` on every
  new claim, and a stored entry keeps the alignment it was given.
- **Ledger writes are not rolled back.** A candidate rsi-ralph rejects may
  still have written to the ledger. Claims are earned only on evidence, so
  such an entry cannot claim more than its sources support.
- **Rewritten as loops:** `supermath-agent solve` and `check`, the
  formalization bench test, and the `agent_*` tools over MCP are gone. The
  bench returns as a meta-loop over the data in `bench/formalize/`, if one is
  built.
