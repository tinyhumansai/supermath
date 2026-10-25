# Repository Guidelines

This file is the single source of truth for how humans and coding agents work
in this repository. `CLAUDE.md` is a symlink to this file, so every agent reads
the same instructions.

## Project Structure

supermath is a toolkit for settling math conjectures, and the loops that use
it; the agents run in [`rsi-ralph`](https://github.com/tinyhumansai/rsi-ralph).
It is a Rust 2024 cargo workspace rooted at a virtual `Cargo.toml`. Every crate
lives under `crates/`, one directory per package, each directory named for the
package it holds.

```text
Cargo.toml              # virtual workspace: members, [workspace.package],
                        # [workspace.lints], [workspace.dependencies]
crates/                 # one directory per package, named for the package
├── supermath-tool/         # the contract: Tool, Envelope, ErrorCode, Budget, Project
├── supermath-files/        # project workspaces: Lean and Python files
├── supermath-lean/         # the Lean service client and the lean_* tools
├── supermath-python/       # the Python solver client and the py_* tools
├── supermath-literature/   # arXiv, Semantic Scholar, OEIS: the lit_* tools
├── supermath-web/          # Gemini grounded search: the web_* tools
├── supermath-ledger/       # CortexDB memory, claims: the ledger_* tools
├── supermath-calc/         # exact arithmetic: calc_factor, calc_gcd_lcm
└── supermath-mcp/          # composes the domains into a Toolkit; the binary
    ├── src/
    │   ├── lib.rs      # crate docs + the entire public re-export surface
    │   ├── main.rs     # `supermath-mcp` serve, call, hive-tools, loop new, score proof, check
    │   ├── error/mod.rs        # crate-wide `Error` and `Result<T>`
    │   └── <feature>/          # cli, toolkit, server, config, ...
    │       ├── mod.rs          # module docs, wiring, smallest useful public API
    │       ├── types.rs        # substantial type definitions
    │       └── test.rs         # module-local unit tests
    ├── template/       # the rsi-ralph loop folder `loop new` writes
    └── tests/          # the binary end to end, against loopback stubs
loops/statements/       # conjecture statements to start loops from
bench/formalize/        # formalization benchmark cases, for a later meta-loop
compose.yaml            # CortexDB and the Lean and Python services
compose.rsi.yaml        # mounts rsi-ralph's run store into Lean and Python
docker/                 # the Lean, Python and MCP images
scripts/                # check-crate-deps.sh: the crate dependency rules
docs/
├── specs/              # behavior and architecture specifications
├── plans/              # test-first implementation plans
├── adr/                # immutable architecture decision records
└── research/           # pinned notes on outside work that informs design
```

How the pieces fit and why are in `docs/specs/loops.md`,
`docs/specs/toolkit/crates.md`, and `docs/adr/0010-run-on-rsi-ralph.md`. In
short: each tool domain is its own crate and depends only on
`supermath-tool`; `supermath-mcp` composes them into a toolkit, serves it over
MCP, runs one tool per process for rsi-ralph seats (`call`), writes loop
folders (`loop new`) and scores candidates (`score proof`). The agents, their
sessions and their models live in the `rsi-ralph` binary, so no crate here
depends on OpenHuman. Surplus Intelligence serves every model call, called
directly with concrete model names
(`docs/adr/0008-call-surplus-intelligence-directly.md`); CortexDB holds
memory. `scripts/check-crate-deps.sh` enforces the dependency rules.

Add a crate by creating `crates/<name>/` — `members = ["crates/*"]` picks it up
by existing. Inherit `version`, `edition`, `rust-version`, `license`, and
`repository` from `[workspace.package]`, and opt into the shared lint set with:

```toml
[lints]
workspace = true
```

Each feature area belongs in a focused module directory under a crate's `src/`.
A module root explains the module, wires its pieces together, and exposes the
smallest useful API. Move substantial type definitions into `types.rs` and put
module-local unit tests in a dedicated `test.rs`, wired from the bottom of the
module root with:

```rust
#[cfg(test)]
mod test;
```

Do not accumulate inline `mod tests` blocks in implementation files, and do not
let a general-purpose `utils.rs` or `helpers.rs` grow — those are a symptom of a
missing module. Prefer many small modules that each do one thing well over few
broad ones.

Keep public exports centralized in each crate's `src/lib.rs` so downstream users
have one predictable surface. Put shared error variants in
the crate's `src/error/mod.rs` and return the crate-wide `Result<T>` from
fallible public APIs.

## Build And Test

Run every command from the repository root. These are the contract; CI runs
exactly them, so a green local run should mean a green CI run.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
.github/scripts/check-file-coverage.sh 90 coverage.json
```

Supporting commands:

- `cargo fmt --all` — format before committing.
- `cargo test <filter>` — run a focused subset while iterating.
- `scripts/check-crate-deps.sh` — the crate dependency rules CI also runs.
- `cargo run -p supermath-mcp -- check` — report whether Lean, Python,
  CortexDB, the Gemini key and arXiv answer (see `README.md` for the
  environment).
- `docker compose up --build` — CortexDB and the Lean and Python services.
- `cargo doc --no-deps --workspace` — build the rustdoc CI also builds
  with `RUSTDOCFLAGS="-D warnings"`.
- `cargo test --doc` — run doctests alone when editing documentation examples.

Never skip, ignore, or delete a failing test to make a command pass. Fix the
root cause, or stop and report the blocker.

## Coding Style

Use standard `rustfmt` output and Rust 2024 idioms. Do not hand-format around
`rustfmt`, and do not add `#[rustfmt::skip]` without a comment explaining why.

- `snake_case` for modules, files, functions, methods, fields, and locals.
- `PascalCase` for types, traits, and enum variants; `SCREAMING_SNAKE_CASE` for
  constants and statics.
- Name things for what they are, not for their layer: `RetryPolicy`, not
  `RetryHelper`.
- Prefer small, typed APIs over stringly-typed ones. Accept `&str` and generic
  `impl Into<String>` at boundaries; return owned, concrete types.
- Keep the public surface minimal: default to private, and export deliberately
  from `src/lib.rs`.
- `unsafe` is forbidden workspace-wide by `[workspace.lints]` in the root
  `Cargo.toml`. If a project genuinely needs it, relax the lint in its own
  commit and document every invariant with a `// SAFETY:` comment.

### Errors

- One crate-wide `Error` enum per crate, in `src/error/mod.rs`, built with
  `thiserror`.
- Fallible public functions return `Result<T>`, the crate alias.
- Add a specific variant instead of stuffing context into a string; error
  messages are lowercase, without trailing punctuation.
- Do not `unwrap()`, `expect()`, or `panic!` in library code paths. They are
  fine in tests, examples, and genuinely unreachable states — where `expect`
  must carry a message explaining the invariant.
- Document a `# Errors` section on every public fallible function and a
  `# Panics` section on anything that can panic.

### Dependencies

Adding a dependency is a design decision. Before adding one, check whether the
standard library or an existing dependency already covers the need. When you do
add one:

- pin a caret range (`serde = "1"`), not an exact version;
- enable only the features you need, with `default-features = false` when that
  meaningfully trims the tree;
- gate anything optional behind a Cargo feature, documented in `Cargo.toml`;
- declare it once in the root `[workspace.dependencies]` when more than one
  crate needs it, and take it with `{ workspace = true }`;
- leave a comment above the entry explaining *why* the crate is needed and what
  uses it — see the existing entries for the expected tone;
- prefer well-maintained crates with a compatible license.

Keep `Cargo.lock` committed; this workspace ships a single lockfile so CI and
releases are reproducible.

### rsi-ralph

`rsi-ralph` is a separate binary, not a dependency: the workspace neither
vendors nor links it, and no crate may depend on an `openhuman*` crate. Make
rsi-ralph changes in its own repository; when a loop needs a newer one, say
which commit in `docs/specs/loops.md`.

## Testing

- Module-local unit tests live in `crates/<crate>/src/<feature>/test.rs` and may
  touch private items.
- Integration tests live in `crates/<crate>/tests/` and exercise only the public
  API — they are the regression suite for the crate's contract.
- Payload types pin their serde representation in a unit test. That
  representation is the wire form: a host and a module that disagree about a
  field name fail at runtime with a decode error.
- Use descriptive, behavioral test names: `rejects_an_empty_name`, not
  `test_greet_2`.
- Cover the failure paths, not just the happy path. Every new error variant
  needs a test that produces it.
- For async behavior, standardize on one runtime (`tokio` as a dev-dependency
  for tests) rather than mixing runtimes.
- Tests must be deterministic and independent of network, wall-clock time, and
  execution order. Gate any live/network test behind a feature or an env var and
  name it `live_*` so it is easy to exclude.
- Maintain at least 90% line coverage in every source file. Add or update tests
  with every behavior change, and note any deliberately untested edge case in
  the pull request description.

Write the test first when fixing a bug: a failing test that reproduces the
report, then the fix that turns it green.

## Documentation

Write documentation for the reader who has never seen the code.

- Every public item gets a rustdoc comment. `missing_docs` is a warning that CI
  treats as an error.
- Start every `mod.rs` and `test.rs` with a concise module-level `//!`
  description.
- Each crate's `src/lib.rs` carries its crate-level overview: what the crate
  does, the primary entry points, and a short runnable example. It should also
  say what the crate deliberately does *not* hold, and why.
- Prefer concrete examples over vague description. Doc examples are compiled and
  run by `cargo test`, so they cannot drift.
- Complex modules must include a module-level `README.md` covering their design,
  public surface, and important operational constraints.
- Keep `README.md`, `docs/`, and module docs aligned with code changes in the
  same commit that changes behavior.
- Write accepted behavior and constraints in `docs/specs/` before creating a
  linked, implementation-ordered plan in `docs/plans/`. Specs define what and
  why; plans define how and in what sequence.
- Keep every Markdown file, including this one, at 500 lines or fewer. When a
  topic outgrows that, split it into focused files and link them from the
  nearest `README.md`.

## Git Workflow

- Never commit directly to `main`. Branch first, one branch per logical change.
- Do not use git worktrees in this repository. Work on a branch in the main
  checkout.
- Commit subjects are concise and imperative: `Add retry policy to the client`.
  Keep the subject specific to the change and under ~72 characters.
- Make small, focused commits. Each commit should cover one logical change,
  build independently, and avoid mixing formatting, refactors, and behavior
  changes unless they are inseparable.
- Never commit secrets. `.env` is git-ignored; document new variables in
  `.env.example` with placeholder values.
- Never force-push a shared branch, rewrite published history, or bypass hooks
  with `--no-verify`.

## Pull Requests

Open pull requests ready for review, not as drafts, unless the work genuinely
must not merge yet. A pull request should:

- summarize what changed and why, in a few sentences;
- call out public API or behavior changes explicitly, or state "None";
- list the validation commands actually run, with their outcome;
- link the related issue;
- include updated tests, docs, and examples in the same change.

The template in `.github/PULL_REQUEST_TEMPLATE.md` encodes this checklist.
Address review feedback by fixing it, and reply on each thread describing what
changed. Do not resolve a thread whose feedback you have not addressed or
explicitly declined with a reason.

## Releases

There is no release workflow yet. Do not hand-edit the `version` field in the
root `[workspace.package]` until one exists; every member inherits it with
`version.workspace = true`. Keep `main` green.

## Agent Working Agreement

For automated contributors specifically:

1. **Read before writing.** Inspect the surrounding module and match its
   conventions, comment density, and idiom rather than importing a house style.
2. **Verify, do not assume.** Run the four contract commands and read their
   output before reporting a task complete. Report failures with the output;
   never claim a check passed that you did not run.
3. **Stay in scope.** Implement what was asked. Do not opportunistically
   refactor, reformat, upgrade dependencies, or "fix" unrelated code — raise it
   instead.
4. **No placeholders in delivered code.** No `todo!()`, no stubbed functions, no
   commented-out alternatives left behind. If something cannot be finished, say
   so explicitly.
5. **Do not weaken the guardrails.** Never add blanket `#[allow(...)]`, relax a
   lint, mark a test `#[ignore]`, or loosen CI to get a green run. Fix the
   cause.
6. **Secrets stay out.** Never read, echo, or commit `.env` contents, tokens, or
   credentials, and never paste them into a pull request or issue.
7. **Ask only when blocked.** Make routine judgment calls yourself; escalate
   only irreversible decisions or genuine forks with no clear default.
