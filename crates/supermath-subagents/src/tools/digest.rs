//! `agent_digest_paper`: a child reads a paper and records facts.

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    Backend, CallContext, Envelope, ErrorCode, Exposure, Kind, Outcome, Permission, Tool,
};

use super::shared::{Agentic, out_of_budget, parse_args, words};
use super::verify::{describe, refusal};
use crate::belt::Belt;
use crate::limits::{BudgetArg, budget_schema};

/// Most words of the child's account of the paper passed through.
const SUMMARY_WORDS: usize = 120;

/// Runs a reader child on an arXiv paper and returns the ledger ids it
/// recorded, verified against the ledger.
pub struct AgentDigestPaper {
    pub(crate) core: Agentic,
}

impl std::fmt::Debug for AgentDigestPaper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentDigestPaper").finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    arxiv_id: String,
    focus: Option<String>,
    budget: Option<BudgetArg>,
}

/// Whether `id` is an arXiv identifier, new style (`2012.04125`, optionally
/// with a version) or old style (`math/0211159`).
pub(crate) fn valid_arxiv_id(id: &str) -> bool {
    let body = match id.rsplit_once('v') {
        Some((head, version))
            if !version.is_empty() && version.bytes().all(|b| b.is_ascii_digit()) =>
        {
            head
        }
        _ => id,
    };
    let digits = |s: &str, n: std::ops::RangeInclusive<usize>| {
        n.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
    };
    if let Some((archive, number)) = body.split_once('/') {
        let (name, subject) = archive.split_once('.').unwrap_or((archive, ""));
        return !name.is_empty()
            && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
            && (subject.is_empty()
                || (subject.len() == 2 && subject.bytes().all(|b| b.is_ascii_uppercase())))
            && digits(number, 7..=7);
    }
    match body.split_once('.') {
        Some((year_month, number)) => digits(year_month, 4..=4) && digits(number, 4..=5),
        None => false,
    }
}

/// What reading the child's ledger ids back found.
#[derive(Default)]
struct ReadBack {
    /// The ids the ledger holds, by kind.
    by_kind: BTreeMap<String, Vec<String>>,
    /// How many ids the ledger answered it does not hold.
    missing: usize,
    /// The ids the ledger could not be asked about.
    unverified: Vec<String>,
    /// The first ledger failure, when there was one.
    outage: Option<String>,
    /// Statuses above `cited`, flagged for `unknowns`.
    notes: Vec<String>,
}

impl AgentDigestPaper {
    /// Reads back every id the child recorded. An id the ledger lacks is
    /// missing, an id the ledger could not be asked about is unverified (an
    /// outage is not an absence), and a status above `cited` is flagged, not
    /// trusted.
    async fn read_back(&self, ctx: &CallContext, belt: &Belt) -> ReadBack {
        let mut ids: Vec<String> = Vec::new();
        for note in belt.succeeded("ledger_record") {
            if let Some(id) = note.envelope.data.get("id").and_then(Value::as_str)
                && !ids.iter().any(|seen| seen == id)
            {
                ids.push(id.to_owned());
            }
        }
        let mut found = ReadBack::default();
        for id in ids {
            match self.core.claim(ctx, &id).await {
                Ok(claim) => {
                    let kind = claim
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("fact")
                        .to_owned();
                    if let Some(status) = claim.get("status").and_then(Value::as_str)
                        && matches!(status, "checked" | "formalised")
                    {
                        found.notes.push(format!(
                            "{id} has status {status}; reading earns at most cited"
                        ));
                    }
                    found.by_kind.entry(kind).or_default().push(id);
                }
                Err(refused) if refused.code() == Some(ErrorCode::NotFound) => found.missing += 1,
                Err(refused) => {
                    found.outage.get_or_insert_with(|| describe(&refused));
                    found.unverified.push(id);
                }
            }
        }
        found
    }

    async fn run(&self, ctx: &CallContext, args: Value) -> Envelope {
        let core = &self.core;
        let args: Args = match parse_args(args) {
            Ok(args) => args,
            Err(envelope) => return *envelope,
        };
        if !valid_arxiv_id(&args.arxiv_id) {
            return Envelope::error(
                ErrorCode::InvalidInput,
                format!("`{}` is not an arXiv id", args.arxiv_id),
            );
        }
        if let Err(envelope) = core.check_project(ctx) {
            return *envelope;
        }
        let limits = match Agentic::admit(ctx, core.spec.defaults, args.budget) {
            Ok(limits) => limits,
            Err(envelope) => return *envelope,
        };
        let paper = core
            .call(ctx, "lit_get_paper", json!({"arxiv_id": args.arxiv_id}))
            .await;
        if paper.outcome == Outcome::Error {
            return refusal(&paper, "lit_get_paper");
        }

        let mut prompt = format!("Digest arXiv paper {}.", args.arxiv_id);
        if let Some(focus) = args.focus.as_deref().filter(|f| !f.trim().is_empty()) {
            prompt = format!("{prompt} Focus on: {focus}");
        }
        let turn = match core.run(ctx, prompt, limits).await {
            Ok(turn) => turn,
            Err(envelope) => return *envelope,
        };

        let ReadBack {
            by_kind,
            missing,
            unverified,
            outage,
            notes,
        } = self.read_back(ctx, &turn.belt).await;
        let verified: usize = by_kind.values().map(Vec::len).sum();
        let data = json!({
            "arxiv_id": args.arxiv_id,
            "recorded": by_kind,
            "count": verified,
            "unverified": unverified,
            "session_id": turn.session_id,
            "untrusted": {"summary": words(&turn.reply, SUMMARY_WORDS)},
        });
        let mut envelope = if let Some(reason) = out_of_budget(&turn.end) {
            Envelope::unknown(reason)
        } else if verified == 0 && !unverified.is_empty() {
            Envelope::unknown("the ledger could not confirm what the turn recorded")
        } else if verified == 0 {
            Envelope::unknown("the turn recorded nothing the ledger holds")
        } else {
            Envelope::ok(Value::Null)
        }
        .with_data(data)
        .with_usage(turn.usage);
        if missing > 0 {
            envelope = envelope.with_unknown(format!(
                "{missing} id(s) the child claimed to record are not in the ledger"
            ));
        }
        if let Some(why) = outage {
            envelope = envelope.with_unknown(format!(
                "{} id(s) could not be read back from the ledger: {why}",
                unverified.len()
            ));
        }
        for note in notes {
            envelope = envelope.with_unknown(note);
        }
        if verified > 0 {
            envelope =
                envelope.with_next([format!("ledger_claims {}", json!({"query": args.arxiv_id}))]);
        }
        envelope
    }
}

#[async_trait]
impl Tool for AgentDigestPaper {
    fn name(&self) -> &'static str {
        "agent_digest_paper"
    }

    fn description(&self) -> &'static str {
        "Runs a reader agent turn on an arXiv paper and returns the ledger ids of the facts \
         it recorded, grouped by kind, after reading each back from the ledger. The reading \
         stays in the child's context. Every fact is cited, never higher."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "arxiv_id": {"type": "string", "description": "e.g. 2012.04125"},
                "focus": {"type": "string", "description": "Narrows what the child records."},
                "budget": budget_schema()
            },
            "required": ["arxiv_id"],
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
