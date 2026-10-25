# `lean_decl`

| Crate | Area | Kind | Exposure | Permission | Phase |
| --- | --- | --- | --- | --- | --- |
| `supermath-lean` | [Lean](../README.md) | primitive | deferred | read | P2 |

**Given → returns:** a declaration name → signature, docstring, module, without the proof.

Given a declaration name, returns its full signature, docstring, module, and
whether it is a theorem, definition, structure, or instance.
`NOT_FOUND` if the pinned environment does not have it.

Names are looked up in Mathlib and in the project's package, so a
declaration written with [`file_write`](../../files/tools/file_write.md) can be inspected
too; for those, the answer adds the `path` and line of the file it lives
in. The signature never includes the proof, so a reviewer can judge a
statement's [alignment](../../ledger/README.md#alignment) without seeing how
it was proved.

`name` must be letters, digits, `_`, `'`, `.`, `!`, `?` only; anything else is
`INVALID_INPUT`. `TIMEOUT` and `BACKEND_UNAVAILABLE` are as for
[`lean_check`](lean_check.md).
