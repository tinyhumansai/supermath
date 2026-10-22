//! `agent_refute`: a child searches for a counterexample; only a witness the
//! tool confirmed in Lean counts.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    Backend, CallContext, Envelope, ErrorCode, Exposure, Kind, Outcome, Permission, Tool,
};

use super::shared::{Agentic, Turn, out_of_budget, parse_args, words};
use crate::belt::{Belt, CallNote};
use crate::limits::{BudgetArg, budget_schema};

/// Most words of the repaired statement the child proposes.
const REPAIR_WORDS: usize = 80;
/// Most Python leads and search frames one call records.
const MAX_RECORDS: usize = 5;

/// Runs a refuter child and returns a counterexample the tool confirmed with
/// `lean_check`, or every frame searched without one.
pub struct AgentRefute {
    pub(crate) core: Agentic,
}

impl std::fmt::Debug for AgentRefute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRefute").finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    claim_id: String,
    budget: Option<BudgetArg>,
}

/// The frame a search call covered without finding a witness, as text.
///
/// Both searches answer `ok` with `data.result` set to `exhausted` when they
/// covered the whole stated frame: `py_search_counterexample` names it in
/// `data.frame_description`; `lean_search_counterexample` answers
/// `data.found: false` and `data.frame` as `{predicate, from, to, covered_to,
/// domain}`. A search that stopped early (a limit, a timeout, a crash in the
/// predicate) ends `unknown` and covered no stated frame.
pub(crate) fn frame_of(note: &CallNote) -> Option<String> {
    if note.envelope.outcome != Outcome::Ok {
        return None;
    }
    let data = &note.envelope.data;
    if data.get("result").and_then(Value::as_str) != Some("exhausted") {
        return None;
    }
    match note.tool.as_str() {
        "py_search_counterexample" => data
            .get("frame_description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        "lean_search_counterexample" => {
            let frame = data
                .get("frame")
                .filter(|_| data["found"] == json!(false))?;
            let covered_to = frame.get("covered_to").and_then(Value::as_i64)?;
            let from = frame.get("from").and_then(Value::as_i64)?;
            let predicate = frame.get("predicate").and_then(Value::as_str)?;
            let domain = if frame.get("domain").and_then(Value::as_str) == Some("int") {
                "ℤ"
            } else {
                "ℕ"
            };
            Some(format!(
                "decide {predicate} over {domain} in {from}..={covered_to}"
            ))
        }
        _ => None,
    }
}

