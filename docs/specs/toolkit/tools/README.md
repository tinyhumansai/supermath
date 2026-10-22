# Tool specs

One file per tool, named for the tool. The catalogue in
[`../README.md`](../README.md#catalogue) lists them all with their crate,
exposure, and phase; the area specs ([`../lean.md`](../lean.md),
[`../literature.md`](../literature.md), [`../web.md`](../web.md),
[`../ledger.md`](../ledger.md),
[`../agents.md`](../agents.md)) hold what a whole area shares: the Lean
service and its verdicts, sources and politeness, statuses and storage,
how a child agent runs.

Every tool file follows the same shape:

1. A header table: crate, area, kind (`primitive`, `agentic (Gemini)`, or
   `agentic (OpenHuman)`), exposure, permission, phase.
2. **Given → returns**, the one postcondition, matching the tool's
   description.
3. An example of its arguments.
4. Its behavior: what it reads, what it returns, its caps and budgets.
5. For an agentic tool, a **Turn** section: the model, what it decides,
   the primitives the turn may call, its budget, and what the tool
   verifies, returns, and discards after the turn
   ([kinds](../README.md#primitive-and-agentic-tools)).
6. Its errors, by [code](../README.md#one-envelope), when they go beyond
   the area's.

A tool's file changes in the same commit as its implementation. A new tool
gets a file and a catalogue row before it gets code.
