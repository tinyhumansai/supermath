//! The [`Tool`] trait and the small enums that describe a tool.
//!
//! A tool is a named, schema'd, asynchronous function from JSON arguments to
//! an [`Envelope`]. Every domain crate implements this trait; the runner and
//! the MCP server adapt it. See `docs/specs/toolkit/tool/README.md`.

mod types;

use async_trait::async_trait;

use crate::{CallContext, Envelope, ErrorCode};

pub use types::{Backend, Exposure, Kind, Permission};

/// One callable tool.
///
/// `call` never fails as a Rust value: a failure is an `error` envelope with a
/// stable [`ErrorCode`], and a result that could not be established is an
/// `unknown` envelope, never a negative answer.
#[async_trait]
pub trait Tool: Send + Sync {
    /// The tool's unique name, e.g. `calc_gcd_lcm`.
    fn name(&self) -> &'static str;
    /// What the tool does. The first sentence states the question it answers;
    /// an agentic tool's begins "Runs a".
    fn description(&self) -> &'static str;
    /// JSON Schema of the arguments (an object with
    /// `additionalProperties: false`).
    fn schema(&self) -> serde_json::Value;
    /// Whether the tool is always in the model's catalogue or found by search.
    fn exposure(&self) -> Exposure;
    /// Whether the tool changes state.
    fn permission(&self) -> Permission;
    /// Whether the tool is a deterministic primitive or runs a model turn.
    fn kind(&self) -> Kind;
    /// Every error code the tool can return.
    fn errors(&self) -> &'static [ErrorCode];
    /// Runs the tool. `ctx` comes from the host; `args` is untrusted JSON.
    async fn call(&self, ctx: &CallContext, args: serde_json::Value) -> Envelope;
}

#[cfg(test)]
mod test;
