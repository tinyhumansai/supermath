# supermath-tool

The contract every supermath tool domain implements. It holds no tools and
calls no backend; it depends on nothing heavier than `serde` and `tokio`'s
`sync`, so `cargo test -p supermath-tool` takes seconds and never compiles
OpenHuman. The spec is
[`docs/specs/toolkit/tool/README.md`](../../docs/specs/toolkit/tool/README.md).

## Public surface

| Module | Exports |
| --- | --- |
| `tool` | `Tool`, `Exposure`, `Permission`, `Kind`, `Backend` |
| `envelope` | `Envelope`, `Outcome`, `ErrorCode`, `ErrorInfo`, `Coverage`, `Usage`, `clamp_limit` |
| `budget` | `Budget`, `Demand`, `Refusal` |
| `context` | `CallContext`, `Project`, `EvidenceOrigin`, `Cancellation` |
| `call_log` | `CallLog` (port), `InMemoryCallLog`, `CallRecord` |
| `registry` | `ToolRegistry` |
| `contract` | `check`, `violations`, `Fixture`, `Violation` |

## Design constraints

- `Tool::call` never fails as a Rust value; a failure is an `error` envelope
  with a stable `ErrorCode`, and `unknown` is never a negative answer.
- The envelope's wire form is pinned by unit tests: a host and a client that
  disagree about a field name fail at runtime.
- A `Project` is built by a host and handed to tools in `CallContext`; a tool
  never takes the project from its arguments.
- The contract checker's schema check is structural (an object schema,
  `additionalProperties: false`, `required` within `properties`), not a full
  JSON Schema validation.
