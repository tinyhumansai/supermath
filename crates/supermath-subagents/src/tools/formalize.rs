//! `agent_formalize`: a child writes Lean files; the tool checks and records
//! them itself.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    Backend, CallContext, Envelope, ErrorCode, Exposure, Kind, Outcome, Permission, Tool, Usage,
};

use super::digest::valid_arxiv_id;
use super::shared::{Agentic, Refusal, Turn, out_of_budget, parse_args, words};
use super::verify::refusal;
use super::verify::{LeanVerdict, Outline};
use crate::belt::Belt;
use crate::limits::{BudgetArg, Limits, budget_schema};

/// Most words of the child's note on what it could not do.
const NOTE_WORDS: usize = 80;
/// Most files one call checks and reports.
const MAX_FILES: usize = 10;
/// Iterations a call that asks for a proof gets by default.
const PROOF_ITERATIONS: u32 = 48;

/// Runs a formalizer child and returns the Lean files it wrote with verdicts
/// the tool computed by running `lean_check` itself.
pub struct AgentFormalize {
    pub(crate) core: Agentic,
}

impl std::fmt::Debug for AgentFormalize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentFormalize").finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    source: SourceArg,
    #[serde(default)]
    depth: Depth,
    path: String,
    budget: Option<BudgetArg>,
}

#[derive(Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Depth {
    #[default]
    Statement,
    Proof,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceArg {
    arxiv_id: Option<String>,
    label: Option<String>,
    statement: Option<String>,
    claim_id: Option<String>,
}

enum Source {
    Arxiv { id: String, label: String },
    Statement(String),
    Claim(String),
}

impl SourceArg {
    fn resolve(self) -> Result<Source, String> {
        match (self.arxiv_id, self.label, self.statement, self.claim_id) {
            (Some(id), Some(label), None, None) if valid_arxiv_id(&id) && !label.is_empty() => {
                Ok(Source::Arxiv { id, label })
            }
            (Some(id), Some(_), None, None) => {
                Err(format!("`{id}` is not an arXiv id with a label"))
            }
            (None, None, Some(statement), None) if !statement.trim().is_empty() => {
                Ok(Source::Statement(statement))
            }
            (None, None, None, Some(claim)) if !claim.is_empty() => Ok(Source::Claim(claim)),
            _ => Err(
                "source must be exactly one of {arxiv_id, label}, {statement}, or {claim_id}"
                    .to_owned(),
            ),
        }
    }
}

/// Whether `path` names a Lean file the way the files crate will accept it.
pub(crate) fn valid_lean_path(path: &str) -> bool {
    path.starts_with("lean/")
        && std::path::Path::new(path)
            .extension()
            .is_some_and(|extension| extension == "lean")
        && path
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.'))
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'_' | b'.' | b'-'))
}

/// Rows per `lit_extract_statements` page (its maximum).
const STATEMENT_PAGE: u32 = 50;
/// Most pages read looking for a label.
const MAX_STATEMENT_PAGES: usize = 100;

/// What the call names, resolved before the child is paid for.
struct Prepared {
    /// The informal statement, when the source gave one.
    informal: Option<String>,
    /// The paper's version, for the fact recorded from an arXiv source.
    version: String,
    /// The ledger fact the result will `use`.
    fact_id: Option<String>,
    /// How the prompt names the thing to formalize.
    subject: String,
    /// What could not be checked before the turn, for `unknowns`.
    notes: Vec<String>,
}

/// What the tool found on disk after the turn.
struct Inspection {
    files: Vec<Value>,
    unknowns: Vec<String>,
    target: Option<(LeanVerdict, Outline)>,
}

/// The files the child wrote, with the newest digest of each, from the
/// registry's own answers to its `file_write` and `file_edit` calls, read in
/// call order so a rewrite after an edit (or an edit after a rewrite) wins.
fn written_files(belt: &Belt) -> Vec<(String, String)> {
    let mut written: Vec<(String, String)> = Vec::new();
    let writes = belt.notes().into_iter().filter(|note| {
        matches!(note.tool.as_str(), "file_write" | "file_edit")
            && note.envelope.outcome == Outcome::Ok
    });
    for note in writes {
        let (Some(path), Some(sha)) = (
            note.envelope.data.get("path").and_then(Value::as_str),
            note.envelope.data.get("sha256").and_then(Value::as_str),
        ) else {
            continue;
        };
        match written.iter_mut().find(|(seen, _)| seen == path) {
            Some(entry) => sha.clone_into(&mut entry.1),
            None => written.push((path.to_owned(), sha.to_owned())),
        }
    }
    written.truncate(MAX_FILES);
    written
}

