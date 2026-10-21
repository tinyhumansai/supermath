# The two projects

## REA: reverse engineer anything

REA lets an agent inspect software without its source and explain how a
feature works, with evidence. The same workflows are available from the
terminal.

**Layers** (from `docs/architecture.mermaid`; dependencies point inward):

```text
agent (Claude Code, Codex, Cursor, ...)
  -> src/server, src/cli        thin MCP and CLI adapters
  -> src/application            shared workflows, sessions, EvidenceLedger
  -> src/domain, src/contracts  pure, side-effect-free semantics and schemas
  -> providers                  Ghidra, Hopper, IDA, Kuna, Rizin, JADX, ...
                                plus in-house decoders (.NET, IL2CPP, Go, ...)
  -> src/process                owned child processes, deadlines, cleanup
```

Where the code goes (non-test lines per folder): `domain` 120k,
`application` 62k, `browser` 28k, `artifacts` 20k, `process` 15k,
`ghidra` 14k, `hopper` 6k, `ida` 4k. The pure domain layer is far larger than
any engine adapter.

**Wrapped engines.** Ghidra (headless Java bridge), Hopper (Python bridge over
a Unix socket), IDA (through the existing `ida-pro-mcp`), JADX (through
`jadx-headless-mcp`), Kuna, Rizin, objdump, apktool, binwalk, unblob, tshark,
adb, Frida, GDB, Chrome DevTools Protocol. Most are bring-your-own: REA never
installs Ghidra, IDA or Java.

**In-house decoders.** .NET metadata and CIL, Unity IL2CPP metadata, Go build
info, Rust cargo-auditable data, GPU code objects, GGUF and safetensors
headers, Godot PCK, Flutter snapshot provenance, PyInstaller archives. None of
these folders start a child process.

**Core idea: every result is an Evidence record** (`src/domain/evidence.ts`):
the subject's name, format and SHA-256, the provider's identity and version,
the result, coverage, limitations and unknowns. The agent explains a feature
from that record. Reverse-engineering claims depend on which bytes, which
engine and how much was examined, so provenance is part of the answer.

## Jacobian: an executable mathematical vocabulary

Jacobian gives an agent a searchable catalog of typed mathematical
operations. The README states its hypothesis: "mathematical reasoning
benefits from an executable vocabulary of semantically scoped, bounded
operations."

**Surface.** Two MCP tools only:

- `math.find` searches the catalog with a natural-language need, or inspects
  one exact operation contract.
- `math.run` executes one operation with a typed payload and returns a typed
  result.

The same library is available as a CLI (`jacobian inspect`, `jacobian run`)
and a native Python API.

**Catalog.** About 1,400 `MathTool` declarations across 331 owner-local
`_tools.py` manifests under `src/jacobian/math/`, in 36 domains: number
theory, polynomials, topology, combinatorics, geometry, graphs, logic,
matrices, groups, Lie algebras, finite fields, probability and others. A
declaration in a manifest is the publication decision; catalog construction
fails closed on malformed declarations or duplicate IDs.

**Backends.** SymPy, NetworkX, Z3 and python-flint are mandatory and ship in
the package. Singular and QEPCAD are optional system runtimes for a small
subset of operations.

**Example operation:** `integer.compute.extended_gcd` with
`{"left":"84","right":"30"}` returns the gcd and the Bézout coefficients.
`matrix.determinant.compute` takes one exact rational matrix and returns its
determinant.

**Lean.** Jacobian once shipped `lean.check` with a packaged Lean runtime and
pinned Mathlib checkouts. Version 0.14.0 (2026-08-26) removed it. Lean now
appears only as frozen source material inside benchmark tasks that the agent
reads but does not run.

## Where they deliberately differ

| Question | REA | Jacobian | Why the difference makes sense |
| --- | --- | --- | --- |
| How many MCP tools? | ~146 separate tools | 2 tools over a ~1,400-operation catalog | At 1,400 operations, advertising every schema would flood the context; discovery becomes a tool call |
| What does a result carry? | A full Evidence record: digest, provider and version, coverage, unknowns | Plain exact values. AGENTS.md: "without certificates, source digests, or `verified` flags by default" | A claim about a binary depends on its provenance. A determinant can be checked on its own, so provenance would only cost tokens |
| State | Per-target analysis sessions and an evidence ledger | A stateless kernel; "the caller owns reasoning, composition, and durable state" | Engine sessions are expensive to start; math operations are not |
| Workflows | Ships a workflow when repeated use shows a recurring multi-source outcome | Refuses workflows and proof strategies. Each operation does one mathematical job | In math, bundling a strategy into a tool shrinks the space of proofs the model can explore |
| Witnesses | Evidence is always attached | A witness is returned only when it is the operation's mathematical purpose, e.g. a matching or a certificate | Same rule as above: return what the caller needs to reason further, and nothing else |

## What they share

- One MCP server for a whole field, with one-command setup across many agent
  clients (`npx rea-agents setup`, `npx jacobian setup`), always showing a
  dry-run plan first.
- A CLI and an MCP adapter over the same workflows, so behavior cannot drift
  between them.
- Strict, typed inputs and outputs; the same contract checked on both
  surfaces.
- Bounded resources enforced before expensive work starts.
- The same development method: very high PR volume from coding agents under
  a long, precise AGENTS.md. See
  [`agent-driven-development.md`](agent-driven-development.md).
