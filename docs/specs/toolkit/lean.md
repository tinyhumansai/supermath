# Lean tools

- **Status:** Draft
- **Parent:** [`README.md`](README.md)
- **Decision:** [`../../adr/0003-run-lean-as-a-pinned-service.md`](../../adr/0003-run-lean-as-a-pinned-service.md)

Lean is the toolkit's arbiter. These tools let an agent state a claim in
Lean, find the Mathlib facts it needs, work a proof one goal at a time, and
get a verdict the ledger can trust.

## The Lean service

A `lean` service in `compose.yaml`, built from `docker/lean/Dockerfile`:

- **Pinned.** One `lean-toolchain` and one Mathlib revision, recorded in
  `docker/lean/lakefile.toml` and `lake-manifest.json`. The image runs
  `lake exe cache get` at build time, so no check compiles Mathlib.
- **REPL-backed.** It runs
  [leanprover-community/repl](https://github.com/leanprover-community/repl)
  processes with `import Mathlib` already elaborated, so a check pays for
  its own source, not for the import. The REPL's JSON commands (`cmd` with an
  `env`, `tactic` with a `proofState`) carry the goal-level tools.
- **Behind HTTP.** A thin server in the image takes a check request, hands it
  to a pooled REPL, and returns the REPL's messages, sorries, and proof
  states. Before writing one, evaluate
  [kimina-lean-server](https://github.com/project-numina/kimina-lean-server),
  which already pools REPLs behind REST; the decision goes in the ADR.
- **Per-project package.** A volume holds one Lake package per project,
  `projects/<slug>/`, depending on the pinned Mathlib. `lean_write` and
  `lean_build` work on it; `lean_check` does not need it.

`supermath-mcp` reaches the service at `LEAN_URL` (default
`http://localhost:3150`). The service's `GET /health` reports the toolchain
and Mathlib revision; `supermath-mcp check` and `supermath-agent check`
report both.

### Isolation

Lean source is untrusted. Elaboration can run arbitrary code through
`#eval`, macros, and `run_cmd`, and can loop. The service:

- runs as an unprivileged user with no network and a read-only root, the
  project volume being the only writable path;
- caps each request's wall time, memory, and output size, and kills the REPL
  that exceeded them;
- refuses source containing `IO.Process`, `IO.FS` writes, or `#exit`-style
  escapes in `lean_check` (a deny-list is a backstop, not the boundary; the
  container is the boundary).

## Verdicts

`lean_check` reduces what Lean said to one of four verdicts:

| Verdict | When |
| --- | --- |
| `checked` | no errors, no `sorry`, and every axiom of every target declaration is in the allowlist |
| `checked_with_sorry` | no errors, but at least one `sorry` (or `admit`, or `sorryAx`) |
| `failed` | Lean reported an error |
| `unknown` | the check timed out, ran out of memory, or was refused by budget |

The allowlist is `propext`, `Classical.choice`, and `Quot.sound`.
`Lean.ofReduceBool` (from `native_decide`) and `Lean.trustCompiler` are
**named debt**: the verdict is `checked`, and `data.debt` lists each
declaration that depends on them, so the ledger can refuse to call it
`formalised` until the debt is retired. Any other axiom, including one the
source declares itself, makes the verdict `failed` with code
`UNTRUSTED_AXIOM` in its diagnostics.

Axioms are read by running `#print axioms` on each target declaration after
elaboration, in the same environment. A declaration that is not a target is
not reported.

### Provenance

Every `checked` and `checked_with_sorry` verdict carries:

```json
{
  "toolchain": "leanprover/lean4:v4.x.y",
  "mathlib": "<commit>",
  "source_sha256": "<hex>",
  "elapsed_ms": 1840,
  "axioms": { "Foo.bar": ["propext", "Classical.choice"] },
  "debt": []
}
```

This is what the ledger stores with a `formalised` claim. Re-checking the
same source on the same toolchain and Mathlib must give the same verdict.

## Tools

Each tool is specified in its own file under [`tools/`](tools/README.md).

| Tool | Given → returns |
| --- | --- |
| [`lean_check`](tools/lean_check.md) | Lean source → a [verdict](lean.md#verdicts) with provenance |
| [`lean_search`](tools/lean_search.md) | a name, pattern, or phrase → Mathlib declarations |
| [`lean_decl`](tools/lean_decl.md) | a declaration name → signature, docstring, module |
| [`lean_goals`](tools/lean_goals.md) | source and a position → the goal state there |
| [`lean_try_tactics`](tools/lean_try_tactics.md) | source, a `sorry`, tactics → per-tactic outcome |
| [`lean_write`](tools/lean_write.md) | a module path and source → the file, checked |
| [`lean_build`](tools/lean_build.md) | the project package → per-module verdicts |
| [`lean_blueprint`](tools/lean_blueprint.md) | the project → the declaration DAG with status |

## Mathlib bumps

The Mathlib revision moves only by an explicit change to
`docker/lean/lake-manifest.json`, in its own commit. A bump re-checks every
`formalised` claim in the ledger; a claim that no longer checks drops to
`asserted` with a `supersedes` record naming the bump.

## Acceptance criteria

- `lean_check` maps each of these to its verdict, in unit tests against a
  stubbed service: a clean proof, a `sorry`, an `admit`, an error, a custom
  axiom, `native_decide`, a timeout.
- A `live_lean_check` test, gated by `LEAN_URL`, checks `two_add_two` as
  `checked` and records the service's toolchain and Mathlib revision.
- `lean_write` refuses a path outside `Supermath/` and a path with `..`.
- The service image has no network at runtime, which a compose test asserts.

## Open questions

1. Build the HTTP wrapper ourselves in Rust, or adopt kimina-lean-server?
2. Host LeanSearch-style natural search ourselves, or accept a hosted index
   on a different Mathlib revision?
3. How many warm REPLs per service, and is one service shared by every
   project?
