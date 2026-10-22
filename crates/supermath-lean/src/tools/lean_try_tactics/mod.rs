//! `lean_try_tactics`: several tactics against one `sorry`, without touching
//! the file.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use super::Shared;
use super::wire::{admit_bytes, expect_ok, guard, parse_args, parse_lean_path, trim_chars};
use crate::client::{TacticResult, TacticsRequest};
use crate::error::{Error, Result};
use crate::verdict::trim_message;

/// Most tactics one call tries.
const MAX_TACTICS: usize = 12;
/// Longest tactic accepted, in characters.
const MAX_TACTIC_CHARS: usize = 500;
/// Tried when the caller names none.
const DEFAULT_TACTICS: [&str; 10] = [
    "rfl",
    "norm_num",
    "simp",
    "omega",
    "decide",
    "linarith",
    "nlinarith",
    "positivity",
    "aesop",
    "exact?",
];

/// Tries tactics against one `sorry` and reports each one's outcome.
#[derive(Debug)]
pub struct LeanTryTactics {
    shared: Arc<Shared>,
}

impl LeanTryTactics {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Position {
    line: u32,
    column: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    path: String,
    sorry: Position,
    #[serde(default)]
    tactics: Option<Vec<String>>,
    #[serde(default)]
    if_sha256: Option<String>,
}

/// Why no ready `file_edit` call could be built for a closing tactic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoEdit {
    /// The `sorry`'s line is past the end of the file.
    PastTheEnd,
    /// The `sorry`'s line holds no `sorry`.
    NoSorryOnLine,
    /// The line occurs more than once, so an edit's `old` would be ambiguous.
    LineNotUnique,
}

impl NoEdit {
    /// The note reported in `unknowns`.
    fn note(self, line: u32) -> String {
        let why = match self {
            Self::PastTheEnd => format!("line {line} is past the end of the file"),
            Self::NoSorryOnLine => format!("line {line} holds no sorry"),
            Self::LineNotUnique => {
                "the line holding the sorry is not unique in the file".to_owned()
            }
        };
        format!("{why}, so no ready file_edit call was built")
    }
}

/// The tactic that replaces the `sorry` for a closing result: `exact` with
/// the term `exact?` found, otherwise the tactic as tried.
fn closing_tactic(result: &TacticResult) -> String {
    match result.term.as_deref().map(str::trim) {
        Some(term) if !term.is_empty() => format!("exact {term}"),
        _ => result.tactic.clone(),
    }
}

/// The `file_edit` call that replaces the `sorry` at `position` with
/// `tactic`, or why none can be built.
fn edit_call(
    path: &str,
    text: &str,
    sha256: &str,
    position: &Position,
    tactic: &str,
) -> std::result::Result<String, NoEdit> {
    let line = text
        .lines()
        .nth(position.line as usize - 1)
        .ok_or(NoEdit::PastTheEnd)?;
    if !line.contains("sorry") {
        return Err(NoEdit::NoSorryOnLine);
    }
    if text.matches(line).count() != 1 {
        return Err(NoEdit::LineNotUnique);
    }
    let chars: Vec<char> = line.chars().collect();
    let column = position.column as usize;
    let at_column = chars
        .get(column..column + 5)
        .map(|s| s.iter().collect::<String>());
    let replaced = if at_column.as_deref() == Some("sorry") {
        let head: String = chars[..column].iter().collect();
        let tail: String = chars[column + 5..].iter().collect();
        format!("{head}{tactic}{tail}")
    } else {
        line.replacen("sorry", tactic, 1)
    };
    Ok(format!(
        "file_edit {}",
        json!({"path": path, "edits": [{"old": line, "new": replaced}], "if_sha256": sha256})
    ))
}

/// Normalises one tactic result: a known outcome, a trimmed error.
fn tidy(mut result: TacticResult) -> TacticResult {
    if !["closes", "progress", "fails"].contains(&result.result.as_str()) {
        result.error = Some(format!("unrecognised outcome {:?}", result.result));
        result.result = "fails".into();
    }
    result.error = result.error.as_deref().map(trim_message);
    result.goals = result.goals.iter().map(|g| trim_chars(g, 2_000)).collect();
    result
}

