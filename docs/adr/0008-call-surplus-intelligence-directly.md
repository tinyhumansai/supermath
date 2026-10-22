# 8. Call Surplus Intelligence directly

- **Status:** Proposed
- **Date:** 2026-10-22
- **Supersedes:** the ladder routing in [ADR 0002](0002-build-on-openhuman.md)

## Context

[ADR 0002](0002-build-on-openhuman.md) routed every chat completion and
embedding through the LLM ladder, a separate router service that picks a
marketplace and a model for a named ladder such as `flash`. In practice the
ladder's chat rungs were nearly all Surplus Intelligence anyway, and it added a
service to run, a config file bind-mounted from the operator's machine, and a
layer of names (`flash`, `formalize`, `vectors`) that hid which model actually
answered. The `formalize` rung the sub-agents defaulted to was never defined
in that config, so `agent_formalize` had no model behind it.

Surplus Intelligence serves an OpenAI-compatible API
(`https://api.surplusintelligence.ai/v1`) for both chat completions and
embeddings, which is all OpenHuman and CortexDB need.

## Decision

- Every chat completion and embedding, the agent's, its `agent_*` children's
  and CortexDB's, goes straight to Surplus Intelligence with `SURPLUS_API_KEY`.
- Configuration names concrete Surplus models, not ladder names:

  | Use | Variable | Default |
  | --- | --- | --- |
  | top-level turns | `SUPERMATH_MODEL` | `deepseek-v4.1-flash` |
  | paper reader | `SUPERMATH_READER_MODEL` | `deepseek-v4.1-flash` |
  | formalizer and refuter | `SUPERMATH_FORMALIZER_MODEL` | `deepseek-v4-pro` |
  | alignment reviewer | `SUPERMATH_REVIEWER_MODEL` | `glm-5.3`, a different model from the formalizer's |
  | embeddings (agent and CortexDB) | `SUPERMATH_EMBEDDING_MODEL` | `venice-embed-1`, 1024 wide |
  | CortexDB extraction and answers | `CORTEX_CHAT_MODEL` | `deepseek-v4.1-flash` |

- The `ladder` compose service is removed.
- Jev stays on OpenRouter. It is a hosted decision model, and OpenHuman's Jev
  ranker reaches it only through TinyHumans, TypeSafe or OpenRouter
  (`openhuman-tinyhumans/src/jev/route.rs`). Moving it needs a route added in
  OpenHuman, not a change here.

## Consequences

- One provider, one key, one less service, and the model that answers is the
  model that is configured.
- No failover. When Surplus has no healthy seller for a model, the call fails
  rather than moving to another marketplace. On 2026-10-22 `venice-embed-1`
  answered `no_healthy_sellers`, so CortexDB could not embed until it
  recovered.
- `venice-embed-1` is a different embedding space from the ladder's BGE-M3.
  compose therefore mounts a new volume, `cortexdb-venice-embed-1`, instead of
  the ladder-era `cortexdb-data`, so an upgraded deployment starts an empty
  store rather than mixing spaces in one index. Memory recorded through the
  ladder is not carried over.
- Price ceilings and per-seller discounts were the ladder's job and are gone;
  Surplus prices each request at its own market rate.
