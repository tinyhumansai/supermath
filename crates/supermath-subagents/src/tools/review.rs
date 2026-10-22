//! `agent_review_alignment`: a child, shown only the declaration, judges
//! whether it says what the informal statement says.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    Backend, CallContext, Envelope, ErrorCode, Exposure, Kind, Outcome, Permission, Tool,
};

use super::shared::{Agentic, Refusal, out_of_budget, parse_args, words};
use super::verify::refusal;
use crate::limits::{BudgetArg, budget_schema};

/// Most words of the child's description of a mismatch.
const MISMATCH_WORDS: usize = 100;

/// Runs a reviewer child that sees `lean_decl` signatures, never the proof.
pub struct AgentReviewAlignment {
    pub(crate) core: Agentic,
}

impl std::fmt::Debug for AgentReviewAlignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentReviewAlignment")
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    claim_id: Option<String>,
    declaration: Option<String>,
    statement: Option<String>,
    budget: Option<BudgetArg>,
}

/// What the reviewer concluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Judgment {
    /// Every hypothesis and the conclusion match.
    Accepted,
    /// The statements differ; the text says how.
    Disputed(String),
    /// The reply was neither.
    Unclear,
}

/// Reads the reviewer's final lines: `VERDICT: accepted|disputed` and an
/// optional `MISMATCH:` description. Anything else is [`Judgment::Unclear`].
pub(crate) fn parse_judgment(reply: &str) -> Judgment {
    let clean = |line: &str| {
        line.trim()
            .trim_matches(|c| matches!(c, '*' | '`' | '.'))
            .trim()
            .to_owned()
    };
    let mut verdict = None;
    let mut mismatch = String::new();
    let mut in_mismatch = false;
    for raw in reply.lines() {
        let line = clean(raw);
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("verdict:") {
            verdict = Some(clean(rest));
            in_mismatch = false;
        } else if lower.starts_with("mismatch:") {
            line["mismatch:".len()..].trim().clone_into(&mut mismatch);
            in_mismatch = true;
        } else if in_mismatch && !line.is_empty() {
            mismatch.push(' ');
            mismatch.push_str(&line);
        }
    }
    match verdict.as_deref() {
        Some("accepted") => Judgment::Accepted,
        Some("disputed") => Judgment::Disputed(words(&mismatch, MISMATCH_WORDS)),
        _ => Judgment::Unclear,
    }
}

/// What the call asks to be reviewed, resolved before the child runs.
struct Subject {
    claim_id: Option<String>,
    declaration: String,
    informal: String,
    lean_path: Option<String>,
}

impl AgentReviewAlignment {
    async fn subject(&self, ctx: &CallContext, args: Args) -> Result<Subject, Refusal> {
        let core = &self.core;
        let mut subject = match (args.claim_id, args.declaration, args.statement) {
            (Some(id), None, None) => self.load_claim(ctx, &id).await?,
            (None, Some(declaration), Some(statement)) if !statement.trim().is_empty() => Subject {
                claim_id: None,
                declaration,
                informal: statement,
                lean_path: None,
            },
            _ => {
                return Err(Box::new(Envelope::error(
                    ErrorCode::InvalidInput,
                    "give either claim_id, or both declaration and statement",
                )));
            }
        };
        let decl = core
            .call(ctx, "lean_decl", json!({"name": subject.declaration}))
            .await;
        if decl.outcome == Outcome::Error {
            return Err(Box::new(refusal(&decl, "lean_decl")));
        }
        if subject.lean_path.is_none() {
            subject.lean_path = decl
                .data
                .get("path")
                .and_then(Value::as_str)
                .map(str::to_owned);
        }
        Ok(subject)
    }

    /// Records the review as a fact about the claim, when there is one.
    async fn record_review(
        &self,
        ctx: &CallContext,
        subject: &Subject,
        alignment: &str,
        judgment: &Judgment,
        reviewer: &str,
        unknowns: &mut Vec<String>,
    ) -> Option<String> {
        let claim = subject.claim_id.as_ref()?;
        let mismatch = match judgment {
            Judgment::Disputed(text) => format!(" Mismatch (reviewer's words): {text}"),
            _ => String::new(),
        };
        let recorded = self
            .core
            .record(
                ctx,
                json!({
                    "kind": "fact",
                    "statement": format!(
                        "Alignment review of {claim} ({}): {alignment}.{mismatch}",
                        subject.declaration
                    ),
                    "source": {"supplied": reviewer},
                    "uses": [claim],
                }),
            )
            .await;
        match recorded {
            Ok(entry) => Some(entry.id),
            Err(why) => {
                unknowns.push(format!("the review was not recorded: {why}"));
                None
            }
        }
    }