impl AgentFormalize {
    /// Whether the paper's statement list holds `label`, reading every page
    /// of `lit_extract_statements` (50 rows each, `cursor` to continue).
    /// `None` when the list could not be read to the end, which settles
    /// nothing.
    async fn lists_label(&self, ctx: &CallContext, id: &str, label: &str) -> Option<bool> {
        let mut cursor: Option<String> = None;
        for _ in 0..MAX_STATEMENT_PAGES {
            let mut args = json!({"arxiv_id": id, "limit": STATEMENT_PAGE});
            if let Some(cursor) = &cursor {
                args["cursor"] = json!(cursor);
            }
            let page = self.core.call(ctx, "lit_extract_statements", args).await;
            if page.outcome != Outcome::Ok {
                return None;
            }
            let rows = page.data.get("statements").and_then(Value::as_array)?;
            if rows
                .iter()
                .any(|row| row.get("label").and_then(Value::as_str) == Some(label))
            {
                return Some(true);
            }
            match page.cursor {
                Some(next) => cursor = Some(next),
                None => return Some(false),
            }
        }
        None
    }

    async fn prepare(&self, ctx: &CallContext, source: &Source) -> Result<Prepared, Refusal> {
        let core = &self.core;
        let mut prepared = Prepared {
            informal: None,
            version: "v1".to_owned(),
            fact_id: None,
            subject: String::new(),
            notes: Vec::new(),
        };
        match source {
            Source::Arxiv { id, label } => {
                let paper = core
                    .call(ctx, "lit_get_paper", json!({"arxiv_id": id}))
                    .await;
                if paper.outcome == Outcome::Error {
                    return Err(Box::new(refusal(&paper, "lit_get_paper")));
                }
                if let Some(version) = paper.data.get("version").and_then(Value::as_str) {
                    version.clone_into(&mut prepared.version);
                }
                // A registry without the statement list is a deliberate
                // composition (no literature backend), not a failed check.
                if core.has("lit_extract_statements") {
                    match self.lists_label(ctx, id, label).await {
                        Some(true) => {}
                        Some(false) => {
                            return Err(Box::new(Envelope::error(
                                ErrorCode::NotFound,
                                format!("arXiv:{id} has no statement labelled {label}"),
                            )));
                        }
                        None => prepared.notes.push(format!(
                            "the label {label} was not verified: the statement list of arXiv:{id} could not be read"
                        )),
                    }
                }
                prepared.subject =
                    format!("statement `{label}` of arXiv:{id}; locate it with lit_get_paper");
            }
            Source::Statement(text) => {
                prepared.informal = Some(text.clone());
                prepared.subject = format!("this informal statement:\n{text}");
            }
            Source::Claim(id) => {
                let claim = core.claim(ctx, id).await?;
                let text = claim
                    .get("statement")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                prepared.subject = format!("ledger claim {id}, which states:\n{text}");
                prepared.informal = Some(text);
                prepared.fact_id = Some(id.clone());
            }
        }
        Ok(prepared)
    }

    /// Reads each written file back and runs `lean_check` on it.
    async fn inspect(
        &self,
        ctx: &CallContext,
        written: &[(String, String)],
        target: &str,
        spent: &Usage,
    ) -> Result<Inspection, Refusal> {
        let core = &self.core;
        let mut found = Inspection {
            files: Vec::new(),
            unknowns: Vec::new(),
            target: None,
        };
        for (path, last_sha) in written {
            let Some(outline) = core.outline(ctx, path).await else {
                found
                    .unknowns
                    .push(format!("{path} could not be read back after the turn"));
                continue;
            };
            if outline.sha256 != *last_sha {
                return Err(Box::new(
                    Envelope::error(
                        ErrorCode::Conflict,
                        format!("{path} changed during the turn under another writer"),
                    )
                    .with_usage(spent.clone()),
                ));
            }
            let verdict = core.lean_verdict(ctx, path).await;
            if let Some(reason) = &verdict.reason {
                found.unknowns.push(format!("{path}: {reason}"));
            }
            let declarations: Vec<Value> = outline
                .qualified(&verdict)
                .into_iter()
                .map(|(decl, name)| {
                    let mut row = decl.to_json();
                    if let Some(name) = name {
                        row["name"] = json!(name);
                    }
                    row
                })
                .collect();
            found.files.push(json!({
                "path": path,
                "sha256": outline.sha256,
                "declarations": declarations,
                "verdict": verdict.verdict,
                "errors": verdict.errors,
                "sorries": verdict.sorries,
            }));
            if path == target {
                found.target = Some((verdict, outline));
            }
        }
        Ok(found)
    }

