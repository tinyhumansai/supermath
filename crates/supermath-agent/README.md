# supermath-agent

A math problem-solving agent harness. It embeds the
[OpenHuman](https://github.com/tinyhumansai/openhuman) runtime for the agent
loop and keeps durable, per-problem memory in CortexDB.

```text
problem.md ─▶ recall (CortexDB) ─▶ OpenHuman agent turn ─▶ VERDICT ─▶ remember (CortexDB)
                                     └─ recall_memory tool ─┘
```

One run:

1. loads a problem file; its stem is the slug that names its memory;
2. recalls what earlier runs recorded at
   `{CORTEX_SCOPE_ROOT}/project:{slug}/app:brain`;
3. runs one OpenHuman turn with a solver prompt and a single read-only host
   tool, `recall_memory`, bound to that scope;
4. reads `VERDICT: proved | refuted | not-established` from the last line of
   the reply (anything else is *not established*, never a positive answer);
5. writes the verdict and reply back, waiting until CortexDB has indexed it.

The behavior is specified in
[`docs/specs/solving-harness.md`](../../docs/specs/solving-harness.md), and the
reason this crate is its own Cargo workspace is in
[`docs/adr/0002-separate-agent-workspace.md`](../../docs/adr/0002-separate-agent-workspace.md).

## Running

With Docker (recommended), from the repository root:

```sh
git submodule update --init --recursive
cp .env.example .env    # set SUPERMATH_API_KEY and SUPERMATH_MODEL
docker compose up --build
```

On the host, with a CortexDB already listening:

```sh
cd crates/supermath-agent
export SUPERMATH_API_KEY=... SUPERMATH_MODEL=... CORTEX_API_KEY=...
cargo run --release -- solve ../../problems/sum-of-odd-numbers.md
```

Exit status: `0` proved or refuted, `2` not established, `1` on error.

## Developing

Run these from `crates/supermath-agent`; the root workspace does not build
this crate.

```sh
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The tests never reach the network. Memory tests run against a loopback stub
of the CortexDB endpoints.

## Layout

| Module | Holds |
| --- | --- |
| `config/` | settings read from the environment |
| `problem/` | problem loading and slug derivation |
| `verdict/` | the three-valued verdict and its parser |
| `memory/` | the CortexDB client: scoped recall and indexed writes |
| `harness/` | the OpenHuman runtime, solver prompt, and `recall_memory` tool |
| `main.rs` | the `supermath-agent solve` command |