impl LeanTryTactics {
    async fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope> {
        let args: Args = parse_args(args)?;
        let path = parse_lean_path(&args.path)?;
        let tactics: Vec<String> = match args.tactics {
            Some(given) => given.into_iter().map(|t| t.trim().to_owned()).collect(),
            None => DEFAULT_TACTICS.iter().map(|t| (*t).to_owned()).collect(),
        };
        if tactics.is_empty() || tactics.len() > MAX_TACTICS {
            return Err(Error::InvalidInput(format!(
                "give between 1 and {MAX_TACTICS} tactics"
            )));
        }
        if tactics
            .iter()
            .any(|t| t.is_empty() || t.chars().count() > MAX_TACTIC_CHARS)
        {
            return Err(Error::InvalidInput(format!(
                "each tactic must be 1 to {MAX_TACTIC_CHARS} characters"
            )));
        }
        if args.sorry.line == 0 {
            return Err(Error::InvalidInput("sorry.line is one-based".into()));
        }
        let slug = ctx.project.slug();
        let file = self.shared.workspace.read(slug, &path)?;
        admit_bytes(ctx, file.text.len())?;
        if let Some(expected) = &args.if_sha256
            && !expected.eq_ignore_ascii_case(&file.sha256)
        {
            return Err(Error::Conflict(format!(
                "{path} changed: expected {expected}, found {}",
                file.sha256
            )));
        }
        if let Some(known) = self.shared.checker.known_sorries(slug, &path, &file.sha256)
            && !known
                .iter()
                .any(|s| s.line == args.sorry.line && s.column == args.sorry.column)
        {
            return Err(Error::InvalidInput(format!(
                "there is no sorry at line {} column {}; lean_check listed {}",
                args.sorry.line,
                args.sorry.column,
                known.len()
            )));
        }
        let reply = self
            .shared
            .client
            .tactics(&TacticsRequest {
                project: slug.to_owned(),
                path: path.to_string(),
                line: args.sorry.line,
                column: args.sorry.column,
                tactics: tactics.clone(),
                timeout_ms: self.shared.config.tactic_timeout_ms,
            })
            .await?;
        expect_ok(reply.status, "the tactic run")?;
        let results: Vec<TacticResult> = reply.results.into_iter().map(tidy).collect();
        let mut envelope = Envelope::ok(json!({
            "path": path.as_str(),
            "sha256": file.sha256,
            "sorry": {"line": args.sorry.line, "column": args.sorry.column},
            "results": results,
        }));
        if let Some(winner) = results.iter().find(|r| r.result == "closes") {
            let tactic = closing_tactic(winner);
            match edit_call(
                path.as_str(),
                &file.text,
                &file.sha256,
                &args.sorry,
                &tactic,
            ) {
                Ok(call) => envelope = envelope.with_next([call]),
                Err(why) => envelope = envelope.with_unknown(why.note(args.sorry.line)),
            }
        }
        Ok(envelope)
    }
}

#[async_trait]
impl Tool for LeanTryTactics {
    fn name(&self) -> &'static str {
        "lean_try_tactics"
    }

    fn description(&self) -> &'static str {
        "Given a Lean file, the position of one of its sorries (as lean_check listed it), and up \
         to 12 tactics, runs each against that sorry's goal and reports closes, progress (with \
         the remaining goals), or fails (with the error). With no tactics it tries rfl, norm_num, \
         simp, omega, decide, linarith, nlinarith, positivity, aesop, and exact?. It never changes \
         the file; a tactic that closes the goal comes with a ready file_edit call in next."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "A Lean module, e.g. lean/Supermath/Sendov/Main.lean"},
                "sorry": {
                    "type": "object",
                    "properties": {
                        "line": {"type": "integer", "minimum": 1},
                        "column": {"type": "integer", "minimum": 0}
                    },
                    "required": ["line", "column"],
                    "additionalProperties": false
                },
                "tactics": {"type": "array", "items": {"type": "string"}, "minItems": 1, "maxItems": 12},
                "if_sha256": {"type": "string", "description": "The file digest the position was reported for"}
            },
            "required": ["path", "sorry"],
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
            ErrorCode::Conflict,
            ErrorCode::Timeout,
            ErrorCode::BackendUnavailable,
            ErrorCode::BudgetExceeded,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        guard(ctx, self.run(ctx, args)).await
    }
}

#[cfg(test)]
mod test;
