# supermath-agent

The supermath agent scaffold. It embeds the
[OpenHuman](https://github.com/tinyhumansai/openhuman) runtime and wires it to:

- the **LLM ladder** for every chat completion and embedding (`flash` at
  `medium` reasoning effort by default);
- **Jev**, the decision model OpenRouter serves, which ranks `tool_search`
  over the agent's deferred tools;
- a small **toolkit** of host tools;
- **CortexDB** for durable memory, scoped per project.

What the agent does with them is not designed yet. The binary only checks
that everything is wired:

```sh
supermath-agent check [project]     # exit 0 when the stack is ready
```

The behavior is specified in
[`docs/specs/agent-runner.md`](../../docs/specs/agent-runner.md); why it is
built on OpenHuman is in
[`docs/adr/0002-build-on-openhuman.md`](../../docs/adr/0002-build-on-openhuman.md).

## Layout

| Module | Holds |
| --- | --- |
| `config/` | settings read from the environment |
| `memory/` | the CortexDB client: project scopes, recall, indexed writes, readiness |
| `toolkit/` | `recall_memory` (direct), `factor_integer` and `gcd_lcm` (deferred) |
| `runner/` | the OpenHuman runtime, the Jev ranker, and agent registration |
| `main.rs` | `supermath-agent check` |

## Developing

From the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Unit tests run memory against a one-request loopback stub; `tests/check.rs`
runs the binary against loopback stubs of the ladder, CortexDB, and the
OpenHuman backend.
