# `lean_write`

| Crate | Area | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- |
| `supermath-mcp` | [Lean](../lean.md) | deferred | write | P4 |

**Given → returns:** a module path and source → the file, checked.

Given a module path inside the project package and its source, writes the
file and checks it in the package's environment. The path must match
`^Supermath/[A-Z][A-Za-z0-9]*(/[A-Z][A-Za-z0-9]*)*\.lean$`; anything else is
`INVALID_INPUT`. Returns the verdict for every declaration in the file.

The project slug comes from the host, not the arguments. `lean_write` is the
only tool that changes the package.
