//! Fixtures and violations for the contract checker.

use serde_json::Value;

use crate::{CallContext, ErrorCode, Project};

/// One call the checker makes: the arguments and the context to make it in.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// The context the tool is called with.
    pub ctx: CallContext,
    /// The arguments the tool is called with.
    pub args: Value,
}

impl Fixture {
    /// A fixture calling with `args` in a scratch project.
    ///
    /// Use [`Fixture::with_ctx`] when a tool needs a specific project or
    /// budget.
    #[must_use]
    pub fn new(args: Value) -> Self {
        // The slug and scope are constants that satisfy `Project::new`.
        let project = Project::new("contract-check", "ws:contract/project:contract-check")
            .unwrap_or_else(|_| unreachable!("constant project is valid"));
        Self {
            ctx: CallContext::new(project, "contract-check"),
            args,
        }
    }

    /// A fixture calling with `args` in `ctx`.
    #[must_use]
    pub fn with_ctx(ctx: CallContext, args: Value) -> Self {
        Self { ctx, args }
    }
}

/// One way a tool can break the tool contract.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Violation {
    /// The checker was given no fixtures.
    #[error("no fixtures")]
    NoFixtures,
    /// The domain argument is not one of the known prefixes.
    #[error("unknown domain {0:?}")]
    UnknownDomain(String),
    /// The name does not match `<domain>_<words>`.
    #[error("name {0:?} does not match the tool naming pattern")]
    BadName(String),
    /// The name's prefix is another domain's.
    #[error("name {name:?} does not belong to domain {domain:?}")]
    WrongDomain {
        /// The tool's name.
        name: String,
        /// The domain it was checked against.
        domain: String,
    },
    /// The description is empty.
    #[error("empty description")]
    EmptyDescription,
    /// An agentic tool's description does not begin "Runs a".
    #[error("agentic description must begin \"Runs a\"")]
    AgenticDescription,
    /// The schema is not an object schema.
    #[error("schema is not an object schema")]
    SchemaNotObject,
    /// The schema does not set `additionalProperties` to `false`.
    #[error("schema must set additionalProperties to false")]
    SchemaOpen,
    /// `required` is not an array of strings.
    #[error("schema `required` is not an array of strings")]
    SchemaRequiredInvalid,
    /// `required` names a property the schema does not declare.
    #[error("schema requires undeclared property {0:?}")]
    SchemaRequiredUnknown(String),
    /// A code in `errors()` that no fixture produced.
    #[error("declared code {0:?} is produced by no fixture")]
    CodeNeverProduced(ErrorCode),
    /// A fixture produced a code missing from `errors()`.
    #[error("code {0:?} is returned but not declared")]
    CodeUndeclared(ErrorCode),
    /// An envelope has an `error` that disagrees with its outcome, or too
    /// many `next` calls.
    #[error("fixture {fixture} returned a malformed envelope")]
    MalformedEnvelope {
        /// Index of the fixture.
        fixture: usize,
    },
    /// An envelope does not serialise to the pinned form and back.
    #[error("fixture {fixture} returned an envelope that does not round-trip")]
    WireForm {
        /// Index of the fixture.
        fixture: usize,
    },
    /// An agentic tool returned an envelope without `usage`.
    #[error("fixture {fixture} returned no usage from an agentic tool")]
    MissingUsage {
        /// Index of the fixture.
        fixture: usize,
    },
}
