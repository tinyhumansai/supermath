//! Reading the primitives back: Lean verdicts, file outlines, and ledger
//! entries, each fetched by the tool itself after the child's turn.

use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Outcome};

use super::shared::{Agentic, Refusal};

/// A Lean verdict the tool obtained by running `lean_check` itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeanVerdict {
    /// `checked`, `checked_with_sorry`, `failed`, or `unknown`.
    pub(crate) verdict: String,
    /// The digest `lean_check` says it checked (`data.provenance.file_sha256`),
    /// when it said.
    pub(crate) file_sha256: Option<String>,
    /// The fully qualified declarations the verdict speaks for, in file order
    /// (`data.targets`).
    pub(crate) targets: Vec<String>,
    /// How many errors Lean reported (`data.error_count`, which counts past
    /// the 20 `data.errors` lists).
    pub(crate) errors: usize,
    /// How many `sorry`s the file holds.
    pub(crate) sorries: usize,
    /// Why the verdict is `unknown`, when it is.
    pub(crate) reason: Option<String>,
}

impl LeanVerdict {
    /// Whether the file elaborates (with or without `sorry`).
    pub(crate) fn compiles(&self) -> bool {
        matches!(self.verdict.as_str(), "checked" | "checked_with_sorry")
    }
}

/// One declaration in a Lean file's outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decl {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) has_sorry: Option<bool>,
}

impl Decl {
    /// Whether this declaration states a result (a theorem or lemma).
    pub(crate) fn is_statement(&self) -> bool {
        matches!(self.kind.as_str(), "theorem" | "lemma")
    }

    pub(crate) fn to_json(&self) -> Value {
        json!({"name": self.name, "kind": self.kind, "has_sorry": self.has_sorry})
    }
}

/// A file as `file_read` describes it without a range.
///
/// Outline names are as written in the file, so inside a `namespace` they are
/// not the names Lean and the ledger use; see [`Outline::qualified`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Outline {
    pub(crate) sha256: String,
    pub(crate) decls: Vec<Decl>,
}

impl Outline {
    /// Each declaration of the outline with the fully qualified name Lean
    /// knows it by, taken from the `targets` of a `lean_check` run without
    /// explicit targets (every named declaration, in file order).
    ///
    /// `None` when the verdict names no target for the declaration, which is
    /// the case for `example`s, unnamed instances, and a verdict that did not
    /// elaborate the file.
    pub(crate) fn qualified<'a>(
        &'a self,
        verdict: &LeanVerdict,
    ) -> Vec<(&'a Decl, Option<String>)> {
        let mut next = 0;
        self.decls
            .iter()
            .map(|decl| {
                let written = decl.name.strip_prefix("_root_.").unwrap_or(&decl.name);
                let suffix = format!(".{written}");
                let found = (decl.name != "<anonymous>" && decl.kind != "example")
                    .then(|| {
                        verdict.targets[next.min(verdict.targets.len())..]
                            .iter()
                            .position(|target| target == written || target.ends_with(&suffix))
                    })
                    .flatten();
                let name = found.map(|offset| {
                    next += offset + 1;
                    verdict.targets[next - 1].clone()
                });
                (decl, name)
            })
            .collect()
    }
}

/// What `ledger_record` answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Recorded {
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) alignment: String,
}

impl Recorded {
    pub(crate) fn to_json(&self) -> Value {
        json!({"id": self.id, "status": self.status, "alignment": self.alignment})
    }
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// A one-line account of an error envelope, for `unknowns`.
pub(crate) fn describe(envelope: &Envelope) -> String {
    envelope.error.as_ref().map_or_else(
        || "no detail".to_owned(),
        |info| format!("{}: {}", info.code.as_str(), info.message),
    )
}

impl Agentic {
    /// Runs `lean_check` on `path` and reduces its answer.
    pub(crate) async fn lean_verdict(&self, ctx: &CallContext, path: &str) -> LeanVerdict {
        self.lean_verdict_for(ctx, json!({"path": path})).await
    }

