//! `supermath-mcp call`: one tool call as one process.
//!
//! The argument object arrives on standard input as JSON; empty input is
//! `{}`. The answer on standard output is exactly the body an MCP
//! `tools/call` carries: the envelope with `call_id` added. Input that is
//! not a JSON object never reaches a tool. It answers an `INVALID_INPUT`
//! envelope with no `call_id`, and the call log does not record it.
//!
//! Every envelope, `error` ones included, is a normal answer: the process
//! exits 0 whenever it prints one, because an executable-tool host may drop
//! standard output of a failed process, and the envelope is what the model
//! needs to read. Only a host failure (an unreadable `--env-file`, an
//! invalid project) exits non-zero.

use serde_json::{Map, Value};
use supermath_tool::{Envelope, ErrorCode};

use crate::server::body;
use crate::toolkit::Toolkit;

/// Calls `tool` with the arguments in `input` and returns the answer body.
pub async fn call(toolkit: &Toolkit, tool: &str, input: &str) -> Value {
    let args = match arguments(input) {
        Ok(args) => args,
        Err(envelope) => return serde_json::to_value(envelope).unwrap_or(Value::Null),
    };
    let done = toolkit.dispatch(tool, args).await;
    body(&done.call_id, &done.envelope)
}

/// The argument object `input` holds, or the envelope refusing it.
fn arguments(input: &str) -> Result<Value, Envelope> {
    if input.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    match serde_json::from_str(input) {
        Ok(Value::Object(object)) => Ok(Value::Object(object)),
        Ok(_) => Err(Envelope::error(
            ErrorCode::InvalidInput,
            "arguments must be a JSON object",
        )),
        Err(error) => Err(Envelope::error(
            ErrorCode::InvalidInput,
            format!("arguments are not JSON: {error}"),
        )),
    }
}

#[cfg(test)]
mod test;