    /// Records each target declaration the kernel accepted, with the digest it
    /// was checked at, linked to the fact it formalizes.
    async fn record_targets(
        &self,
        ctx: &CallContext,
        source: &Source,
        prepared: &mut Prepared,
        path: &str,
        checked: &(LeanVerdict, Outline),
        unknowns: &mut Vec<String>,
    ) -> Vec<Value> {
        let core = &self.core;
        let (verdict, outline) = checked;
        if !verdict.compiles() || !core.has("ledger_record") {
            return Vec::new();
        }
        if prepared.fact_id.is_none()
            && let Source::Arxiv { id, label } = source
        {
            let fact = core
                .record(
                    ctx,
                    json!({
                        "kind": "fact",
                        "statement": format!("Statement {label} of arXiv:{id}, formalized in {path}"),
                        "source": {"arxiv": {"id": id, "version": prepared.version, "label": label}},
                    }),
                )
                .await;
            match fact {
                Ok(fact) => prepared.fact_id = Some(fact.id),
                Err(why) => unknowns.push(format!("the source fact was not recorded: {why}")),
            }
        }
        // `lean_check` names the digest it checked; the file's own digest
        // stands in only when it said nothing.
        let file_sha = verdict
            .file_sha256
            .clone()
            .unwrap_or_else(|| outline.sha256.clone());
        let uses: Vec<&String> = prepared.fact_id.iter().collect();
        let mut recorded = Vec::new();
        for (decl, qualified) in outline.qualified(verdict) {
            if !decl.is_statement() {
                continue;
            }
            // The ledger's `lean` source names the declaration as Lean does,
            // namespaces included; the outline only has it as written.
            let Some(name) = qualified else {
                unknowns.push(format!(
                    "{} was not recorded: lean_check names no target for it",
                    decl.name
                ));
                continue;
            };
            let statement = prepared
                .informal
                .clone()
                .unwrap_or_else(|| format!("Lean declaration {name} in {path}"));
            let result = core
                .record(
                    ctx,
                    json!({
                        "kind": "result",
                        "statement": statement,
                        "source": {"lean": {
                            "path": path,
                            "declaration": name,
                            "file_sha256": file_sha,
                        }},
                        "uses": uses,
                    }),
                )
                .await;
            match result {
                Ok(entry) => {
                    let mut row = entry.to_json();
                    row["declaration"] = json!(name);
                    recorded.push(row);
                }
                Err(why) => unknowns.push(format!("{name} was not recorded: {why}")),
            }
        }
        recorded
    }

    async fn run(&self, ctx: &CallContext, args: Value) -> Envelope {
        let core = &self.core;
        let args: Args = match parse_args(args) {
            Ok(args) => args,
            Err(envelope) => return *envelope,
        };
        let source = match args.source.resolve() {
            Ok(source) => source,
            Err(message) => return Envelope::error(ErrorCode::InvalidInput, message),
        };
        if !valid_lean_path(&args.path) {
            return Envelope::error(
                ErrorCode::InvalidInput,
                format!("`{}` is not a Lean file under lean/", args.path),
            );
        }
        if let Err(envelope) = core.check_project(ctx) {
            return *envelope;
        }
        let base = match args.depth {
            Depth::Statement => core.spec.defaults,
            Depth::Proof => core.spec.defaults.with_iterations(PROOF_ITERATIONS),
        };
        let limits: Limits = match Agentic::admit(ctx, base, args.budget) {
            Ok(limits) => limits,
            Err(envelope) => return *envelope,
        };
        // Everything the call names must exist before the child is paid for.
        let mut prepared = match self.prepare(ctx, &source).await {
            Ok(prepared) => prepared,
            Err(envelope) => return *envelope,
        };
        let existing = core.outline(ctx, &args.path).await.is_some();
        let depth_text = match args.depth {
            Depth::Statement => "Write the statement with a `sorry` proof and check it.",
            Depth::Proof => "Write the statement, then attempt the proof and check it.",
        };
        let prompt = format!(
            "Formalize {}\nTarget file: {} ({}). {depth_text}",
            prepared.subject,
            args.path,
            if existing { "exists; extend it" } else { "new" },
        );
        let turn = match core.run(ctx, prompt, limits).await {
            Ok(turn) => turn,
            Err(envelope) => return *envelope,
        };

        let written = written_files(&turn.belt);
        let mut found = match self.inspect(ctx, &written, &args.path, &turn.usage).await {
            Ok(found) => found,
            Err(envelope) => return *envelope,
        };
        found.unknowns.append(&mut prepared.notes);
        let recorded = match &found.target {
            Some(checked) => {
                self.record_targets(
                    ctx,
                    &source,
                    &mut prepared,
                    &args.path,
                    checked,
                    &mut found.unknowns,
                )
                .await
            }
            None => Vec::new(),
        };

        report(
            &args.path,
            args.depth,
            turn,
            written.is_empty(),
            found,
            &recorded,
        )
    }
}

