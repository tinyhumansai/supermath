//! The tool contract every supermath domain crate implements.
//!
//! A tool is a named, schema'd asynchronous function from JSON arguments to
//! an [`Envelope`]. This crate holds the types around that idea and nothing
//! else: no tool, no backend call.
//!
//! # Entry points
//!
//! - [`Tool`] is the trait a domain crate implements per tool.
//! - [`Envelope`], with [`Envelope::ok`], [`Envelope::unknown`], and
//!   [`Envelope::error`], is the one answer shape.
//! - [`CallContext`] carries the [`Project`], [`Budget`], and cancellation a
//!   host hands each call.
//! - [`ToolRegistry`] holds tools by name; [`CallLog`] records calls.
//! - [`contract::check`] is run over every tool by its crate's tests.
//!
//! # Example
//!
//! ```
//! use async_trait::async_trait;
//! use serde_json::{Value, json};
//! use supermath_tool::{
//!     CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool,
//! };
//!
//! struct Double;
//!
//! #[async_trait]
//! impl Tool for Double {
//!     fn name(&self) -> &'static str { "calc_double" }
//!     fn description(&self) -> &'static str { "Given an integer, returns twice it." }
//!     fn schema(&self) -> Value {
//!         json!({"type": "object", "properties": {"n": {"type": "integer"}},
//!                "required": ["n"], "additionalProperties": false})
//!     }
//!     fn exposure(&self) -> Exposure { Exposure::Deferred }
//!     fn permission(&self) -> Permission { Permission::Read }
//!     fn kind(&self) -> Kind { Kind::Primitive }
//!     fn errors(&self) -> &'static [ErrorCode] { &[ErrorCode::InvalidInput] }
//!     async fn call(&self, _ctx: &CallContext, args: Value) -> Envelope {
//!         match args["n"].as_i64() {
//!             Some(n) => Envelope::ok(json!({"result": n * 2})),
//!             None => Envelope::error(ErrorCode::InvalidInput, "n must be an integer"),
//!         }
//!     }
//! }
//! ```
//!
//! # What this crate does not hold
//!
//! No tools, and no backend client: those belong to the domain crates. No
//! `OpenHuman` or MCP types either; the runner and the MCP server adapt
//! [`Tool`] at their own edges, so this crate compiles in seconds.

mod budget;
mod call_log;
mod context;
pub mod contract;
mod envelope;
mod error;
#[cfg(test)]
mod fake;
mod registry;
mod tool;

pub use budget::{Budget, Demand, Refusal};
pub use call_log::{CallLog, CallRecord, InMemoryCallLog};
pub use context::{CallContext, Cancellation, EvidenceOrigin, Project};
pub use envelope::{
    Coverage, DEFAULT_LIMIT, Envelope, ErrorCode, ErrorInfo, MAX_LIMIT, MAX_NEXT, Outcome, Usage,
    clamp_limit,
};
pub use error::{Error, Result};
pub use registry::ToolRegistry;
pub use tool::{Backend, Exposure, Kind, Permission, Tool};
