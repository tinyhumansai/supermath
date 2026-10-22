//! The child description and the runner port.

use std::sync::Arc;

use supermath_tool::Project;

use super::{TurnControl, prompts};
use crate::belt::Belt;
use crate::limits::Limits;

/// Which child: one of the four `agent_*` children, or one side of the
/// `agent_settle` duel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildKind {
    /// Reads a paper and records facts (`agent_digest_paper`).
    Reader,
    /// Writes Lean statements and proofs (`agent_formalize`).
    Formalizer,
    /// Judges whether a Lean statement says what the informal one says
    /// (`agent_review_alignment`).
    Reviewer,
    /// Looks for a counterexample (`agent_refute`).
    Refuter,
    /// Proves the pinned conjecture, racing the duel refuter
    /// (`agent_settle`).
    DuelProver,
    /// Refutes the pinned conjecture in Lean, racing the duel prover
    /// (`agent_settle`).
    DuelRefuter,
}

impl ChildKind {
    /// The four children that each back one `agent_*` tool, in catalogue
    /// order.
    pub const ALL: [Self; 4] = [
        Self::Reader,
        Self::Formalizer,
        Self::Reviewer,
        Self::Refuter,
    ];

    /// The two sides of `agent_settle`, prover first.
    pub const DUEL: [Self; 2] = [Self::DuelProver, Self::DuelRefuter];

    /// The static description of this child.
    #[must_use]
    pub const fn spec(self) -> ChildSpec {
        match self {
            Self::Reader => ChildSpec {
                kind: self,
                tool: "agent_digest_paper",
                suffix: "reader",
                primitives: &[
                    "lit_get_paper",
                    "lit_extract_statements",
                    "lit_citations",
                    "ledger_record",
                    "ledger_link",
                    "recall_memory",
                ],
                verifies: &["ledger_claims"],
                prompt: prompts::READER,
                defaults: Limits::DEFAULT,
            },
            Self::Formalizer => ChildSpec {
                kind: self,
                tool: "agent_formalize",
                suffix: "formalizer",
                primitives: &[
                    "lit_get_paper",
                    "lean_search",
                    "lean_decl",
                    "file_list",
                    "file_read",
                    "file_write",
                    "file_edit",
                    "lean_check",
                    "lean_goals",
                    "lean_try_tactics",
                    "recall_memory",
                ],
                verifies: &["ledger_record", "ledger_claims"],
                prompt: prompts::FORMALIZER,
                defaults: Limits::DEFAULT,
            },
            Self::Reviewer => ChildSpec {
                kind: self,
                tool: "agent_review_alignment",
                suffix: "reviewer",
                primitives: &["lean_decl", "lean_search", "lit_get_paper", "recall_memory"],
                verifies: &["lean_check", "ledger_claims", "ledger_record"],
                prompt: prompts::REVIEWER,
                defaults: Limits::REVIEW,
            },
            Self::Refuter => ChildSpec {
                kind: self,
                tool: "agent_refute",
                suffix: "refuter",
                primitives: &[
                    "file_write",
                    "file_edit",
                    "file_read",
                    "py_solvers",
                    "py_run",
                    "py_search_counterexample",
                    "lean_check",
                    "lean_search_counterexample",
                    "lit_oeis",
                ],
                verifies: &["ledger_claims", "ledger_record"],
                prompt: prompts::REFUTER,
                defaults: Limits::DEFAULT,
            },
            Self::DuelProver => ChildSpec {
                kind: self,
                tool: "agent_settle",
                suffix: "duel-prover",
                primitives: &[
                    "lean_search",
                    "lean_decl",
                    "lean_goals",
                    "lean_try_tactics",
                    "lean_check",
                    "file_list",
                    "file_read",
                    "file_write",
                    "file_edit",
                    "recall_memory",
                    "duel_post",
                    "duel_board",
                ],
                verifies: &["ledger_claims", "ledger_record"],
                prompt: prompts::DUEL_PROVER,
                defaults: Limits::DUEL,
            },
            Self::DuelRefuter => ChildSpec {
                kind: self,
                tool: "agent_settle",
                suffix: "duel-refuter",
                primitives: &[
                    "py_solvers",
                    "py_run",
                    "py_search_counterexample",
                    "lean_search_counterexample",
                    "lean_decl",
                    "lean_check",
                    "lit_oeis",
                    "file_list",
                    "file_read",
                    "file_write",
                    "file_edit",
                    "duel_post",
                    "duel_board",
                ],
                verifies: &["ledger_claims", "ledger_record"],
                prompt: prompts::DUEL_REFUTER,
                defaults: Limits::DUEL,
            },
        }
    }
}

