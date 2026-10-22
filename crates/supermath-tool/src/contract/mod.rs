//! The contract checker every domain crate runs over its tools.
//!
//! A domain crate's test builds a [`Fixture`] for each behaviour a tool has
//! (success, each error code it can return) and calls [`check`]:
//!
//! ```
//! # use async_trait::async_trait;
//! # use serde_json::{Value, json};
//! # use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};
//! # use supermath_tool::contract::{self, Fixture};
//! # struct Double;
//! # #[async_trait]
//! # impl Tool for Double {
//! #     fn name(&self) -> &'static str { "calc_double" }
//! #     fn description(&self) -> &'static str { "Given an integer, returns twice it." }
//! #     fn schema(&self) -> Value {
//! #         json!({"type": "object", "properties": {"n": {"type": "integer"}},
//! #                "required": ["n"], "additionalProperties": false})
//! #     }
//! #     fn exposure(&self) -> Exposure { Exposure::Deferred }
//! #     fn permission(&self) -> Permission { Permission::Read }
//! #     fn kind(&self) -> Kind { Kind::Primitive }
//! #     fn errors(&self) -> &'static [ErrorCode] { &[ErrorCode::InvalidInput] }
//! #     async fn call(&self, _ctx: &CallContext, args: Value) -> Envelope {
//! #         match args["n"].as_i64() {
//! #             Some(n) => Envelope::ok(json!({"result": n * 2})),
//! #             None => Envelope::error(ErrorCode::InvalidInput, "n must be an integer"),
//! #         }
//! #     }
//! # }
//! let fixtures = [
//!     Fixture::new(json!({"n": 12})),
//!     Fixture::new(json!({})), // produces INVALID_INPUT
//! ];
//! let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
//! runtime
//!     .block_on(contract::check(&Double, "calc", &fixtures))
//!     .unwrap();
//! ```
//!
//! [`check`] asserts that the tool:
//!
//! - is named `<domain>_<words>` for its crate's domain (or is
//!   `recall_memory` in the ledger);
//! - has a description whose first sentence names the question: a
//!   primitive's begins "Given " (given X, returns Y), an agentic tool's
//!   begins "Runs a";
//! - has a schema that is an object schema with `additionalProperties:
//!   false` and whose `required` names are all `properties`;
//! - returns every code in `errors()` from at least one fixture, and no code
//!   outside `errors()`;
//! - returns well-formed envelopes that serialise to the pinned form and
//!   back unchanged;
//! - carries `usage` on every envelope, if agentic.
//!
//! The schema check is structural, not a full JSON Schema validation: the
//! crate keeps its dependencies minimal.

mod types;

use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::{ErrorCode, Tool};

pub use types::{Fixture, Violation};

/// The domain prefixes a tool name may carry.
pub const DOMAINS: [&str; 8] = [
    "file", "lean", "py", "lit", "web", "ledger", "calc", "agent",
];

/// The one tool name that carries no domain prefix; it lives in `ledger`.
pub const RECALL_MEMORY: &str = "recall_memory";

/// Runs every contract check and returns the violations found; empty means
/// the tool keeps the contract.
pub async fn violations(tool: &dyn Tool, domain: &str, fixtures: &[Fixture]) -> Vec<Violation> {
    let mut found = Vec::new();
    check_name(tool.name(), domain, &mut found);
    check_description(tool, &mut found);
    check_schema(&tool.schema(), &mut found);
    if fixtures.is_empty() {
        found.push(Violation::NoFixtures);
    }

    let mut produced = BTreeSet::new();
    for (index, fixture) in fixtures.iter().enumerate() {
        let envelope = tool.call(&fixture.ctx, fixture.args.clone()).await;
        if !envelope.is_well_formed() {
            found.push(Violation::MalformedEnvelope { fixture: index });
        }
        if !round_trips(&envelope) {
            found.push(Violation::WireForm { fixture: index });
        }
        if tool.kind().is_agentic() && envelope.usage.is_none() {
            found.push(Violation::MissingUsage { fixture: index });
        }
        if let Some(code) = envelope.code() {
            produced.insert(code);
        }
    }

    let declared: BTreeSet<ErrorCode> = tool.errors().iter().copied().collect();
    found.extend(
        declared
            .difference(&produced)
            .map(|c| Violation::CodeNeverProduced(*c)),
    );
    found.extend(
        produced
            .difference(&declared)
            .map(|c| Violation::CodeUndeclared(*c)),
    );
    found
}

