# Tool spec files

Every crate's spec folder has a `tools/` folder with one file per tool,
named for the tool: [`lean/tools/lean_check.md`](lean/tools/lean_check.md),
[`python/tools/py_run.md`](python/tools/py_run.md), and so on. The
[catalogue](README.md#catalogue) lists them all. The crate's `README.md`
holds what its tools share: the backend service, verdicts or outcomes,
status rules, and the `## Crate` section.

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
   ([kinds](README.md#primitive-and-agentic-tools)).
6. Its errors, by [code](README.md#one-envelope), when they go beyond
   the crate's.

A tool's file changes in the same commit as its implementation, in the
crate that implements it. A new tool gets a file and a catalogue row
before it gets code.
