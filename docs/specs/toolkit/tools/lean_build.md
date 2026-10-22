# `lean_build`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | deferred | read | P5 |

**Given → returns:** the project package → per-module verdicts.

Given nothing but the project, runs `lake build` on the package and returns
a verdict per module, plus every declaration whose proof depends on a
`sorry` elsewhere in the package.
