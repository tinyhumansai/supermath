# `lean_build`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | primitive | deferred | read | P5 |

**Given → returns:** the project package → per-file verdicts.

Runs `lake build` on the whole package and returns a verdict per file, as
[`lean_check`](lean_check.md) would, plus every declaration whose proof
depends on a `sorry` in another file. Files whose digests and imports have
not changed reuse their cached verdicts. Budget: 15 minutes per build.