/// The answer: the verdicts the tool computed, what it recorded, and the
/// child's note as untrusted prose.
fn report(
    path: &str,
    depth: Depth,
    turn: Turn,
    nothing_written: bool,
    found: Inspection,
    recorded: &[Value],
) -> Envelope {
    let target_verdict = found.target.as_ref().map(|(verdict, _)| verdict.clone());
    let data = json!({
        "path": path,
        "depth": match depth { Depth::Statement => "statement", Depth::Proof => "proof" },
        "verdict": target_verdict.as_ref().map(|v| v.verdict.clone()),
        "files": found.files,
        "recorded": recorded,
        "session_id": turn.session_id,
        "untrusted": {"note": words(note_of(&turn.reply), NOTE_WORDS)},
    });
    let mut envelope = if let Some(reason) = out_of_budget(&turn.end) {
        Envelope::unknown(reason)
    } else if nothing_written {
        Envelope::unknown("the child wrote no file")
    } else if found.target.is_none() {
        Envelope::unknown(format!("the child did not write the target file {path}"))
    } else if target_verdict
        .as_ref()
        .is_some_and(|v| v.verdict == "unknown")
    {
        Envelope::unknown("lean_check could not give a verdict for the target file")
    } else {
        Envelope::ok(Value::Null)
    }
    .with_data(data)
    .with_usage(turn.usage);
    if let Some(verdict) = &target_verdict
        && verdict.verdict == "checked_with_sorry"
        && depth == Depth::Proof
    {
        envelope =
            envelope.with_unknown(format!("{} sorry(s) remain in the proof", verdict.sorries));
    }
    for unknown in found.unknowns {
        envelope = envelope.with_unknown(unknown);
    }
    let next = match (&target_verdict, recorded_first(recorded)) {
        (Some(v), Some(id)) if v.compiles() => {
            vec![format!(
                "agent_review_alignment {}",
                json!({"claim_id": id})
            )]
        }
        (Some(_), _) => vec![format!("lean_check {}", json!({"path": path}))],
        _ => Vec::new(),
    };
    envelope.with_next(next)
}

fn recorded_first(recorded: &[Value]) -> Option<String> {
    recorded
        .first()
        .and_then(|row| row.get("id"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// The text after the child's `NOTE:` marker, or its whole reply without one.
fn note_of(reply: &str) -> &str {
    reply.rsplit_once("NOTE:").map_or(reply, |(_, note)| note)
}

#[async_trait]
impl Tool for AgentFormalize {
    fn name(&self) -> &'static str {
        "agent_formalize"
    }

    fn description(&self) -> &'static str {
        "Runs a formalizer agent turn that writes a statement, and optionally its proof, as \
         Lean files in the project package, then returns each file's path, digest, \
         declarations, and a verdict the tool computed itself by running lean_check. Each \
         checked target declaration is recorded in the ledger."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "object",
                    "description": "Exactly one of {arxiv_id, label}, {statement}, or {claim_id}.",
                    "properties": {
                        "arxiv_id": {"type": "string"},
                        "label": {"type": "string"},
                        "statement": {"type": "string"},
                        "claim_id": {"type": "string"}
                    },
                    "additionalProperties": false
                },
                "depth": {"type": "string", "enum": ["statement", "proof"],
                    "description": "statement writes a sorry proof; proof also attempts it."},
                "path": {"type": "string", "description": "A Lean file under lean/ to create or extend."},
                "budget": budget_schema()
            },
            "required": ["source", "path"],
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
        Kind::Agentic(Backend::OpenHuman)
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[
            ErrorCode::InvalidInput,
            ErrorCode::NotFound,
            ErrorCode::Conflict,
            ErrorCode::BackendUnavailable,
            ErrorCode::BudgetExceeded,
            ErrorCode::Forbidden,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        let envelope = self.run(ctx, args).await;
        self.core.finish(envelope)
    }
}