/// The witness a search call found, as text, if it found one.
pub(crate) fn witness_of(note: &CallNote) -> Option<String> {
    note.envelope
        .data
        .get("witness")
        .filter(|w| !w.is_null())
        .map(|w| match w {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
}

/// The proposed repair after the child's `REPAIRED:` marker, if it gave one.
fn repair_of(reply: &str) -> Option<String> {
    let (_, text) = reply.rsplit_once("REPAIRED:")?;
    let text = words(text, REPAIR_WORDS);
    (!text.is_empty() && !text.eq_ignore_ascii_case("none")).then_some(text)
}

impl AgentRefute {
    /// Re-runs `lean_check` on the witness theorem the Lean search generated
    /// and records the first that checks as a refutation.
    ///
    /// A found witness answers `{found: true, witness, witness_file, theorem,
    /// verdict, provenance, ...}`; the tool trusts none of it and asks
    /// `lean_check` itself, for the witness theorem only.
    async fn confirm(
        &self,
        ctx: &CallContext,
        belt: &Belt,
        claim: (&str, &str),
        unknowns: &mut Vec<String>,
    ) -> Option<Value> {
        let core = &self.core;
        let (claim_id, statement) = claim;
        for note in belt.succeeded("lean_search_counterexample") {
            let data = &note.envelope.data;
            let (Some(witness), Some(path), Some(theorem)) = (
                witness_of(&note),
                data.get("witness_file").and_then(Value::as_str),
                data.get("theorem").and_then(Value::as_str),
            ) else {
                continue;
            };
            let verdict = core
                .lean_verdict_for(ctx, json!({"path": path, "targets": [theorem]}))
                .await;
            if verdict.verdict != "checked" {
                unknowns.push(format!(
                    "the Lean witness file {path} is {} on re-check, so it is not a refutation",
                    verdict.verdict
                ));
                continue;
            }
            let Some(sha) = verdict.file_sha256 else {
                unknowns.push(format!("lean_check named no digest for {path}"));
                continue;
            };
            let recorded = core
                .record(
                    ctx,
                    json!({
                        "kind": "refutation",
                        "statement": format!(
                            "Counterexample to {claim_id}: witness {witness} ({theorem} in {path}). {statement}"
                        ),
                        "source": {"lean": {"path": path, "declaration": theorem, "file_sha256": sha}},
                        "refutes": claim_id,
                    }),
                )
                .await;
            match recorded {
                Ok(entry) => {
                    return Some(json!({
                        "witness": witness,
                        "path": path,
                        "declaration": theorem,
                        "file_sha256": sha,
                        "verdict": "checked",
                        "refutation": entry.to_json(),
                    }));
                }
                Err(why) => unknowns.push(format!("the refutation was not recorded: {why}")),
            }
        }
        None
    }

    /// Records Python witnesses Lean did not confirm as leads.
    async fn record_leads(
        &self,
        ctx: &CallContext,
        belt: &Belt,
        claim_id: &str,
        unknowns: &mut Vec<String>,
    ) -> Vec<Value> {
        let mut leads = Vec::new();
        for note in belt
            .succeeded("py_search_counterexample")
            .iter()
            .take(MAX_RECORDS)
        {
            let Some(witness) = witness_of(note) else {
                continue;
            };
            let recorded = self
                .core
                .record(
                    ctx,
                    json!({
                        "kind": "lead",
                        "statement": format!("Python witness {witness} for {claim_id}; not confirmed in Lean"),
                        "source": {"supplied": format!("py_search_counterexample call {}", note.call_id)},
                        "rationale": "A Python witness is only a lead until lean_check confirms it as a decidable predicate.",
                    }),
                )
                .await;
            match recorded {
                Ok(entry) => leads.push(json!({"witness": witness, "lead": entry.to_json()})),
                Err(why) => unknowns.push(format!("a lead was not recorded: {why}")),
            }
        }
        leads
    }

    /// Records every frame searched to the end without a witness as a bound
    /// sourced from the search call itself, so "none found" always says how
    /// far the search went. The ledger earns `checked` on that frame only.
    async fn record_frames(
        &self,
        ctx: &CallContext,
        belt: &Belt,
        claim_id: &str,
        unknowns: &mut Vec<String>,
    ) -> Vec<Value> {
        let searched = ["py_search_counterexample", "lean_search_counterexample"]
            .into_iter()
            .flat_map(|tool| belt.notes().into_iter().filter(move |n| n.tool == tool))
            .filter(|n| n.envelope.outcome == Outcome::Ok);
        let mut frames = Vec::new();
        for note in searched.take(MAX_RECORDS) {
            let Some(frame) = frame_of(&note) else {
                continue;
            };
            let recorded = self
                .core
                .record(
                    ctx,
                    json!({
                        "kind": "bound",
                        "statement": format!("No counterexample to {claim_id} found in: {frame}"),
                        // A search that covered its whole frame ends `ok`, which
                        // is what the ledger needs of a `tool` source.
                        "source": {"tool": {"tool": note.tool, "call_id": note.call_id}},
                        "frame": frame,
                    }),
                )
                .await;
            match recorded {
                Ok(entry) => frames.push(json!({
                    "tool": note.tool, "frame": frame, "bound": entry.to_json(),
                })),
                Err(why) => unknowns.push(format!("the frame {frame} was not recorded: {why}")),
            }
        }
        frames
    }

    /// Records the child's repaired statement as the lead it is.
    async fn record_repair(
        &self,
        ctx: &CallContext,
        turn: &Turn,
        claim_id: &str,
        unknowns: &mut Vec<String>,
    ) -> Option<Value> {
        let text = repair_of(&turn.reply)?;
        let recorded = self
            .core
            .record(
                ctx,
                json!({
                    "kind": "lead",
                    "statement": text,
                    "source": {"supplied": format!("repair proposed by the refuter, session {}", turn.session_id)},
                    "rationale": format!("Nearest repaired statement after the counterexample to {claim_id}."),
                }),
            )
            .await;
        match recorded {
            Ok(entry) => Some(json!({"statement": text, "lead": entry.to_json()})),
            Err(why) => {
                unknowns.push(format!("the repair was not recorded: {why}"));
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
        let claim = match core.claim(ctx, &args.claim_id).await {
            Ok(claim) => claim,
            Err(envelope) => return *envelope,
        };
        let statement = claim
            .get("statement")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let prompt = format!(
            "Look for a counterexample to ledger claim {}, which states:\n{statement}",
            args.claim_id
        );
        let turn = match core.run(ctx, prompt, limits).await {
            Ok(turn) => turn,
            Err(envelope) => return *envelope,
        };

        // A witness is a lead until Lean confirms it: the tool re-runs
        // lean_check on the witness file the Lean search generated.
        let mut unknowns = Vec::new();
        let counterexample = self
            .confirm(ctx, &turn.belt, (&args.claim_id, &statement), &mut unknowns)
            .await;
        let found = counterexample.is_some();
        let (leads, frames, repaired) = if found {
            let repaired = self
                .record_repair(ctx, &turn, &args.claim_id, &mut unknowns)
                .await;
            (Vec::new(), Vec::new(), repaired)
        } else {
            let leads = self
                .record_leads(ctx, &turn.belt, &args.claim_id, &mut unknowns)
                .await;
            let frames = self
                .record_frames(ctx, &turn.belt, &args.claim_id, &mut unknowns)
                .await;
            (leads, frames, None)
        };

        let data = json!({
            "claim_id": args.claim_id,
            "counterexample": counterexample,
            "leads": leads,
            "frames": frames,
            "session_id": turn.session_id,
            "untrusted": {"repaired_statement": repaired},
        });
        let mut envelope = if found {
            Envelope::ok(Value::Null)
        } else if let Some(reason) = out_of_budget(&turn.end) {
            Envelope::unknown(reason)
        } else {
            Envelope::unknown("no counterexample was confirmed in Lean")
        }
        .with_data(data)
        .with_usage(turn.usage);
        for unknown in unknowns {
            envelope = envelope.with_unknown(unknown);
        }
        envelope
    }
}

#[async_trait]
impl Tool for AgentRefute {
    fn name(&self) -> &'static str {
        "agent_refute"
    }

    fn description(&self) -> &'static str {
        "Runs a refuter agent turn that searches for a counterexample to a ledger claim before \
         anyone tries to prove it. A Python witness is only a lead; the tool returns a \
         counterexample only after lean_check confirms its witness file, and otherwise \
         returns every frame searched."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "claim_id": {"type": "string", "description": "The ledger claim to refute."},
                "budget": budget_schema()
            },
            "required": ["claim_id"],
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
