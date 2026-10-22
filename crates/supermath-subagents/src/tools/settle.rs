//! `agent_settle`: a prover and a refuter race on one aligned conjecture;
//! only a theorem the tool checks against the pinned statement settles it.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    Backend, CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool, Usage,
};

use super::shared::{Agentic, parse_args};
use crate::config::DuelSettings;
use crate::duel::{Duel, Settled, Side, Target, Verdict, Win};
use crate::limits::{BudgetArg, Limits, budget_schema};

/// Runs the duel for a ledger claim and reports `proved`, `refuted`,
/// `unknown`, or `inconsistent`, with what both sides established.
pub struct AgentSettle {
    pub(crate) prover: Arc<Agentic>,
    pub(crate) refuter: Arc<Agentic>,
    pub(crate) settings: DuelSettings,
}

impl std::fmt::Debug for AgentSettle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentSettle")
            .field("settings", &self.settings)
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    claim_id: String,
    budget: Option<BudgetArg>,
}

fn win_json(win: &Win, entry: Value) -> Value {
    json!({
        "side": win.side.name(),
        "path": win.path,
        "declaration": win.declaration,
        "file_sha256": win.file_sha256,
        "debt": win.debt,
        "entry": entry,
    })
}

impl AgentSettle {
    /// One side's limits, admitted for both sides at once: twice the
    /// iterations and tokens, the wall time once, since they run together.
    fn admit(ctx: &CallContext, arg: Option<BudgetArg>) -> Result<Limits, Box<Envelope>> {
        let limits = Limits::DUEL
            .apply(arg.unwrap_or_default())
            .map_err(|message| Box::new(Envelope::error(ErrorCode::InvalidInput, message)))?;
        let mut demand = limits.demand();
        demand.iterations = demand.iterations.saturating_mul(2);
        demand.tokens = demand.tokens.saturating_mul(2);
        ctx.budget
            .admit(&demand)
            .map_err(|refusal| Box::new(Envelope::from(refusal)))?;
        Ok(limits)
    }

    fn usage(&self, settled: &Settled) -> Usage {
        let mut usage = Usage {
            model: self.prover.runner.model().to_owned(),
            ..Usage::default()
        };
        for report in &settled.reports {
            usage.turns += report.usage.turns;
            usage.tool_calls += report.usage.tool_calls;
            usage.input_tokens += report.usage.input_tokens;
            usage.output_tokens += report.usage.output_tokens;
            usage.elapsed_ms = usage.elapsed_ms.max(report.usage.elapsed_ms);
        }
        usage
    }

    async fn run(&self, ctx: &CallContext, args: Value) -> Envelope {
        let args: Args = match parse_args(args) {
            Ok(args) => args,
            Err(envelope) => return *envelope,
        };
        if let Err(envelope) = self.prover.check_project(ctx) {
            return *envelope;
        }
        let limits = match Self::admit(ctx, args.budget) {
            Ok(limits) => limits,
            Err(envelope) => return *envelope,
        };
        let claim = match self.prover.claim(ctx, &args.claim_id).await {
            Ok(claim) => claim,
            Err(envelope) => return *envelope,
        };
        let target = match Target::from_claim(&args.claim_id, &claim) {
            Ok(target) => target,
            Err(message) => return Envelope::error(ErrorCode::InvalidInput, message),
        };
        let statement_path = target.statement_path();
        let duel = Duel {
            cores: [&self.prover, &self.refuter],
            target,
            settings: self.settings,
            limits,
            ctx,
        };
        if let Err(envelope) = duel.pin().await {
            return *envelope;
        }
        let settled = duel.run().await;
        let mut unknowns = settled.unknowns.clone();
        let entry = duel.record(&settled, &mut unknowns).await;
        let usage = self.usage(&settled);

        let failures: Vec<String> = Side::BOTH
            .iter()
            .filter_map(|side| {
                settled.reports[side.index()]
                    .failure
                    .as_ref()
                    .map(|why| format!("the {}'s model backend failed: {why}", side.name()))
            })
            .collect();
        if failures.len() == 2 && settled.verdict == Verdict::Unknown {
            return Envelope::error(ErrorCode::BackendUnavailable, failures.join("; "))
                .with_usage(usage);
        }
        unknowns.extend(failures);

        let (bounds, lemmas, leads) = settled.board.entries();
        let (outcome, settled_json, inconsistent) = match &settled.verdict {
            Verdict::Won(win) => (
                match win.side {
                    Side::Prover => "proved",
                    Side::Refuter => "refuted",
                },
                win_json(win, entry),
                Value::Null,
            ),
            Verdict::Inconsistent(a, b) => (
                "inconsistent",
                Value::Null,
                json!([win_json(a, Value::Null), win_json(b, Value::Null)]),
            ),
            Verdict::Unknown => ("unknown", Value::Null, Value::Null),
        };
        let data = json!({
            "claim_id": args.claim_id,
            "outcome": outcome,
            "settled": settled_json,
            "inconsistent": inconsistent,
            "statement": {"path": statement_path},
            "bounds": bounds,
            "lemmas": lemmas,
            "leads": leads,
            "sides": {
                "prover": settled.reports[0].to_json(),
                "refuter": settled.reports[1].to_json(),
            },
        });
        let mut envelope = match &settled.verdict {
            Verdict::Won(_) => Envelope::ok(Value::Null),
            Verdict::Inconsistent(..) => Envelope::unknown(
                "both sides checked against the pinned statement: the statement or the Lean environment is inconsistent, so neither result was recorded",
            ),
            Verdict::Unknown => Envelope::unknown("neither side settled the claim within its budget"),
        }
        .with_data(data)
        .with_usage(usage);
        for unknown in unknowns {
            envelope = envelope.with_unknown(unknown);
        }
        envelope
    }
}

#[async_trait]
impl Tool for AgentSettle {
    fn name(&self) -> &'static str {
        "agent_settle"
    }

    fn description(&self) -> &'static str {
        "Runs a prover agent and a refuter agent in parallel, with equal budgets, on a ledger \
         claim whose Lean statement is alignment-accepted. They share the project workspace and \
         exchange checked lemmas, searched bounds, and leads; the first theorem the tool checks \
         against the pinned statement settles the claim as proved or refuted and stops the \
         other side. Otherwise the outcome is unknown, with every bound searched."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "claim_id": {"type": "string", "description": "The ledger claim to settle; its Lean statement must be alignment-accepted."},
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
        self.prover.finish(envelope)
    }
}
