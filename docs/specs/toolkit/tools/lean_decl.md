# `lean_decl`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | deferred | read | P2 |

**Given → returns:** a declaration name → signature, docstring, module.

Given a declaration name, returns its full signature, docstring, module, and
whether it is a theorem, definition, structure, or instance.
`NOT_FOUND` if the pinned environment does not have it.
