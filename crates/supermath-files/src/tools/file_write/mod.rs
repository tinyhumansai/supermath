//! `file_write`: create or replace a whole file in the project workspace.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use super::wire;
use crate::workspace::{Workspace, WriteMeta};

/// Creates or replaces a whole Lean or Python file, guarded by `if_sha256`.
#[derive(Debug, Clone)]
pub struct FileWrite {
    workspace: Arc<Workspace>,
}

impl FileWrite {
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
    content: String,
    #[serde(default)]
    if_sha256: Option<String>,
}

/// The digest `if_sha256` names, or `None` for a new file.
///
/// Models often send the string `"null"` (or an empty string) where the
/// schema means JSON `null`; read as a digest, it was a `CONFLICT` that named
/// no real mismatch. Anything else that is not a sha256 is `INVALID_INPUT`.
fn expected_digest(raw: Option<&str>) -> Result<Option<&str>, wire::Failure> {
    let Some(raw) = raw.map(str::trim) else {
        return Ok(None);
    };
    if raw.is_empty() || raw.eq_ignore_ascii_case("null") {
        return Ok(None);
    }
    if raw.len() == 64 && raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(Some(raw));
    }
    Err(wire::bad(format!(
        "if_sha256 must be the file's 64-hex sha256 or null for a new file, not {raw:?}"
    )))
}

impl FileWrite {
    fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope, wire::Failure> {
        let args: Args = wire::parse_args(args)?;
        let path = wire::parse_path(&args.path)?;
        wire::admit_bytes(ctx, args.content.len())?;
        let meta = WriteMeta::new("file_write", &ctx.call_id);
        let outcome = self
            .workspace
            .write(
                ctx.project.slug(),
                &path,
                &args.content,
                expected_digest(args.if_sha256.as_deref())?,
                &meta,
            )
            .map_err(|e| wire::failure(&e))?;
        Ok(Envelope::ok(json!({
            "path": outcome.path.as_str(),
            "sha256": outcome.sha256,
            "lines": outcome.lines,
        }))
        .with_next([wire::follow_up(&path)]))
    }
}

#[async_trait]
impl Tool for FileWrite {
    fn name(&self) -> &'static str {
        "file_write"
    }

    fn description(&self) -> &'static str {
        "Given a path and its content, creates or replaces a whole Lean or Python file in the \
         project workspace and returns its sha256. Pass the sha256 you last saw as if_sha256 to \
         replace a file, or null for a new one; a mismatch is CONFLICT. Does not check or run \
         the file. Use file_edit to change part of an existing file."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative to the project root: lean/Supermath/<Module>.lean or python/work/<module>.py"
                },
                "content": {"type": "string", "description": "UTF-8 source, at most 1 MB"},
                "if_sha256": {
                    "type": ["string", "null"],
                    "description": "Digest of the file you last saw; null for a new file"
                }
            },
            "required": ["path", "content"],
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
            ErrorCode::Conflict,
            ErrorCode::BudgetExceeded,
            ErrorCode::BackendUnavailable,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        self.run(ctx, args).unwrap_or_else(|failure| *failure)
    }
}
