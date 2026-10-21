# REA and Jacobian: how morluto builds hard tools for agents

Both projects come from the same author (GitHub `morluto`) and follow the same
playbook in two very different fields. Studying them side by side shows which
parts of the approach are general method and which are adapted to the field.

## Sources

| Repository | What it is | Revision studied | Size at that revision |
| --- | --- | --- | --- |
| [morluto/rea](https://github.com/morluto/rea) | One MCP server and CLI for reverse engineering: native binaries, Android, Electron/JavaScript, .NET, firmware, websites, captures and more | `01788ba`, package `rea-agents` 6.7.0 | ~400k lines of TypeScript source, ~566k lines of tests, ~146 MCP tool contracts, ~6,100 PRs since April 2026 |
| [morluto/jacobian](https://github.com/morluto/jacobian) | One MCP server, CLI and Python API exposing typed mathematical operations | `9dc2aaf`, package `jacobian` 0.23.0 | ~600k lines of Python source, ~417k lines of tests, ~1,400 operations behind 2 MCP tools, ~4,500 PRs since July 2026 |

Figures were measured from local clones and the GitHub API on 2026-10-21.
Line counts are rough (`wc -l` over source and test files) and include
comments.

## Files

- [`projects.md`](projects.md): what each project does, how it is structured,
  and where the two deliberately differ.
- [`principles.md`](principles.md): the design principles both projects
  share, with the source rule each one comes from.
- [`verification.md`](verification.md): how they test, evaluate and benchmark
  work whose correctness is hard to establish.
- [`agent-driven-development.md`](agent-driven-development.md): how one
  maintainer ships thousands of agent-written PRs without the codebase
  degrading.
- [`implications-for-supermath.md`](implications-for-supermath.md): what we
  should take from this, and what we should not.

## Summary

The visible part of both projects is "wrap existing engines in MCP". REA wraps
Ghidra, Hopper, IDA, JADX and others; Jacobian wraps SymPy, NetworkX, Z3 and
FLINT. That part is easy to copy and is not what makes them work. The
differences are in five places:

1. **The contract is owned, not inherited.** Each project defines its own
   provider-neutral operations and treats backend behavior as an
   implementation detail. A backend exception never defines the public
   contract.
2. **Honest result semantics.** "Unknown" is never collapsed into "absent" or
   "false". A timeout never becomes a negative answer.
3. **Designed for the agent's context window.** REA ranks agent tokens above
   analysis time and CPU. Jacobian hides 1,400 operations behind a search tool
   so their schemas are never all loaded at once.
4. **Very deep verification.** Real-engine CI lanes, mutation testing,
   independent oracles, adversarial boundary tests, and benchmark tasks with
   clean-room verifiers.
5. **Agent-driven development under strict written rules.** AGENTS.md, PR
   templates and project skills act as the control system that keeps a fleet
   of coding agents consistent.

The most transferable lesson is the method, not the code: decide what one
operation means, prove it at its boundary, report what you do not know, and
write the rules down so agents can follow them.
