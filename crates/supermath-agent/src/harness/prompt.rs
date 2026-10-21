//! The text the harness sends and stores: the solver prompt, the turn
//! message, and the note written back to memory.

use crate::problem::Problem;
use crate::verdict::Verdict;

/// The longest note written back to memory, in characters.
const NOTE_LIMIT: usize = 6_000;

/// The solver agent's system prompt.
pub(crate) const SOLVER_PROMPT: &str = "\
You are supermath, a careful research mathematician working on one problem.

Method:
1. Restate the problem precisely. Name every quantifier and hypothesis.
2. Try small cases and special cases first; record what they show.
3. Look for a proof. If one step fails, say exactly which step and why.
4. Look for a counterexample as seriously as for a proof.
5. Use `recall_memory` to check what earlier runs on this problem recorded.
   Treat recalled notes as leads to verify, not as established results.

Honesty rules:
- Never claim a proof you have not written out in full.
- A refutation needs an explicit counterexample or a complete disproof.
- If you cannot settle the problem, say what you established and what is left.
- Running out of ideas is not a refutation.

End your reply with exactly one line, and nothing after it:
VERDICT: proved
VERDICT: refuted
VERDICT: not-established";

/// The message that opens a run: the statement, and what memory recalled.
pub(crate) fn turn_message(problem: &Problem, recalled: Option<&str>) -> String {
    let memory = recalled.map_or_else(
        || "Memory holds nothing for this problem yet.".to_string(),
        |notes| format!("Memory recalled these notes from earlier runs:\n\n{notes}"),
    );
    format!(
        "## Problem `{}`\n\n{}\n\n## Memory\n\n{memory}",
        problem.slug(),
        problem.statement()
    )
}

/// The note a finished run writes back to memory.
pub(crate) fn memory_note(verdict: &Verdict, reply: &str) -> String {
    let reply = reply.trim();
    let body = match reply.char_indices().nth(NOTE_LIMIT) {
        Some((cut, _)) => format!("{}…", &reply[..cut]),
        None => reply.to_string(),
    };
    format!("Run verdict: {verdict}\n\n{body}")
}
