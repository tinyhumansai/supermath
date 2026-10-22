//! `lean_decl`: a declaration's signature, docstring, and module, without
//! its proof.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_files::{Language, ProjectPath};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use super::Shared;
use super::wire::{expect_ok, guard, parse_args, trim_chars, valid_name};
use crate::client::DeclRequest;
use crate::declarations::declarations;
use crate::error::{Error, Result};

/// Longest signature returned, in characters.
const MAX_SIGNATURE_CHARS: usize = 2_000;

/// Looks a declaration up in Mathlib and in the project's package.
#[derive(Debug)]
pub struct LeanDecl {
    shared: Arc<Shared>,
}

impl LeanDecl {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    name: String,
}

impl LeanDecl {
    /// The project file and line that declare `name`, if any.
    fn in_project(&self, slug: &str, name: &str) -> Result<Option<(ProjectPath, usize)>> {
        for path in self.shared.workspace.list(slug)? {
            if path.language() != Language::Lean {
                continue;
            }
            // A file that vanished or cannot be read since it was listed has
            // nothing to say about the name.
            let Ok(file) = self.shared.workspace.read(slug, &path) else {
                continue;
            };
            if let Some(found) = declarations(&file.text)
                .into_iter()
                .find(|d| d.name == name)
            {
                return Ok(Some((path, found.line)));
            }
        }
        Ok(None)
    }

    async fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope> {
        let args: Args = parse_args(args)?;
        let name = args.name.trim();
        if !valid_name(name) {
            return Err(Error::InvalidInput(format!(
                "{name:?} is not a declaration name"
            )));
        }
        let slug = ctx.project.slug();
        let local = self.in_project(slug, name)?;
        let reply = self
            .shared
            .client
            .decl(&DeclRequest {
                project: slug.to_owned(),
                name: name.to_owned(),
                path: local.as_ref().map(|(path, _)| path.to_string()),
                timeout_ms: self.shared.config.query_timeout_ms,
            })
            .await?;
        expect_ok(reply.status, "the declaration lookup").map_err(|e| match e {
            Error::NotFound(_) => {
                Error::NotFound(format!("{name} is in neither Mathlib nor the project"))
            }
            other => other,
        })?;
        let mut data = json!({
            "name": if reply.name.is_empty() { name } else { reply.name.as_str() },
            "kind": reply.kind,
            "signature": trim_chars(&reply.signature, MAX_SIGNATURE_CHARS),
            "doc": reply.doc,
            "module": reply.module,
        });
        if let Some((path, line)) = local {
            data["path"] = json!(path.as_str());
            data["line"] = json!(line);
        }
        Ok(Envelope::ok(data))
    }
}

#[async_trait]
impl Tool for LeanDecl {
    fn name(&self) -> &'static str {
        "lean_decl"
    }

    fn description(&self) -> &'static str {
        "Given a declaration name, returns its full signature, docstring, module, and kind \
         (theorem, definition, structure, instance, axiom), never its proof. Looks in the pinned \
         Mathlib and in the project's own Lean files; for the latter the answer adds the file and \
         line. NOT_FOUND if neither has it."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "A fully qualified name, e.g. Nat.Prime.two_le"}
            },
            "required": ["name"],
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
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        guard(ctx, self.run(ctx, args)).await
    }
}
