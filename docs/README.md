# Documentation

This directory holds documentation that does not belong in rustdoc: the shape
of the system, the reasoning behind it, and the constraints a reader needs
before touching the code. API reference lives in doc comments next to the code,
where it cannot drift.

## Layout

```text
docs/
├── README.md      # this index
├── specs/         # behavior and architecture specifications
├── plans/         # implementation plans derived from approved specs
├── adr/           # architecture decision records, numbered and immutable
└── research/      # notes on outside work that informs the design
```

- **[`specs/`](specs/README.md)** — one file per feature, module, or subsystem,
  describing its behavior, public surface, invariants, and acceptance criteria.
- **[`plans/`](plans/README.md)** — implementation-ordered, test-first steps for
  delivering an approved specification. Plans name exact files and verification
  commands, and are updated as the work progresses.
- **`adr/`** — a dated record per significant decision. Use
  [`adr/0001-record-architecture-decisions.md`](adr/0001-record-architecture-decisions.md)
  as the template. An accepted ADR is not edited; it is superseded by a later
  one.
- **[`research/`](research/README.md)** — notes on outside repositories,
  papers, and techniques, pinned to a revision. Research informs specs and
  ADRs but does not replace them.

Complex modules also carry a module-level `README.md` inside `src/<module>/`
covering their design, public surface, and important constraints.

The agent runner is specified in
[`specs/agent-runner.md`](specs/agent-runner.md), and
[`adr/0002-build-on-openhuman.md`](adr/0002-build-on-openhuman.md) records why
it embeds OpenHuman.

The conjecture toolkit (literature, Lean, the claim ledger, and sub-agents),
split into the `supermath-mcp` toolkit crate and the `supermath-agent`
OpenHuman runner, is specified in [`specs/toolkit/`](specs/toolkit/README.md),
planned in [`plans/lean-toolkit.md`](plans/lean-toolkit.md), and its
decisions are ADRs [0003](adr/0003-run-lean-as-a-pinned-service.md) and
[0004](adr/0004-split-the-toolkit-into-an-mcp-crate.md).

## Conventions

- Keep every Markdown file at 500 lines or fewer. When a topic outgrows that,
  split it into focused files and link them from the nearest `README.md`.
- Update documentation in the same commit as the behavior it describes.
- Prefer a concrete example over an abstract description.
- Link between documents rather than duplicating content; one fact lives in one
  place.
- Write a specification before a plan: the spec defines the outcome and
  constraints, while the plan defines the implementation sequence.