/// What a child is: its tool, its primitives, its prompt, and its budget.
#[derive(Debug, Clone, Copy)]
pub struct ChildSpec {
    /// Which child.
    pub kind: ChildKind,
    /// The `agent_*` tool that drives it.
    pub tool: &'static str,
    /// The suffix of its agent id: `supermath-<slug>-<suffix>`.
    pub suffix: &'static str,
    /// The primitives the child may call, and no others: never an `agent_*`
    /// tool, so children do not spawn children. A duel side's list also
    /// names `duel_post` and `duel_board`, which the duel itself provides.
    pub primitives: &'static [&'static str],
    /// Primitives only the tool calls, to verify what the child produced.
    pub verifies: &'static [&'static str],
    /// The child's system prompt.
    pub prompt: &'static str,
    /// The budget a call gets when it names none.
    pub defaults: Limits,
}

impl ChildSpec {
    /// The agent id of this child for `project`.
    #[must_use]
    pub fn agent_id(&self, project: &Project) -> String {
        format!("supermath-{}-{}", project.slug(), self.suffix)
    }

    /// Every registry tool the `agent_*` tool needs: the child's primitives
    /// and the ones it verifies with, less the duel's own `duel_*` tools,
    /// which no registry holds.
    pub fn required(&self) -> impl Iterator<Item = &'static str> {
        self.primitives
            .iter()
            .chain(self.verifies)
            .copied()
            .filter(|name| !name.starts_with("duel_"))
    }
}

/// Why a child turn ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildEnd {
    /// The child gave its final reply.
    Finished,
    /// A budget cut the turn short; the text says which.
    OutOfBudget(String),
}

/// One turn to run.
#[derive(Debug)]
pub struct ChildRequest {
    /// The user message that opens the turn.
    pub prompt: String,
    /// What the turn may spend.
    pub limits: Limits,
    /// The primitives the child may call.
    pub belt: Arc<Belt>,
    /// The host's handle on the running turn, when the host steers or
    /// cancels it; `None` for a turn that runs to its end untouched.
    pub control: Option<TurnControl>,
}

/// What a child turn reported. Its reply is prose to be treated as
/// untrusted; its claims are never verdicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildTurn {
    /// The child's final text.
    pub reply: String,
    /// The `OpenHuman` session the turn ran in, for inspection.
    pub session_id: String,
    /// Model calls the turn made.
    pub turns: u32,
    /// Input tokens the turn spent.
    pub input_tokens: u64,
    /// Output tokens the turn spent.
    pub output_tokens: u64,
    /// Why the turn ended.
    pub end: ChildEnd,
    /// Steers the host queued that the turn never applied, in order, so the
    /// host can carry them into its next turn.
    pub unapplied: Vec<String>,
}

/// Runs one turn of one child.
#[async_trait::async_trait]
pub trait ChildRunner: Send + Sync + std::fmt::Debug {
    /// The model the child runs on, reported in `usage`.
    fn model(&self) -> &str;

    /// Runs the turn.
    ///
    /// # Errors
    ///
    /// A message when the model backend failed before the turn could finish;
    /// the tool answers it as `BACKEND_UNAVAILABLE`.
    async fn run(&self, request: ChildRequest) -> Result<ChildTurn, String>;
}