    /// Runs `lean_check` with `args` (`{path}` or `{path, targets}`) and
    /// reduces its answer.
    pub(crate) async fn lean_verdict_for(&self, ctx: &CallContext, args: Value) -> LeanVerdict {
        let envelope = self.call(ctx, "lean_check", args).await;
        if envelope.outcome == Outcome::Error {
            return LeanVerdict {
                verdict: "unknown".into(),
                file_sha256: None,
                targets: Vec::new(),
                errors: 0,
                sorries: 0,
                reason: Some(describe(&envelope)),
            };
        }
        let data = &envelope.data;
        let verdict = text(data, "verdict")
            .filter(|v| matches!(v.as_str(), "checked" | "checked_with_sorry" | "failed"))
            .unwrap_or_else(|| "unknown".into());
        let reason = (verdict == "unknown").then(|| {
            envelope
                .unknowns
                .first()
                .cloned()
                .unwrap_or_else(|| "lean_check gave no verdict".into())
        });
        LeanVerdict {
            file_sha256: data
                .get("provenance")
                .and_then(|provenance| text(provenance, "file_sha256")),
            targets: data
                .get("targets")
                .and_then(Value::as_array)
                .map(|targets| {
                    targets
                        .iter()
                        .filter_map(|target| target.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            errors: data
                .get("error_count")
                .and_then(Value::as_u64)
                .and_then(|count| usize::try_from(count).ok())
                .unwrap_or(0),
            sorries: data
                .get("sorries")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
            verdict,
            reason,
        }
    }

    /// The outline and digest of `path`, or `None` when the file is absent
    /// or unreadable.
    pub(crate) async fn outline(&self, ctx: &CallContext, path: &str) -> Option<Outline> {
        let envelope = self.call(ctx, "file_read", json!({"path": path})).await;
        if envelope.outcome != Outcome::Ok {
            return None;
        }
        let sha256 = text(&envelope.data, "sha256")?;
        let decls = envelope
            .data
            .get("outline")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        Some(Decl {
                            name: text(item, "name")?,
                            kind: text(item, "kind")?,
                            has_sorry: item.get("has_sorry").and_then(Value::as_bool),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(Outline { sha256, decls })
    }

    /// The ledger entry `id`, in full, or the envelope that refused it.
    ///
    /// # Errors
    ///
    /// `NOT_FOUND` for an unknown id; `BACKEND_UNAVAILABLE` for any other
    /// ledger failure.
    pub(crate) async fn claim(&self, ctx: &CallContext, id: &str) -> Result<Value, Refusal> {
        let envelope = self.call(ctx, "ledger_claims", json!({"id": id})).await;
        match envelope.outcome {
            Outcome::Ok => Ok(envelope.data.get("claim").cloned().unwrap_or(envelope.data)),
            _ => Err(Box::new(refusal(&envelope, "ledger_claims"))),
        }
    }

    /// Records an entry in the ledger.
    ///
    /// # Errors
    ///
    /// A message naming the ledger's refusal.
    pub(crate) async fn record(&self, ctx: &CallContext, args: Value) -> Result<Recorded, String> {
        let envelope = self.call(ctx, "ledger_record", args).await;
        if envelope.outcome == Outcome::Error {
            return Err(describe(&envelope));
        }
        let data = &envelope.data;
        let id = text(data, "id").ok_or_else(|| "ledger_record returned no id".to_owned())?;
        Ok(Recorded {
            id,
            status: text(data, "status").unwrap_or_else(|| "asserted".into()),
            alignment: text(data, "alignment").unwrap_or_else(|| "not_reviewed".into()),
        })
    }
}

/// Maps a primitive's refusal onto this tool's error: `NOT_FOUND` stays,
/// anything else is `BACKEND_UNAVAILABLE` naming the primitive.
pub(crate) fn refusal(envelope: &Envelope, tool: &str) -> Envelope {
    match envelope.code() {
        Some(ErrorCode::NotFound) => Envelope::error(
            ErrorCode::NotFound,
            envelope
                .error
                .as_ref()
                .map_or_else(String::new, |info| info.message.clone()),
        ),
        _ => Envelope::error(
            ErrorCode::BackendUnavailable,
            format!("{tool} failed: {}", describe(envelope)),
        ),
    }
}
