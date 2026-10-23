//! Running the top-level agent on a problem: `supermath-agent solve`.
//!
//! [`solve`] sends the problem prompt as the first turn of one session, then
//! sends [`CONTINUE_PROMPT`] as each later turn, up to the turn limit or until
//! a reply carries [`DONE_MARKER`] on a line of its own. Every turn's tool
//! calls are reported as they start and end, and a watcher tails the
//! toolkit's call log, so calls the `agent_*` children make (the duel's
//! prover and refuter among them) are reported too.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use openhuman_embed::AgentProgress;
use supermath_tool::{CallLog, CallRecord};
use tokio::sync::mpsc;

use crate::error::{Error, Result};
use crate::runner::Solver;

/// The prompt every turn after the first sends.
pub const CONTINUE_PROMPT: &str = "\
Continue the work. Check the ledger and the project files for what earlier \
turns established, then take the next step that moves the problem forward. \
This turn starts with only the core tools loaded: a tool you loaded with \
tool_search in an earlier turn is not loaded now, so call tool_search for \
each other tool you need before calling it. \
When the problem is settled, or nothing more can be done, end your reply \
with the line STATUS: DONE.";

/// The line a reply ends the run with.
pub const DONE_MARKER: &str = "STATUS: DONE";

/// How often the call-log watcher looks for new records.
const WATCH_EVERY: Duration = Duration::from_secs(2);

/// The longest argument or result excerpt a progress line carries.
const EXCERPT_CHARS: usize = 240;

/// What a [`solve`] run produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
    /// The session every turn ran in.
    pub session_id: String,
    /// Each turn's final reply, in order.
    pub replies: Vec<String>,
}

impl Transcript {
    /// Whether the last reply declared the run done.
    #[must_use]
    pub fn done(&self) -> bool {
        self.replies
            .last()
            .is_some_and(|reply| declares_done(reply))
    }
}

/// Whether `reply` has [`DONE_MARKER`] on a line of its own.
#[must_use]
pub fn declares_done(reply: &str) -> bool {
    reply.lines().any(|line| line.trim() == DONE_MARKER)
}

/// Runs `solver` on `prompt` for at most `turns` turns in one session,
/// writing a line to `report` for each tool call and each reply.
///
/// # Errors
///
/// Returns [`Error::Turn`] when `OpenHuman` fails a turn; the turns before it
/// are lost with the error, but their effects (files, ledger entries) stay.
pub async fn solve(
    solver: &Solver,
    prompt: &str,
    turns: NonZeroUsize,
    report: Arc<dyn Fn(String) + Send + Sync>,
) -> Result<Transcript> {
    let log = solver.host().toolkit().call_log();
    let reported = Arc::new(AtomicUsize::new(0));
    let watcher = tokio::spawn(watch(
        Arc::clone(&log),
        Arc::clone(&reported),
        Arc::clone(&report),
    ));
    let result = run_turns(solver, prompt, turns, &report).await;
    watcher.abort();
    // One last sweep, so calls logged since the watcher's last look are
    // reported too.
    sweep(log.as_ref(), &reported, report.as_ref());
    result
}

async fn run_turns(
    solver: &Solver,
    prompt: &str,
    turns: NonZeroUsize,
    report: &Arc<dyn Fn(String) + Send + Sync>,
) -> Result<Transcript> {
    let mut transcript = Transcript {
        session_id: String::new(),
        replies: Vec::new(),
    };
    for number in 1..=turns.get() {
        let message = if number == 1 { prompt } else { CONTINUE_PROMPT };
        report(format!("== turn {number}/{turns}"));
        let (sender, mut receiver) = mpsc::channel(256);
        let mut turn = solver.agent().turn(message).on_progress(sender);
        if !transcript.session_id.is_empty() {
            turn = turn.session(transcript.session_id.clone());
        }
        let progress_report = Arc::clone(report);
        let progress = tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                if let Some(line) = progress_line(&event) {
                    progress_report(line);
                }
            }
        });
        let sent = turn.send().await;
        // The turn dropped its sender, so the reader ends on its own.
        let _ = progress.await;
        let outcome = sent.map_err(|error| Error::Turn(error.to_string()))?;
        report(format!("== reply {number}\n{}", outcome.reply));
        transcript.session_id = outcome.session_id;
        transcript.replies.push(outcome.reply);
        if transcript.done() {
            break;
        }
    }
    Ok(transcript)
}

/// Reports every call-log record once, as it appears.
async fn watch(
    log: Arc<dyn CallLog>,
    reported: Arc<AtomicUsize>,
    report: Arc<dyn Fn(String) + Send + Sync>,
) {
    loop {
        sweep(log.as_ref(), &reported, report.as_ref());
        tokio::time::sleep(WATCH_EVERY).await;
    }
}

/// Reports the records past `reported` and moves it to the end of the log.
fn sweep(log: &dyn CallLog, reported: &AtomicUsize, report: &(dyn Fn(String) + Send + Sync)) {
    let records = log.all();
    let from = reported.swap(records.len(), Ordering::Relaxed);
    for record in records.iter().skip(from) {
        report(call_line(record));
    }
}

/// One line for a finished call in the call log.
#[must_use]
pub fn call_line(record: &CallRecord) -> String {
    let code = record
        .code
        .map_or_else(String::new, |code| format!(" {code:?}"));
    format!(
        "   log {} {} -> {:?}{code} ({} ms)",
        record.call_id, record.tool, record.outcome, record.elapsed_ms
    )
}

/// One line for a progress event worth reporting, or `None`.
#[must_use]
pub fn progress_line(event: &AgentProgress) -> Option<String> {
    match event {
        AgentProgress::IterationStarted {
            iteration,
            max_iterations,
        } => Some(format!("-- iteration {iteration}/{max_iterations}")),
        AgentProgress::ToolCallStarted {
            tool_name,
            arguments,
            ..
        } => Some(format!(
            "-> {tool_name} {}",
            excerpt(&arguments.to_string())
        )),
        AgentProgress::ToolCallCompleted {
            tool_name,
            success,
            output,
            elapsed_ms,
            ..
        } => Some(format!(
            "<- {tool_name} {} ({elapsed_ms} ms) {}",
            if *success { "ok" } else { "failed" },
            excerpt(output)
        )),
        AgentProgress::ContextCompacting => Some("-- compacting context".into()),
        _ => None,
    }
}

/// `text` on one line, cut to [`EXCERPT_CHARS`] characters.
fn excerpt(text: &str) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= EXCERPT_CHARS {
        flat
    } else {
        let cut: String = flat.chars().take(EXCERPT_CHARS).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod test;
