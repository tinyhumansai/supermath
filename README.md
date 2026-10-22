# supermath

A scaffold for a math agent built on
[OpenHuman](https://github.com/tinyhumansai/openhuman): the runtime, model
routing, tool selection, a small toolkit, and durable memory, wired together.
How the agent works on a problem is not designed yet.

```text
supermath-agent ── OpenHuman runtime ── chat + embeddings ──▶ LLM ladder (`flash`, medium effort)
                     │  agent per project
                     │  tools: toolkit (deferred ones via tool_search)
                     │  tool_search ranked by Jev ──────────▶ OpenRouter
                     └─ memory ─────────────────────────────▶ CortexDB (embeds + extracts via the ladder)
```

## Run it

```sh
git submodule update --init --recursive
cp .env.example .env          # ladder key, OpenRouter, Surplus, Venice
docker compose up --build     # ladder + CortexDB, then `supermath-agent check`
```

`compose.yaml` runs three services:

| Service | Image | Role |
| --- | --- | --- |
| `ladder` | `ghcr.io/senamakel/llm-ladder-router` | every chat completion and embedding; config from `~/.config/ladder/config.toml`, published on 6970 |
| `cortexdb` | `cortexdb/cortexdb:v0.10.4` | memory; extracts with `flash`, embeds with `vectors`, both through the ladder |
| `agent` | built from `Dockerfile` | `supermath-agent check`: boots the runner and reports readiness |

On the host, against a running ladder and CortexDB:

```sh
export LADDER_API_KEY=... OPENROUTER_API_KEY=... CORTEX_API_KEY=supermath-local
export LADDER_URL=http://localhost:6970/v1
cargo run -p supermath-agent -- check
```

OpenHuman's logs go to `$SUPERMATH_WORKSPACE/logs/` (default `.supermath/`).

## Use the tools from Claude Code (MCP)

`supermath-mcp` serves the whole toolkit (project files, Lean, Python, arXiv
and web search, the claim ledger, exact arithmetic) over MCP on stdio. It does
not build OpenHuman, so it installs in a minute rather than ten:

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
Cargo.toml                 # workspace, lints, OpenHuman's [patch] table
crates/supermath-mcp/      # composes the tool domains; `supermath-mcp serve`
crates/supermath-agent/    # config, memory, toolkit, runner, `check` binary
vendor/openhuman/          # OpenHuman, a git submodule
compose.yaml, Dockerfile   # the stack
docs/                      # specs, ADRs, and research
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Tests never reach the network: memory and the binary run against loopback
stubs. A cold build compiles all of OpenHuman, which takes a few minutes.

## Documentation

- [`docs/specs/agent-runner.md`](docs/specs/agent-runner.md): what is wired,
  the configuration, and the invariants
- [`docs/adr/`](docs/adr/0001-record-architecture-decisions.md): decisions,
  starting with why the agent embeds OpenHuman
- [`docs/research/`](docs/research/README.md): REA and Jacobian, and the
  math-superagent research notes
- [`AGENTS.md`](AGENTS.md): guidelines for humans and agents

## License

GPL-3.0-only. See [LICENSE](LICENSE).
