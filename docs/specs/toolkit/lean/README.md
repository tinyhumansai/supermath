# Lean tools

- **Status:** Implemented; the `docker/lean` image is not yet built or run against real Lean and Mathlib
- **Crate:** `supermath-lean`
- **Parent:** [`README.md`](../README.md)
- **Decision:** [`../../../adr/0003-run-lean-as-a-pinned-service.md`](../../../adr/0003-run-lean-as-a-pinned-service.md)

Lean is the toolkit's arbiter. These tools let an agent state a claim in
Lean, find the Mathlib facts it needs, work a proof one goal at a time, and
get a verdict the ledger can trust. They work on **files in a project's
Lean package**, never on source passed as an argument
([below](#files-not-source)).

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
  states. It is a small stdlib-only Python server we wrote
  (`docker/lean/server.py`); [kimina-lean-server](https://github.com/project-numina/kimina-lean-server)
  was considered and not adopted
  ([ADR 0003](../../../adr/0003-run-lean-as-a-pinned-service.md#evaluation-of-kimina-lean-server)).
- **Per-project package.** The service mounts the `projects` volume and
  builds each project's `lean/` package there
  ([`files/`](../files/README.md#layout)).

`supermath-lean` reaches the service at `LEAN_URL` (for example
`http://lean:3150` inside compose, or `http://localhost:3150` through the
opt-in `lean-gateway`). **With `LEAN_URL` unset there is no service, and every
tool that needs it answers `BACKEND_UNAVAILABLE`**; nothing is checked and
nothing is guessed. The service's `GET /health` reports the toolchain and
Mathlib revision; `supermath-mcp check` reports both.

### Isolation

Lean source is untrusted. Elaboration can run arbitrary code through
`#eval`, macros, and `run_cmd`, and can loop. The service:

- runs as an unprivileged user with no network and a read-only root, the
  project volume being the only writable path;
- caps each request's wall time, memory, and output size, and kills the REPL
  that exceeded them;
- refuses to write files containing `IO.Process`, `IO.FS` writes, or
  `#exit`-style escapes (a deny-list in `file_write` and `file_edit` is a
  backstop, not the boundary; the container is the boundary).

## The HTTP API

The contract between `crates/supermath-lean` and `docker/lean/server.py`.
Every route but `/health` is a JSON `POST`; a request names a project `slug`
and a `path` and the service reads the file itself from the `projects` volume
(a request never carries source). A request may also carry `root`, the
project's absolute directory, when the project is not on that volume, as with
an rsi-ralph candidate checkout (`Workspace::single`, which sends it). The
root must lie inside a directory the service's `SUPERMATH_ROOTS` lists
(colon-separated; unset allows none), or the request is a `400`; see
`compose.rsi.yaml`. Such a project is only read: when a file imports other
project modules, its `lean/` sources are staged under `LEAN_BUILDS_DIR`
(default `/tmp/lean-builds`) and built there, so no `.lake` lands in the
checkout. Replies are `200` with a `status`:

| `status` | Meaning |
| --- | --- |
| `ok` | the operation ran to completion |
| `timeout`, `memory`, `output_limit` | it hit a limit and the REPL was killed |

A REPL that dies on its own while `/check` runs (Lean aborts with SIGABRT on
a stack overflow, typically deep `decide` or kernel reduction) is not a
limit: `/check` answers `ok` with one error at line 1 naming the crash, so the
file fails with a reason the caller can act on. Only the memory limit's
SIGKILL stays `memory`.
| `no_sorry`, `bad_position`, `bad_predicate`, `not_found` | the operation's own refusals, named below |

`404` means the project file does not exist and `400` that the request was
malformed (a bad slug, path, or name); both carry `{"status": "refused",
"message"}`. Anything else (`5xx`, a reply that does not parse) is the service
being down. Every execution reply also carries `toolchain`, `mathlib`, and
`elapsed_ms`.

| Route | Request | Reply |
| --- | --- | --- |
| `GET /health` | | `{status, toolchain, mathlib}` |
| `POST /check` | `{project, path, targets, uses, timeout_ms}` | `{messages, sorries, axioms, uses}` |
| `POST /goals` | `{project, path, line, column, timeout_ms}` | `{goals}` (`bad_position` outside the file) |
| `POST /tactics` | `{project, path, line, column, tactics, timeout_ms}` (per tactic) | `{results: [{tactic, result, goals, error, term}]}` (`no_sorry`) |
| `POST /decl` | `{project, name, path, timeout_ms}` | `{name, kind, signature, doc, module}` (`not_found`) |
| `POST /search` | `{query, limit, timeout_ms}` | `{mathlib, results: [{name, type, module}]}` |
| `POST /counterexample` | `{project, path, predicate, from, to, timeout_ms}` | `{domain, witness, covered_to}` (`bad_predicate` with `message`; `timeout` keeps `covered_to`) |

- `messages` is `[{severity, line, column, text}]` (`error`, `warning`, or
  `info`), lines one-based and columns zero-based, exactly as the REPL
  reports them. `sorries` is `[{line, column, goal}]`.
- `axioms` maps each target to the axioms `#print axioms` listed after
  elaboration in the same environment, or `null` when the target does not
  exist. With `uses: true`, `uses` maps each target to the project
  declarations its type and value mention.
- `/decl` looks in Mathlib; when `path` is given it elaborates that project
  file first, so the project's own declarations resolve. `signature` is
  `#check`'s output, which never includes the proof.
- `/tactics` runs each tactic against the `sorry`'s proof state in order, in
  one REPL, each under `timeout_ms`; `result` is `closes`, `progress`, or
  `fails`, and `term` is what `exact?` found.
- `/search` matches name fragments over Mathlib's environment, shortest
  names first. `pattern` and `natural` modes never reach the service: it has
  no network, so the agent calls Loogle and LeanSearch itself
  (`LOOGLE_URL`, `LEANSEARCH_URL`).
- `/counterexample` evaluates the predicate in chunks of 1,000 and stops at
  the first witness; the range is inclusive, `domain` is `nat` or `int`.

A file that imports `Supermath.*` modules is checked after the service runs
`lake build` on them, in the project's own package; a file that imports only
Mathlib runs in a warm REPL with `import Mathlib` already elaborated. Per-
request memory is the container's limit: a REPL killed by it is `memory`.

## Files, not source

Every Lean tool takes a **path** to a file in the project's
[workspace](../files/README.md), under `lean/Supermath/`, never a string of
Lean source. Source gets there only through
[`file_write`](../files/tools/file_write.md) and
[`file_edit`](../files/tools/file_edit.md); the reasons, the path rules, and
the write log are in [`files/`](../files/README.md).

The Lean package is `projects/<slug>/lean/`, a Lake package requiring the
pinned Mathlib. A Lean file may import `Mathlib` and `Supermath.*` modules
of the same project.

**Workflow.** A typical proof loop is: `file_write` the statement with a
`sorry` proof → `lean_check` (verdict `checked_with_sorry`, the `sorry`s
listed) → `lean_goals` on one → `lean_try_tactics` → `file_edit` the
closing tactic in → `lean_check` again.

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
**named debt**: the verdict is `checked`, and `provenance.debt` (in
`lean_check`, `data.provenance.debt`; `lean_build` repeats it per file as
`debt`) lists each declaration that depends on them, so the ledger can refuse to call it
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
  "path": "lean/Supermath/Sendov/Main.lean",
  "file_sha256": "<hex>",
  "imports_sha256": "<hex over every Supermath module it imports>",
  "elapsed_ms": 1840,
  "axioms": { "Foo.bar": ["propext", "Classical.choice"] },
  "debt": []
}
```

This is what the ledger stores with a `formalised` claim. Re-checking the
same file and imports on the same toolchain and Mathlib must give the same
verdict. When the file changes, every verdict given for its old digest is
stale, and the ledger reports claims resting on it as unchecked until
`lean_check` runs again.

## Crate

| | |
| --- | --- |
| Name | `supermath-lean` |
| Depends on | `supermath-tool`, `supermath-files`, `reqwest`, `serde_json`, `tokio` |
| Public API | `Lean` (builds everything), `LeanClient`, `LeanChecker` (`verify` a declaration at a path, `is_current` for staleness; what `supermath-mcp` adapts to the ledger's `LeanEvidence` port, so this crate does not depend on the ledger), `Verdict`, `Provenance`, `CheckReport`, `LeanObserver` (a `FileObserver`), `ClaimIndex` (the port `lean_blueprint` asks for claim ids), and the tools below |
| Configuration | `LEAN_URL`; `LOOGLE_URL`, `LEANSEARCH_URL` (hosted indexes); `LeanConfig` also holds the per-tool time budgets |
| Tests | verdict reduction and every tool against a loopback stub replaying recorded REPL answers; `live_lean_*` against a real service, gated by `LEAN_URL` |

## Tools

Each tool is specified in its own file in this crate's `tools/` folder, in
the shape [`../tools.md`](../tools.md) sets out.

| Tool | Given → returns |
| --- | --- |
| [`lean_check`](tools/lean_check.md) | a Lean file in the project → a [verdict](#verdicts) with provenance |
| [`lean_search`](tools/lean_search.md) | a name, pattern, or phrase → Mathlib declarations |
| [`lean_decl`](tools/lean_decl.md) | a declaration name → signature, docstring, module, without the proof |
| [`lean_goals`](tools/lean_goals.md) | a file and a position → the goal state there |
| [`lean_try_tactics`](tools/lean_try_tactics.md) | a file, one of its `sorry`s, and tactics → per-tactic outcome |
| [`lean_search_counterexample`](tools/lean_search_counterexample.md) | a decidable predicate in a project file and a range → a witness, or `exhausted` over the whole range |
| [`lean_build`](tools/lean_build.md) | the project package → per-file verdicts |
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
- A `live_lean_check` test (`tests/live.rs`), gated by `LEAN_URL`, writes `two_add_two` to a
  file, checks it as `checked`, and records the service's toolchain and
  Mathlib revision.
- `file_write` and `file_edit` refuse a path outside `Supermath/`, a path
  with `..`, a symlink out of the package, and a forbidden import.
- `file_edit` refuses an `old` string that matches zero or two times, and
  both write tools refuse a stale `if_sha256` with `CONFLICT`.
- A verdict is reported stale by `file_list` once its file changes.
- No `lean_*` tool's schema has a Lean-source argument except `file_write`'s
  `content` and `file_edit`'s `new`; a test walks the schemas.
- The service image has no network at runtime, which a compose test asserts.

## Open questions

1. ~~Build the HTTP wrapper ourselves, or adopt kimina-lean-server?~~ Decided:
   our own small wrapper, [ADR 0003](../../../adr/0003-run-lean-as-a-pinned-service.md#evaluation-of-kimina-lean-server).
2. Host LeanSearch-style natural search ourselves, or accept a hosted index
   on a different Mathlib revision?
3. How many warm REPLs per service, and is one service shared by every
   project?
