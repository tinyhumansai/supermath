# supermath

A math problem-solving agent built on
[OpenHuman](https://github.com/tinyhumansai/openhuman), with durable
per-problem memory in CortexDB.

```text
problem.md ─▶ recall ─▶ OpenHuman agent turn ─▶ VERDICT ─▶ remember
              (CortexDB)   │ chat:  LLM ladder (`flash`, medium effort)  (CortexDB)
                           │ tools: toolkit, deferred ones found by
                           │        tool_search, ranked by Jev (OpenRouter)
```

Each run recalls what memory holds for the problem, runs one OpenHuman turn,
and ends in a three-valued verdict (proved, refuted, or not established) that
is written back to memory.

## Run it

```sh
git submodule update --init --recursive
cp .env.example .env          # ladder key, OpenRouter, Surplus, Venice
docker compose up --build     # ladder + CortexDB + one run on problems/sum-of-odd-numbers.md
PROBLEM=largest-prime-factor docker compose up agent
```

`compose.yaml` runs three services:

| Service | Image | Role |
| --- | --- | --- |
| `ladder` | `ghcr.io/senamakel/llm-ladder-router` | every chat completion and embedding, priced across marketplaces; config from `~/.config/ladder/config.toml` |
| `cortexdb` | `cortexdb/cortexdb:v0.10.4` | memory; extracts with `flash`, embeds with `vectors`, both through the ladder |
| `agent` | built from `Dockerfile` | `supermath-agent solve /problems/<name>.md` |

On the host, against a running ladder and CortexDB:

```sh
export LADDER_API_KEY=... OPENROUTER_API_KEY=... CORTEX_API_KEY=supermath-local
export LADDER_URL=http://localhost:6970/v1
cargo run -p supermath-agent -- solve problems/largest-prime-factor.md
```

The run prints the reply, the verdict, every tool call the turn made, and
the memory event it was stored as. OpenHuman's own logs go to
`$SUPERMATH_WORKSPACE/logs/`.

## Layout

```text
Cargo.toml                 # workspace, lints, OpenHuman's [patch] table
crates/supermath-agent/    # the harness: config, problem, verdict, memory,
                           # toolkit, harness, and the `supermath-agent` binary
vendor/openhuman/          # OpenHuman, a git submodule (embed + Jev ranker)
problems/                  # one Markdown file per problem
compose.yaml, Dockerfile   # the stack
docs/                      # specs, ADRs, and research
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Tests never reach the network: memory tests run against a loopback stub of
CortexDB. A cold build compiles all of OpenHuman, which takes a few minutes.

## Documentation

- [`docs/specs/solving-harness.md`](docs/specs/solving-harness.md): what a
  run does and the invariants it keeps
- [`docs/adr/`](docs/adr/0001-record-architecture-decisions.md): decisions,
  starting with why the harness embeds OpenHuman
- [`docs/research/`](docs/research/README.md): REA and Jacobian, and the
  math-superagent research notes
- [`AGENTS.md`](AGENTS.md): guidelines for humans and agents

## License

GPL-3.0-only. See [LICENSE](LICENSE).
