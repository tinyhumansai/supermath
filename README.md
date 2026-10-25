# supermath

A toolkit for settling math conjectures, and the loops that use it. The
agents run in [`rsi-ralph`](https://github.com/tinyhumansai/rsi-ralph):
a hive of seats works on a frozen Lean statement over epochs, and a scorer
promotes only what Lean checks. supermath supplies the tools (project files,
Lean, Python, arXiv and web search, the claim ledger, exact arithmetic), the
scorer, and the loop template.

```text
rsi-ralph run loops/<slug> ── seats (prover, refuter, lead) ── models ──▶ Surplus Intelligence
                     │  tools: `supermath-mcp call <tool>`, one process per call
                     │           ├─ lean_* ─▶ Lean service   py_* ─▶ Python service
                     │           └─ ledger_*, recall_memory ─▶ CortexDB (embeds + extracts via Surplus)
                     └─ scorer: `supermath-mcp score proof` ─▶ Lean service
```

## Settle a conjecture with an rsi-ralph loop

`supermath-mcp` writes a loop folder for one conjecture:
- the statement is frozen;
- a prover and a refuter each work in their own checkout;
- every toolkit tool is offered as an executable tool;
- a scorer accepts a side only when Lean checks its theorem at exactly the
  pinned type.

```sh
cp .env.example .env          # the Surplus Intelligence key
cargo install --path crates/supermath-mcp
docker compose -f compose.yaml -f compose.rsi.yaml --profile host up -d lean python cortexdb
supermath-mcp loop new gauss-sum loops/statements/gauss-sum.lean --env-file .env
TINYRSI_UPSTREAM=https://api.surplusintelligence.ai/v1 rsi-ralph run loops/gauss-sum
```

It needs an `rsi-ralph` built from `e40c7e3` or later. Details:
[`docs/specs/loops.md`](docs/specs/loops.md).

`compose.yaml` runs three services by default:

| Service | Image | Role |
| --- | --- | --- |
| `cortexdb` | `cortexdb/cortexdb:v0.10.4` | memory; extracts with `deepseek-v4.1-flash` and embeds with `venice-embed-1`, both on Surplus |
| `lean` | built from `docker/lean` | the pinned Lean toolchain and Mathlib, no egress |
| `python` | built from `docker/python` | the pinned Python solvers, no egress |

The `host` profile publishes Lean and Python on loopback for tools running
on the host, and `compose.rsi.yaml` mounts rsi-ralph's run store into both,
so they can check a candidate checkout.

## Use the tools from Claude Code (MCP)

`supermath-mcp` also serves the whole toolkit over MCP on stdio:

```sh
cargo install --path crates/supermath-mcp
claude mcp add supermath -- supermath-mcp serve my-project
supermath-mcp check            # Lean, Python, CortexDB, Gemini key, arXiv
```

The project comes from the command line, never from a tool argument. A
service that is not configured keeps its tools in the list; they answer
`BACKEND_UNAVAILABLE`. Point it at the stack with `LEAN_URL`, `PYTHON_URL`,
`CORTEX_API_URL` and `CORTEX_API_KEY`, and `SUPERMATH_PROJECTS` for the
project workspaces (see `.env.example`). From the compose stack instead:

```sh
claude mcp add supermath -- docker compose --profile mcp run --rm -T mcp serve my-project
```

Details: [`docs/specs/toolkit/mcp/README.md`](docs/specs/toolkit/mcp/README.md).

## Layout

```text
Cargo.toml                 # workspace and lints
crates/supermath-mcp/      # composes the tool domains; serve, call, loop new, score proof
crates/supermath-*/        # one crate per tool domain, and the tool contract
loops/statements/          # conjecture statements to start loops from
bench/formalize/           # the formalization benchmark cases
compose.yaml, docker/      # CortexDB and the Lean, Python and MCP images
docs/                      # specs, ADRs, and research
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Tests never reach the network: every backend runs against loopback stubs.

## Documentation

- [`docs/specs/loops.md`](docs/specs/loops.md): a conjecture as an rsi-ralph
  loop, its scorer, and the toolkit as executable tools
- [`docs/specs/toolkit/`](docs/specs/toolkit/README.md): every tool, one
  folder per crate
- [`docs/adr/`](docs/adr/0001-record-architecture-decisions.md): decisions,
  up to why the agents run in rsi-ralph
  ([0010](docs/adr/0010-run-on-rsi-ralph.md))
- [`docs/research/`](docs/research/README.md): rsi-ralph, REA and Jacobian,
  and the math-superagent research notes
- [`AGENTS.md`](AGENTS.md): guidelines for humans and agents

## License

GPL-3.0-only. See [LICENSE](LICENSE).
