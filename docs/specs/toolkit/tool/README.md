# Tool: the contract every domain implements

- **Status:** Implemented
- **Crate:** `supermath-tool`
- **Parent:** [`../README.md`](../README.md)

The smallest crate in the toolkit: the types every domain crate builds
tools from, and the test helper that checks a domain kept the
[tool contract](../README.md#tool-contract). It holds no tools and calls
no backend.

## The `Tool` trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn schema(&self) -> serde_json::Value;   // JSON Schema of the arguments
    fn exposure(&self) -> Exposure;          // Direct | Deferred
    fn permission(&self) -> Permission;      // Read | Write
    fn kind(&self) -> Kind;                  // Primitive | Agentic(Backend)
    fn errors(&self) -> &'static [ErrorCode]; // the codes it can return
    async fn call(&self, ctx: &CallContext, args: serde_json::Value) -> Envelope;
}
```

- `call` never returns `Err`: a failure is an `error` envelope with a code
  ([`../README.md`](../README.md#one-envelope)).
- `CallContext` carries the `Project` (slug and scope, set by the host,
  never from arguments), the `call_id` for the call log, the caller's
  `evidence_origin` (`Collected` or `Supplied`), a `Budget`, and a
  `Cancellation` signal (a cloneable watch channel; `tokio` `sync` only).
- `Kind::Agentic` names its backend (`Gemini` or `OpenHuman`), and an
  agentic tool's description must begin "Runs a … turn"
  ([kinds](../README.md#primitive-and-agentic-tools)).

## Types

| Type | Holds |
| --- | --- |
| `Envelope` | `outcome`, `data`, `coverage`, `unknowns`, `next`, `cursor`, `usage`, `error`; constructors `ok`, `unknown`, `error`, and `with_*` builders |
| `ErrorInfo` | the `error` field: `code`, `message`, `retry_after_ms`; present exactly when `outcome` is `error` |
| `Outcome` | `Ok`, `Unknown`, `Error` |
| `ErrorCode` | the stable codes of [`../README.md`](../README.md#one-envelope); `#[non_exhaustive]` |
| `Coverage` | `searched`, `not_covered`, `cached_age` |
| `Usage` | `model`, `turns`, `tool_calls`, `input_tokens`, `output_tokens`, `elapsed_ms`; required on agentic tools |
| `Budget` | `wall_time_ms`, `rows`, `bytes`, and for agentic tools `iterations` and `tokens`; `admit(&Demand)` returns a `Refusal`, which converts to a `BUDGET_EXCEEDED` envelope |
| `Project` | slug (`[a-z0-9-]+`) and scope root; constructed only by a host |
| `ToolRegistry` | tools by name; what the composition hands to `supermath-subagents` |
| `CallLog` | a port recording one `CallRecord` per call, by `call_id`; `InMemoryCallLog` keeps them for the process; `ToolRegistry::dispatch` logs each call |

## Contract tests

`supermath_tool::contract::check(&dyn Tool, domain, &[Fixture])` is run by
every domain crate over each of its tools; `domain` is the crate's prefix
(`file`, `lean`, `py`, `lit`, `web`, `ledger`, `calc`, or `agent`). A
`Fixture` is a `CallContext` and arguments (`Fixture::new(args)` uses a
scratch project). `check` returns `Error::Contract` listing every
`Violation`; `violations` returns the list itself. It asserts:

- the name matches `^(file|lean|py|lit|web|ledger|calc|agent)_[a-z_]+$` or
  is `recall_memory`, and its prefix matches the crate's domain;
- the description's first sentence states the question: a primitive's
  begins "Given " (given X, returns Y), an agentic tool's begins "Runs a"
  (`Violation::PrimitiveDescription`, `Violation::AgenticDescription`);
  `contract::description_violation` applies this rule alone, for catalogue
  tests;
- the schema is an object schema with `additionalProperties: false` and
  `required` names drawn from `properties` (a structural check, not full
  JSON Schema validation);
- every code in `errors()` is produced by at least one fixture, and no
  fixture produces a code not in `errors()`;
- every envelope serializes to the pinned form and back;
- every envelope is well formed (`error` present exactly when the outcome
  is `error`; at most three `next` calls);
- an agentic tool's envelopes all carry `usage`;
- at least one fixture is given.

Lists share `clamp_limit`: default 10, at most 50.

## Crate

| | |
| --- | --- |
| Name | `supermath-tool` |
| Depends on | `async-trait`, `serde`, `serde_json`, `thiserror`, `tokio` (cancellation only) |
| Public API | the trait and types above, and `contract` |
| Tests | serde forms of every type; `Budget::admit`; the contract checker against a passing and a failing fake tool |

## Open questions

1. Should `Envelope::data` be typed per tool (a generic parameter) and
   erased only at the MCP and OpenHuman boundaries?
