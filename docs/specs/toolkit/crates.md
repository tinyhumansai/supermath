# Crates: one per tool domain

- **Status:** Implemented
- **Parent:** [`README.md`](README.md)
- **Decisions:** [`../../adr/0006-one-crate-per-tool-domain.md`](../../adr/0006-one-crate-per-tool-domain.md),
  [`../../adr/0007-run-python-solvers-as-a-pinned-service.md`](../../adr/0007-run-python-solvers-as-a-pinned-service.md),
  [`../../adr/0010-run-on-rsi-ralph.md`](../../adr/0010-run-on-rsi-ralph.md)

Each tool domain is its own crate, with its own spec folder, its own tests,
and its own coverage. A small core crate holds the contract every domain
implements; `supermath-mcp` composes the domains and serves them. The
agents run outside this repository, in the `rsi-ralph` binary
([`../loops.md`](../loops.md)).

```text
crates/
├── supermath-tool/         # the contract: Tool, Envelope, ErrorCode, Kind, Usage, Budget, Project
├── supermath-files/        # the project workspace: Lean and Python files, path rules, write log
├── supermath-lean/         # the Lean service client and the lean_* tools
├── supermath-python/       # the Python solver service client and the py_* tools
├── supermath-literature/   # arXiv, Semantic Scholar, OEIS and the lit_* tools
├── supermath-web/          # Gemini grounded search, page reading and the web_* tools
├── supermath-ledger/       # CortexDB memory, claims, statuses and the ledger_* tools
├── supermath-calc/         # small exact arithmetic: calc_factor, calc_gcd_lcm
└── supermath-mcp/          # composes the domains into a Toolkit; `serve`, `call`, `loop new`, `score proof`
```

| Crate | Spec | Kind of tools |
| --- | --- | --- |
| `supermath-tool` | [`tool/`](tool/README.md) | none; the contract |
| `supermath-files` | [`files/`](files/README.md) | primitive |
| `supermath-lean` | [`lean/`](lean/README.md) | primitive |
| `supermath-python` | [`python/`](python/README.md) | primitive |
| `supermath-literature` | [`literature/`](literature/README.md) | primitive |
| `supermath-web` | [`web/`](web/README.md) | agentic (Gemini) and primitive |
| `supermath-ledger` | [`ledger/`](ledger/README.md) | primitive |
| `supermath-calc` | [`calc/`](calc/README.md) | primitive |
| `supermath-mcp` | [`mcp/`](mcp/README.md) | none; composition, serving, and loops |

## Dependency rules

```text
                         supermath-tool
     ┌────────┬───────────┬────┴────┬────────┐
   files  literature     web     ledger    calc
   ┌─┴──┐
 lean  python
     └────────── every domain ─────────────┐
                                     supermath-mcp
```

- Every domain crate depends on `supermath-tool`. `lean` and `python` also
  depend on `supermath-files`. **No other domain crate depends on another
  domain crate.**
- Where one domain needs another's evidence, it defines a port (a trait),
  and `supermath-mcp` wires the real implementation: the ledger's
  `LeanEvidence` ([`ledger/`](ledger/README.md#crate)) and the files
  crate's `FileObserver` ([`files/`](files/README.md#observers)). Each
  crate is tested against fakes of its ports.
- No crate depends on an `openhuman*` crate.
- Only `supermath-web` calls a model (Gemini, as its search backend;
  [ADR 0005](../../adr/0005-search-the-web-with-gemini-grounding.md)).

CI enforces the rules with `scripts/check-crate-deps.sh` over
`cargo metadata`: it fails if a domain crate (`files`, `lean`, `python`,
`literature`, `web`, `ledger`, `calc`) depends on a `supermath-*` crate
other than `supermath-tool` (and, for `lean` and `python`,
`supermath-files`), or if any crate depends on an `openhuman*` crate.

## Testing each crate on its own

Each crate is specified, tested, and gated separately:

- **Spec.** Its folder under `docs/specs/toolkit/` holds a `README.md` with
  the domain's shared rules and a `## Crate` section (name, dependencies,
  public API, configuration, tests), and a `tools/` folder with one file
  per tool.
- **Unit tests** in `src/<module>/test.rs`, and **integration tests** in
  the crate's `tests/`, exercising only its public API against loopback
  stubs of its backend and fakes of its ports.
- **Contract tests.** Every domain crate runs
  [`supermath-tool`'s contract checker](tool/README.md#contract-tests) over
  its own tools.
- **Live tests** are named `live_<domain>_*` and gated by the domain's
  environment variables.
- **Coverage** is per crate and per file: at least 90% line coverage,
  reported by `cargo llvm-cov -p <crate>`.

```sh
cargo test -p supermath-lean            # one domain, seconds
cargo test --workspace                  # everything
```

## Compose and images

| Service | Image | Role |
| --- | --- | --- |
| `cortexdb` | pinned by digest | memory |
| `lean` | `docker/lean/` | the pinned Lean service ([`lean/`](lean/README.md#the-lean-service)) |
| `python` | `docker/python/` | the pinned Python solver service ([`python/`](python/README.md#the-python-service)) |
| `mcp` | `docker/mcp/Dockerfile` | `supermath-mcp serve` on stdio; opt-in (`--profile mcp`) |
| `lean-gateway`, `python-gateway` | `alpine/socat` | publish `lean` and `python` on loopback for `supermath-mcp` on the host; opt-in (`--profile host`) |

`mcp`, `lean`, and `python` share the `projects` volume: the toolkit writes
files there through `file_write` and `file_edit`; the language services
build and run them ([`files/`](files/README.md#layout)). `python` mounts it
read-only; `lean` writes only Lake build outputs. Both sit only on an
`internal: true` network and have no egress; `mcp` joins it to reach them.

`compose.rsi.yaml` is an override for loops: it bind-mounts the rsi-ralph
run store read-only into `lean` and `python` and sets `SUPERMATH_ROOTS`
([`../loops.md`](../loops.md#running-a-loop)).

## Acceptance criteria

- The dependency-rules script passes in CI, and fails on a fixture
  workspace that breaks each rule (`scripts/test-check-crate-deps.sh`).
- Every domain crate runs the contract checker over its tools.
- `cargo test -p <crate>` passes for each crate on its own.

## Open questions

1. Should `supermath-calc` fold into `supermath-python` once Python is
   available, or stay as the dependency-free fast path?
