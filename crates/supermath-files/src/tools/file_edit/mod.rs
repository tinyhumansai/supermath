//! `file_edit`: change part of a file by exact string replacement.

mod splice;

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use super::wire;
use crate::error::Error;
use crate::workspace::{Workspace, WriteMeta};
use splice::Edit;

/// Edits a Lean or Python file by exact replacement, guarded by `if_sha256`.
#[derive(Debug, Clone)]
pub struct FileEdit {
    workspace: Arc<Workspace>,
}

impl FileEdit {
    /// The tool over `workspace`.
    #[must_use]
    pub fn new(workspace: Arc<Workspace>) -> Self {
        Self { workspace }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    path: String,
    edits: Vec<Edit>,
    if_sha256: String,
}

impl FileEdit {
    fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope, wire::Failure> {
        let args: Args = wire::parse_args(args)?;
        let path = wire::parse_path(&args.path)?;
        if args.edits.is_empty() {
            return Err(wire::bad("edits is empty"));
        }
        let slug = ctx.project.slug();
        let current = self
            .workspace
            .read(slug, &path)
            .map_err(|e| wire::failure(&e))?;
        if !current.sha256.eq_ignore_ascii_case(&args.if_sha256) {
            return Err(wire::failure(&Error::Conflict {
                path: path.to_string(),
                expected: args.if_sha256,
                actual: current.sha256,
            }));
        }
        let applied = splice::apply(&current.text, &args.edits).map_err(|fault| {
            wire::bad(match fault.occurrences {
                0 => format!("edit {} has an empty or absent `old` string", fault.index),
                n => format!(
                    "edit {}: `old` occurs {n} times; it must occur exactly once",
                    fault.index
                ),
            })
        })?;
        wire::admit_bytes(ctx, applied.text.len())?;
        let meta = WriteMeta::new("file_edit", &ctx.call_id);
        let outcome = self
            .workspace
            .write(slug, &path, &applied.text, Some(&args.if_sha256), &meta)
            .map_err(|e| wire::failure(&e))?;
        let changed: Vec<Value> = applied
            .changed_lines()
            .into_iter()
            .map(|(start, end)| json!({"start_line": start, "end_line": end}))
            .collect();
        Ok(Envelope::ok(json!({
            "path": outcome.path.as_str(),
            "sha256": outcome.sha256,
            "changed": changed,
        }))
        .with_next([wire::follow_up(&path)]))
    }
}

#[async_trait]
impl Tool for FileEdit {
    fn name(&self) -> &'static str {
        "file_edit"
    }

    fn description(&self) -> &'static str {
        "Given a file and exact replacements, edits part of a Lean or Python file in the project \
         workspace and returns the new sha256 and the changed line ranges. Each old string must \
         occur exactly once; edits apply in order, all or none. if_sha256 is required: a stale \
         digest is CONFLICT. Does not check or run the file."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Relative to the project root"},
                "edits": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "type": "object",
                        "properties": {
                            "old": {"type": "string", "minLength": 1, "description": "Text to replace; must occur exactly once"},
                            "new": {"type": "string", "description": "Its replacement"}
                        },
                        "required": ["old", "new"],
                        "additionalProperties": false
                    }
                },
                "if_sha256": {"type": "string", "description": "Digest file_read or file_write returned"}
            },
            "required": ["path", "edits", "if_sha256"],
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }

    fn permission(&self) -> Permission {
        Permission::Write
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[
            ErrorCode::InvalidInput,
            ErrorCode::NotFound,
            ErrorCode::Conflict,
            ErrorCode::BudgetExceeded,
            ErrorCode::BackendUnavailable,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        self.run(ctx, args).unwrap_or_else(|failure| *failure)
    }
}

#[cfg(test)]
mod test;
