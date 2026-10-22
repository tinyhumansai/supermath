//! The duel behind `agent_settle`: a prover and a refuter racing on one
//! pinned conjecture.
//!
//! Both sides run at once, as two `OpenHuman` children, in the same project
//! workspace with disjoint write areas ([`guard`]). They talk only through
//! `duel_post`, which the host checks before relaying ([`post`]), and the
//! host reads every call they make ([`board`]) to deliver digests, nudge or
//! restart a drifting turn, and notice a `Main.lean` worth verifying. A win
//! counts only after the host checks the side's theorem against the pinned
//! `Conjecture` in a file it writes itself ([`target`]); the first win
//! cancels the other side.
//!
//! Each side gets the same limits for the whole duel, spread over at most
//! [`DuelSettings::max_rounds`] turns, and never inherits the other side's
//! leftover.

mod board;
mod brief;
mod guard;
mod post;
mod target;
mod types;

use std::sync::Arc;
use std::time::Instant;

use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Cancellation, Envelope, ErrorCode, Tool, ToolRegistry, Usage,
};
use tokio::sync::mpsc;

use crate::child::{ChildEnd, TurnControl};
use crate::config::DuelSettings;
use crate::limits::Limits;
use crate::tools::{Agentic, describe};

pub(crate) use board::Board;
pub(crate) use post::{DuelBoard, DuelPost, Offline};
pub(crate) use target::Target;
pub(crate) use types::{Side, SideReport, Win};

use board::{Bucket, Event};
use brief::Carry;
use guard::WriteGuard;

/// A turn does not start with less than this left: iterations, tokens,
/// seconds.
const MIN_TURN: Limits = Limits::new(2, 10_000, 30);
/// Most refuter frames recorded as bounds when the duel ends.
const MAX_CLOSING_BOUNDS: usize = 10;

/// How the duel ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// One side's theorem checked against the pinned statement.
    Won(Win),
    /// Both did: the statement or the environment is broken.
    Inconsistent(Win, Win),
    /// Neither did.
    Unknown,
}

/// What a finished duel hands back to `agent_settle`.
pub(crate) struct Settled {
    pub(crate) verdict: Verdict,
    pub(crate) board: Arc<Board>,
    pub(crate) reports: [SideReport; 2],
    pub(crate) unknowns: Vec<String>,
}

/// One duel: the two sides' cores, the target, and the call it runs under.
pub(crate) struct Duel<'a> {
    pub(crate) cores: [&'a Arc<Agentic>; 2],
    pub(crate) target: Target,
    pub(crate) settings: DuelSettings,
    pub(crate) limits: Limits,
    pub(crate) ctx: &'a CallContext,
}

/// `ctx` with its call id extended by `suffix`, so each side's turns log
/// under ids of their own.
fn sub_ctx(ctx: &CallContext, suffix: &str) -> CallContext {
    let mut sub = ctx.clone();
    sub.call_id = format!("{}.{suffix}", ctx.call_id);
    sub
}

/// What is left of `limits` after `used` and `elapsed` seconds, or `None`
/// when less than a useful turn remains.
fn remaining(limits: Limits, used: &Usage, elapsed: u64) -> Option<Limits> {
    let left = Limits::new(
        limits.max_iterations.saturating_sub(used.turns),
        limits
            .max_tokens
            .saturating_sub(used.input_tokens + used.output_tokens),
        limits.wall_secs.saturating_sub(elapsed),
    );
    (left.max_iterations >= MIN_TURN.max_iterations
        && left.max_tokens >= MIN_TURN.max_tokens
        && left.wall_secs >= MIN_TURN.wall_secs)
        .then_some(left)
}

fn add_usage(total: &mut Usage, more: &Usage) {
    total.turns += more.turns;
    total.tool_calls += more.tool_calls;
    total.input_tokens += more.input_tokens;
    total.output_tokens += more.output_tokens;
    total.elapsed_ms = total.elapsed_ms.max(more.elapsed_ms);
}

