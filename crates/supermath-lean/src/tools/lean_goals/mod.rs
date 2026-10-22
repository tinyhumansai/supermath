//! `lean_goals`: the goal state at a position in a project file.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use super::Shared;
use super::wire::{admit_bytes, expect_ok, guard, parse_args, parse_lean_path, trim_chars};
use crate::client::GoalsRequest;
use crate::error::{Error, Result};

/// Most characters of goal state returned.
const MAX_GOAL_CHARS: usize = 8_000;

/// Returns the goals at a line and column of a project file.
#[derive(Debug)]
pub struct LeanGoals {
    shared: Arc<Shared>,
}

impl LeanGoals {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    path: String,
    line: u32,
    column: u32,
}

/// Whether `line` (one-based) and `column` (zero-based, in characters) lie
/// inside `text`.
pub(super) fn position_in(text: &str, line: u32, column: u32) -> bool {
    line >= 1
        && text
            .lines()
            .nth(line as usize - 1)
            .is_some_and(|l| column as usize <= l.chars().count())
}

impl LeanGoals {
    async fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope> {
        let args: Args = parse_args(args)?;
        let path = parse_lean_path(&args.path)?;
        let slug = ctx.project.slug();
        let file = self.shared.workspace.read(slug, &path)?;
        admit_bytes(ctx, file.text.len())?;
        if !position_in(&file.text, args.line, args.column) {
            return Err(Error::InvalidInput(format!(
                "line {} column {} is outside {path}",
                args.line, args.column
            )));
        }
        let reply = self
            .shared
            .client
            .goals(&GoalsRequest {
                project: slug.to_owned(),
                path: path.to_string(),
                line: args.line,
                column: args.column,
                timeout_ms: self.shared.config.check_timeout_ms,
            })
            .await?;
        expect_ok(reply.status, "the goal query")?;
        let truncated = reply.goals.chars().count() > MAX_GOAL_CHARS;
        Ok(Envelope::ok(json!({
            "path": path.as_str(),
            "sha256": file.sha256,
            "line": args.line,
            "column": args.column,
            "goals": trim_chars(&reply.goals, MAX_GOAL_CHARS),
            "truncated": truncated,
        })))
    }
}

#[async_trait]
impl Tool for LeanGoals {
    fn name(&self) -> &'static str {
        "lean_goals"
    }

    fn description(&self) -> &'static str {
        "Given a Lean file in the project and a position (one-based line, zero-based column), \
         returns the goals there as Lean pretty-prints them: hypotheses and targets, at most \
         8000 characters. A position inside a sorry gives that sorry's goal. The answer carries \
         the file's sha256."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "A Lean module, e.g. lean/Supermath/Sendov/Main.lean"},
                "line": {"type": "integer", "minimum": 1, "description": "One-based line"},
                "column": {"type": "integer", "minimum": 0, "description": "Zero-based column"}
            },
            "required": ["path", "line", "column"],
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }

    fn permission(&self) -> Permission {
        Permission::Read
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[
            ErrorCode::InvalidInput,
            ErrorCode::NotFound,
            ErrorCode::Timeout,
            ErrorCode::BackendUnavailable,
            ErrorCode::BudgetExceeded,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        guard(ctx, self.run(ctx, args)).await
    }
}
