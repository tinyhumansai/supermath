//! How a [`Tool`] and an [`Envelope`] look on the MCP wire.

use std::sync::Arc;

use rmcp::model::{CallToolResult, ContentBlock, JsonObject, MetaObject, ToolAnnotations};
use serde_json::{Value, json};
use supermath_tool::{Backend, Envelope, Kind, Outcome, Permission, Tool};

/// The key under a tool's `_meta` that holds what MCP has no field for.
pub(crate) const META_KEY: &str = "supermath";

/// The wire name of a tool's [`Kind`]: `primitive`, or `agentic:<backend>`.
fn kind_name(kind: Kind) -> String {
    match kind {
        Kind::Primitive => "primitive".to_owned(),
        Kind::Agentic(Backend::Gemini) => "agentic:gemini".to_owned(),
        Kind::Agentic(Backend::OpenHuman) => "agentic:openhuman".to_owned(),
    }
}

/// `tools/list` entry for `tool`: name, description, input schema,
/// `readOnlyHint` from its permission, and `_meta.supermath` with its
/// exposure and kind.
pub(crate) fn describe(tool: &dyn Tool) -> rmcp::model::Tool {
    let schema = match tool.schema() {
        Value::Object(object) => object,
        _ => JsonObject::new(),
    };
    let read_only = tool.permission() == Permission::Read;
    let mut described = rmcp::model::Tool::new(tool.name(), tool.description(), Arc::new(schema))
        .with_annotations(ToolAnnotations::new().read_only(read_only));
    let mut meta = MetaObject::new();
    meta.0.insert(
        META_KEY.to_owned(),
        json!({
            "exposure": tool.exposure(),
            "kind": kind_name(tool.kind()),
        }),
    );
    described.meta = Some(meta);
    described
}

/// The result of a `tools/call`: the envelope as JSON text with the call's
/// id added as `call_id`, and `isError` set when the envelope is an `error`.
/// An `unknown` envelope is a real answer, so it is not an error.
pub(crate) fn result(call_id: &str, envelope: &Envelope) -> CallToolResult {
    let content = vec![ContentBlock::text(body(call_id, envelope).to_string())];
    if envelope.outcome == Outcome::Error {
        CallToolResult::error(content)
    } else {
        CallToolResult::success(content)
    }
}

/// The envelope as JSON with the call's id added as `call_id`: what every
/// caller sees, over MCP or from `supermath-mcp call`.
pub(crate) fn body(call_id: &str, envelope: &Envelope) -> Value {
    let mut body = serde_json::to_value(envelope).unwrap_or(Value::Null);
    if let Value::Object(object) = &mut body {
        object.insert("call_id".to_owned(), Value::String(call_id.to_owned()));
    }
    body
}

#[cfg(test)]
mod test;