/// Writes a host-owned file, replacing whatever is there.
///
/// # Errors
///
/// A one-line account of the write's refusal.
async fn write_host(
    core: &Agentic,
    ctx: &CallContext,
    path: &str,
    content: &str,
) -> Result<(), String> {
    let current = core.outline(ctx, path).await.map(|outline| outline.sha256);
    let written = core
        .call(
            ctx,
            "file_write",
            json!({"path": path, "content": content, "if_sha256": current}),
        )
        .await;
    if written.is_ok() {
        Ok(())
    } else {
        Err(describe(&written))
    }
}

/// The belt scope of `side`: its primitives, with the write tools confined
/// to its area, and its own `duel_post` and `duel_board`.
fn side_scope(core: &Arc<Agentic>, side: Side, board: &Arc<Board>) -> ToolRegistry {
    let areas = board.target().areas(side);
    let mut scope = ToolRegistry::new();
    let mut add = |tool: Arc<dyn Tool>| {
        // Names come from one registry plus the two duel tools, so none
        // repeats; a duplicate would only drop the second copy.
        let _ = scope.register(tool);
    };
    for tool in core.scope().iter() {
        if matches!(tool.name(), "file_write" | "file_edit") {
            add(Arc::new(WriteGuard::new(Arc::clone(tool), areas.clone())));
        } else {
            add(Arc::clone(tool));
        }
    }
    add(Arc::new(DuelPost {
        side,
        board: Arc::clone(board),
        core: Arc::clone(core),
    }));
    add(Arc::new(DuelBoard {
        side,
        board: Arc::clone(board),
    }));
    scope
}

