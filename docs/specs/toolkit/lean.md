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

The agent reaches the service at `LEAN_URL` (default `http://localhost:3150`).
The service's `GET /health` reports the toolchain and Mathlib revision; the
agent's `check` command reports both.

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

### `lean_check`

Given Lean source and the names of its target declarations, returns a
verdict.

```json
{
  "source": "theorem two_add_two : 2 + 2 = 4 := by norm_num",
  "targets": ["two_add_two"],
  "depth": "proof"
}
```

- `source` starts after `import Mathlib`; the import is implicit and other
  imports are refused (`INVALID_INPUT`), so every check sees the same
  environment.
- `depth: "statement"` checks only that the statement elaborates: the proof
  is replaced by `sorry`, and the verdict is `checked_with_sorry` at best.
  Formalizing a statement before proving it is the common case.
- `data.errors` lists at most 20 errors as `{line, column, message}`, the
  messages trimmed to 400 characters. `data.sorries` lists each `sorry` with
  its position and goal.
- Budget: 60 s and 200 KB of source by default.

### `lean_search`

Given a query and a mode, returns Mathlib declarations.

| Mode | Backend | Example |
| --- | --- | --- |
| `name` | the REPL's environment, by name fragment | `Nat.Prime.two_le` |
| `pattern` | [Loogle](https://loogle.lean-lang.org) | `(?a + ?b) ^ 2` |
| `natural` | a natural-language index such as [LeanSearch](https://leansearch.net) | `sum of two squares` |

Each row is `{name, type, module}`, with types trimmed to 300 characters.
`coverage` names the backend and the Mathlib revision it indexes; a hosted
backend on a different revision than ours is noted in `unknowns`.

### `lean_decl`

Given a declaration name, returns its full signature, docstring, module, and
whether it is a theorem, definition, structure, or instance.
`NOT_FOUND` if the pinned environment does not have it.

### `lean_goals`

Given source and a position (line and column), returns the goal state there:
hypotheses and targets, pretty-printed. Used to work a proof without
re-sending it for every step.

### `lean_try_tactics`

Given source, the index of one `sorry`, and up to 12 tactics, returns per
tactic one of `closes`, `progress` (with the remaining goals), or `fails`
(with the error). The default list, when none is given, is `rfl`,
`norm_num`, `simp`, `omega`, `decide`, `linarith`, `nlinarith`, `positivity`,
`aesop`, and `exact?`. A tactic that closes the goal is returned with the
term `exact?` found, when it found one.

Tactics run in the REPL's tactic mode against the `sorry`'s proof state, in
parallel, each under its own timeout (10 s by default).

### `lean_write`

Given a module path inside the project package and its source, writes the
file and checks it in the package's environment. The path must match
`^Supermath/[A-Z][A-Za-z0-9]*(/[A-Z][A-Za-z0-9]*)*\.lean$`; anything else is
`INVALID_INPUT`. Returns the verdict for every declaration in the file.

The project slug comes from the host, not the arguments. `lean_write` is the
only tool that changes the package.

### `lean_build`

Given nothing but the project, runs `lake build` on the package and returns
a verdict per module, plus every declaration whose proof depends on a
`sorry` elsewhere in the package.

### `lean_blueprint`

Given the project, returns the dependency graph of its declarations: nodes
with their verdict and ledger claim id, edges for each declaration's use of
another. The answer to "what does the main theorem still rest on" is the
set of `checked_with_sorry` nodes reachable from it. Formalization effort
goes first to the node most other nodes rest on (Scholze's rule,
[`strategies/04-trust.md`](../../research/strategies/04-trust.md)).

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