    async fn run(&self, ctx: &CallContext, args: Value) -> Envelope {
        let core = &self.core;
        let args: Args = match parse_args(args) {
            Ok(args) => args,
            Err(envelope) => return *envelope,
        };
        if let Err(envelope) = core.check_project(ctx) {
            return *envelope;
        }
        let limits = match Agentic::admit(ctx, core.spec.defaults, args.budget) {
            Ok(limits) => limits,
            Err(envelope) => return *envelope,
        };
        let subject = match self.subject(ctx, args).await {
            Ok(subject) => subject,
            Err(envelope) => return *envelope,
        };
        let prompt = format!(
            "Review the Lean declaration `{}` against this informal statement:\n{}",
            subject.declaration, subject.informal
        );
        let turn = match core.run(ctx, prompt, limits).await {
            Ok(turn) => turn,
            Err(envelope) => return *envelope,
        };
        let judgment = parse_judgment(&turn.reply);

        // A statement that does not elaborate cannot be aligned, whatever the
        // reviewer said: the tool reports the kernel's verdict and records
        // nothing.
        let lean = match &subject.lean_path {
            Some(path) => Some(core.lean_verdict(ctx, path).await),
            None => None,
        };
        let blocked = lean.as_ref().is_some_and(|v| !v.compiles());
        let alignment = match (&judgment, blocked) {
            (Judgment::Accepted, false) => "accepted",
            (Judgment::Disputed(_), false) => "disputed",
            _ => "not_reviewed",
        };
        let mut unknowns = Vec::new();
        if let Some(lean) = lean.as_ref().filter(|_| blocked) {
            unknowns.push(format!(
                "the declaration's file is {} in Lean, so no alignment was recorded",
                lean.verdict
            ));
        }
        if matches!(judgment, Judgment::Unclear) {
            unknowns.push("the reviewer's reply was neither accepted nor disputed".to_owned());
        }
        let review_id = if alignment == "not_reviewed" {
            None
        } else {
            let reviewer = format!(
                "reviewer model {}, session {}",
                turn.usage.model, turn.session_id
            );
            self.record_review(
                ctx,
                &subject,
                alignment,
                &judgment,
                &reviewer,
                &mut unknowns,
            )
            .await
        };

        let mismatch = match &judgment {
            Judgment::Disputed(text) if !text.is_empty() => Some(text.clone()),
            _ => None,
        };
        let data = json!({
            "claim_id": subject.claim_id,
            "declaration": subject.declaration,
            "alignment": alignment,
            "lean_verdict": lean.as_ref().map(|v| v.verdict.clone()),
            "review_id": review_id,
            "reviewer": {"model": turn.usage.model, "session_id": turn.session_id},
            "untrusted": {"mismatch": mismatch},
        });
        let mut envelope = if let Some(reason) = out_of_budget(&turn.end) {
            Envelope::unknown(reason)
        } else if alignment == "not_reviewed" {
            Envelope::unknown("alignment stays not_reviewed")
        } else {
            Envelope::ok(Value::Null)
        }
        .with_data(data)
        .with_usage(turn.usage);
        for unknown in unknowns {
            envelope = envelope.with_unknown(unknown);
        }
        envelope
    }

    /// The declaration, informal statement, and Lean file of a ledger result.
    async fn load_claim(&self, ctx: &CallContext, id: &str) -> Result<Subject, Refusal> {
        let core = &self.core;
        let claim = core.claim(ctx, id).await?;
        let invalid = |message: String| Box::new(Envelope::error(ErrorCode::InvalidInput, message));
        let lean = claim
            .get("lean")
            .filter(|lean| !lean.is_null())
            .or_else(|| claim.pointer("/source/lean"))
            .ok_or_else(|| invalid(format!("claim {id} has no lean source")))?;
        let declaration = lean
            .get("declaration")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(format!("claim {id} names no declaration")))?
            .to_owned();
        let lean_path = lean.get("path").and_then(Value::as_str).map(str::to_owned);
        // The informal statement is the fact the result formalizes.
        let mut informal = None;
        for used in claim
            .get("uses")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if let Ok(fact) = core.claim(ctx, used).await
                && let Some(text) = fact.get("statement").and_then(Value::as_str)
                && !text.trim().is_empty()
            {
                informal = Some(text.to_owned());
                break;
            }
        }
        let informal = informal
            .or_else(|| {
                claim
                    .get("statement")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_default();
        Ok(Subject {
            claim_id: Some(id.to_owned()),
            declaration,
            informal,
            lean_path,
        })
    }
}

#[async_trait]
impl Tool for AgentReviewAlignment {
    fn name(&self) -> &'static str {
        "agent_review_alignment"
    }

    fn description(&self) -> &'static str {
        "Runs a reviewer agent turn that judges whether a Lean declaration says what its \
         informal statement says, seeing only the signature and never the proof, and returns \
         accepted or disputed with the specific mismatch. A declaration whose file does not \
         check in Lean is never marked aligned."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "claim_id": {"type": "string",
                    "description": "A ledger result with a lean source."},
                "declaration": {"type": "string", "description": "With statement, instead of claim_id."},
                "statement": {"type": "string", "description": "The informal statement."},
                "budget": budget_schema()
            },
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