/// Checks that `tool`, belonging to `domain`, keeps the tool contract.
///
/// `domain` is the crate's prefix: one of [`DOMAINS`].
///
/// # Errors
///
/// [`Error::Contract`] listing every [`Violation`] found.
pub async fn check(tool: &dyn Tool, domain: &str, fixtures: &[Fixture]) -> Result<()> {
    let found = violations(tool, domain, fixtures).await;
    if found.is_empty() {
        Ok(())
    } else {
        Err(Error::Contract {
            tool: tool.name().to_owned(),
            violations: found,
        })
    }
}

fn check_name(name: &str, domain: &str, found: &mut Vec<Violation>) {
    if !DOMAINS.contains(&domain) {
        found.push(Violation::UnknownDomain(domain.to_owned()));
        return;
    }
    if name == RECALL_MEMORY {
        if domain != "ledger" {
            found.push(Violation::WrongDomain {
                name: name.to_owned(),
                domain: domain.to_owned(),
            });
        }
        return;
    }
    let Some((prefix, rest)) = name.split_once('_') else {
        found.push(Violation::BadName(name.to_owned()));
        return;
    };
    let words_ok = !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    if !DOMAINS.contains(&prefix) || !words_ok {
        found.push(Violation::BadName(name.to_owned()));
    } else if prefix != domain {
        found.push(Violation::WrongDomain {
            name: name.to_owned(),
            domain: domain.to_owned(),
        });
    }
}

/// The description rule alone: the one [`Violation`] `tool`'s description
/// breaks, if any.
///
/// A catalogue test runs it over every composed tool without calling any.
#[must_use]
pub fn description_violation(tool: &dyn Tool) -> Option<Violation> {
    let description = tool.description().trim();
    if description.is_empty() {
        Some(Violation::EmptyDescription)
    } else if tool.kind().is_agentic() {
        (!description.starts_with("Runs a")).then_some(Violation::AgenticDescription)
    } else {
        (!description.starts_with("Given ")).then_some(Violation::PrimitiveDescription)
    }
}

fn check_description(tool: &dyn Tool, found: &mut Vec<Violation>) {
    found.extend(description_violation(tool));
}

fn check_schema(schema: &Value, found: &mut Vec<Violation>) {
    let Some(object) = schema.as_object() else {
        found.push(Violation::SchemaNotObject);
        return;
    };
    if object.get("type").and_then(Value::as_str) != Some("object") {
        found.push(Violation::SchemaNotObject);
    }
    if object.get("additionalProperties") != Some(&Value::Bool(false)) {
        found.push(Violation::SchemaOpen);
    }
    let properties = object.get("properties").and_then(Value::as_object);
    if properties.is_none() && object.contains_key("properties") {
        found.push(Violation::SchemaNotObject);
    }
    if let Some(required) = object.get("required") {
        let names: Vec<&str> = required
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if required.as_array().map(Vec::len) != Some(names.len()) {
            found.push(Violation::SchemaRequiredInvalid);
        }
        for name in names {
            if !properties.is_some_and(|p| p.contains_key(name)) {
                found.push(Violation::SchemaRequiredUnknown(name.to_owned()));
            }
        }
    }
}

fn round_trips(envelope: &crate::Envelope) -> bool {
    serde_json::to_value(envelope).is_ok_and(|value| wire_form_holds(&value, envelope))
}

/// Whether `value`, the serialised `envelope`, carries every pinned key and
/// deserialises back to the same envelope.
fn wire_form_holds(value: &Value, envelope: &crate::Envelope) -> bool {
    let pinned = ["outcome", "data", "unknowns", "next", "cursor"]
        .iter()
        .all(|key| value.get(key).is_some());
    pinned
        && serde_json::from_value::<crate::Envelope>(value.clone())
            .is_ok_and(|back| back == *envelope)
}

#[cfg(test)]
mod test;