impl Duel<'_> {
    fn core(&self, side: Side) -> &Arc<Agentic> {
        self.cores[side.index()]
    }

    /// Checks that the claim's Lean file is still the one its verdict holds
    /// for, then writes and checks `Statement.lean`.
    ///
    /// # Errors
    ///
    /// `INVALID_INPUT` for a missing or changed claim file;
    /// `BACKEND_UNAVAILABLE` when the pinned statement cannot be written or
    /// does not check.
    pub(crate) async fn pin(&self) -> Result<(), Box<Envelope>> {
        let core = self.core(Side::Prover);
        let target = &self.target;
        let refuse = |code, message: String| Box::new(Envelope::error(code, message));
        match core.outline(self.ctx, &target.lean_path).await {
            None => {
                return Err(refuse(
                    ErrorCode::InvalidInput,
                    format!(
                        "the claim's Lean file {} cannot be read; run agent_formalize again",
                        target.lean_path
                    ),
                ));
            }
            Some(outline) if outline.sha256 != target.lean_sha256 => {
                return Err(refuse(
                    ErrorCode::InvalidInput,
                    format!(
                        "{} changed since its Lean verdict; run agent_formalize and agent_review_alignment again",
                        target.lean_path
                    ),
                ));
            }
            Some(_) => {}
        }
        let path = target.statement_path();
        write_host(core, self.ctx, &path, &target.statement_text())
            .await
            .map_err(|why| {
                refuse(
                    ErrorCode::BackendUnavailable,
                    format!("the pinned statement could not be written: {why}"),
                )
            })?;
        let verdict = core
            .lean_verdict_for(
                self.ctx,
                json!({"path": path, "targets": [target.conjecture()]}),
            )
            .await;
        if verdict.verdict == "checked" {
            Ok(())
        } else {
            Err(refuse(
                ErrorCode::BackendUnavailable,
                format!(
                    "the pinned statement {path} is {} in Lean{}",
                    verdict.verdict,
                    verdict
                        .reason
                        .map(|why| format!(": {why}"))
                        .unwrap_or_default()
                ),
            ))
        }
    }

    /// Runs both sides and the controller to the end.
    pub(crate) async fn run(&self) -> Settled {
        let (board, events) = Board::new(self.target.clone(), self.settings);
        let scopes = Side::BOTH.map(|side| side_scope(self.core(side), side, &board));
        let done = Cancellation::new();
        let drivers = async {
            let reports = tokio::join!(
                self.drive(Side::Prover, &board, &scopes[0]),
                self.drive(Side::Refuter, &board, &scopes[1]),
            );
            done.cancel();
            reports
        };
        let (reports, decided) = tokio::join!(drivers, self.control(&board, events, &done));
        let mut unknowns = Vec::new();
        let verdict = match decided {
            Some(verdict) => verdict,
            None => self.closing(&mut unknowns).await,
        };
        board.finish();
        Settled {
            verdict,
            board,
            reports: [reports.0, reports.1],
            unknowns,
        }
    }

    /// Runs one side's turns until it wins, loses, or spends its budget.
    async fn drive(&self, side: Side, board: &Arc<Board>, scope: &ToolRegistry) -> SideReport {
        let core = self.core(side);
        let started = Instant::now();
        let mut report = SideReport::default();
        let mut last_end = String::new();
        let mut unapplied: Vec<String> = Vec::new();
        loop {
            if board.is_finished() {
                break;
            }
            if report.rounds >= self.settings.max_rounds {
                board.set_status(side, "stopped: every turn used");
                break;
            }
            let Some(left) = remaining(self.limits, &report.usage, started.elapsed().as_secs())
            else {
                board.set_status(side, "stopped: budget spent");
                break;
            };
            let prompt = if report.rounds == 0 {
                brief::opening(&self.target, side, self.limits, self.settings.max_rounds)
            } else {
                brief::continuation(
                    board,
                    side,
                    &Carry {
                        round: report.rounds + 1,
                        rounds: self.settings.max_rounds,
                        last_end: &last_end,
                        unapplied: &unapplied,
                        remaining: left,
                    },
                )
            };
            let ctx = sub_ctx(self.ctx, &format!("{}{}", side.name(), report.rounds + 1));
            let control = TurnControl::new();
            board.begin_turn(side, &control);
            let belt = Arc::new(
                core.belt(&ctx, scope.clone())
                    .with_watcher(board.watcher(side)),
            );
            let turn = core
                .run_on(&ctx, prompt, left, belt, Some(control.clone()))
                .await;
            let drifted = board.end_turn(side);
            report.rounds += 1;
            match turn {
                Err(envelope) => {
                    if let Some(usage) = &envelope.usage {
                        add_usage(&mut report.usage, usage);
                    }
                    report.ended.push("backend failed".into());
                    report.failure = Some(describe(&envelope));
                    board.set_status(side, "stopped: model backend failed");
                    break;
                }
                Ok(turn) => {
                    add_usage(&mut report.usage, &turn.usage);
                    if !turn.session_id.is_empty() {
                        report.sessions.push(turn.session_id.clone());
                    }
                    last_end = if board.is_finished() {
                        "settled".into()
                    } else if drifted {
                        "drift".into()
                    } else {
                        match &turn.end {
                            ChildEnd::Finished => "finished".into(),
                            ChildEnd::OutOfBudget(why) => why.clone(),
                        }
                    };
                    report.ended.push(last_end.clone());
                    unapplied = turn.unapplied;
                    unapplied.extend(control.take_steers());
                }
            }
        }
        report.usage.model = core.runner.model().to_owned();
        report
    }

    /// Verifies each side whose own check said its `Main.lean` is
    /// `checked`, until one wins or both drivers stop.
    async fn control(
        &self,
        board: &Board,
        mut events: mpsc::UnboundedReceiver<Event>,
        done: &Cancellation,
    ) -> Option<Verdict> {
        loop {
            tokio::select! {
                event = events.recv() => match event {
                    Some(Event::Checked(side)) => {
                        if let Some(win) = self.verify(side).await {
                            board.finish();
                            return Some(match self.verify(side.partner()).await {
                                Some(other) => Verdict::Inconsistent(win, other),
                                None => Verdict::Won(win),
                            });
                        }
                    }
                    None => return None,
                },
                () = done.cancelled() => return None,
            }
        }
    }

    /// Checks both sides once more after the race, in case a win was never
    /// announced by a `lean_check` of its own.
    async fn closing(&self, unknowns: &mut Vec<String>) -> Verdict {
        let prover = self.verify(Side::Prover).await;
        let refuter = self.verify(Side::Refuter).await;
        match (prover, refuter) {
            (Some(a), Some(b)) => Verdict::Inconsistent(a, b),
            (Some(win), None) | (None, Some(win)) => Verdict::Won(win),
            (None, None) => {
                unknowns.push("neither side's theorem checked against the pinned statement".into());
                Verdict::Unknown
            }
        }
    }

    /// The host's two-step check of a side's win: its theorem `checked` in
    /// its own file, then a host-written file proving the pinned statement
    /// (or its negation) from that theorem `checked` too.
    async fn verify(&self, side: Side) -> Option<Win> {
        let core = self.core(side);
        let target = &self.target;
        let ctx = sub_ctx(self.ctx, &format!("verify.{}", side.name()));
        let own = core
            .lean_verdict_for(
                &ctx,
                json!({"path": target.main_path(side), "targets": [target.theorem(side)]}),
            )
            .await;
        if own.verdict != "checked" {
            return None;
        }
        let path = target.verdict_path(side);
        write_host(core, &ctx, &path, &target.verdict_text(side))
            .await
            .ok()?;
        let declaration = target.verdict_theorem(side);
        let settled = core
            .lean_verdict_for(&ctx, json!({"path": path, "targets": [declaration]}))
            .await;
        if settled.verdict != "checked" {
            return None;
        }
        Some(Win {
            side,
            path,
            declaration,
            file_sha256: settled.file_sha256?,
            debt: settled.debt,
        })
    }

    /// Records the verdict and the refuter's unposted frames in the ledger,
    /// and returns the win's entry.
    pub(crate) async fn record(&self, settled: &Settled, unknowns: &mut Vec<String>) -> Value {
        let core = self.core(Side::Prover);
        let target = &self.target;
        let frames = settled.board.take_unposted_frames(Side::Refuter);
        for frame in frames.into_iter().take(MAX_CLOSING_BOUNDS) {
            let recorded = core
                .record(
                    self.ctx,
                    json!({
                        "kind": "bound",
                        "statement": format!("No counterexample to {} found in: {}", target.claim_id, frame.frame),
                        "source": {"tool": {"tool": frame.tool, "call_id": frame.call_id}},
                        "frame": frame.frame,
                    }),
                )
                .await;
            match recorded {
                Ok(entry) => settled.board.keep_entry(Bucket::Bound, entry.to_json()),
                Err(why) => unknowns.push(format!("the frame {} was not recorded: {why}", frame.frame)),
            }
        }
        let Verdict::Won(win) = &settled.verdict else {
            return Value::Null;
        };
        let source = json!({"lean": {
            "path": win.path, "declaration": win.declaration, "file_sha256": win.file_sha256,
        }});
        let args = match win.side {
            Side::Prover => json!({
                "kind": "result",
                "statement": format!(
                    "Ledger claim {} is proved in Lean ({} in {}): {}",
                    target.claim_id, win.declaration, win.path, target.statement
                ),
                "source": source,
            }),
            Side::Refuter => json!({
                "kind": "refutation",
                "statement": format!(
                    "Ledger claim {} is refuted in Lean ({} in {}): {}",
                    target.claim_id, win.declaration, win.path, target.statement
                ),
                "source": source,
                "refutes": target.claim_id,
            }),
        };
        match core.record(self.ctx, args).await {
            Ok(entry) => entry.to_json(),
            Err(why) => {
                unknowns.push(format!("the {} win was not recorded: {why}", win.side.name()));
                Value::Null
            }
        }
    }
}

#[cfg(test)]
mod test;
