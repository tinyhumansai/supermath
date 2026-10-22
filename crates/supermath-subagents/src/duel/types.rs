//! The duel's small types: its two sides, the kinds of post, and what a side
//! reports when it stops.

use serde_json::{Value, json};
use supermath_tool::Usage;

/// One side of the duel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Side {
    /// Proves `Conjecture`.
    Prover,
    /// Proves `¬ Conjecture`.
    Refuter,
}

impl Side {
    /// Both sides, prover first.
    pub(crate) const BOTH: [Self; 2] = [Self::Prover, Self::Refuter];

    /// The side's name, as reports and prompts spell it.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Prover => "prover",
            Self::Refuter => "refuter",
        }
    }

    /// The other side.
    pub(crate) const fn partner(self) -> Self {
        match self {
            Self::Prover => Self::Refuter,
            Self::Refuter => Self::Prover,
        }
    }

    /// The side's position in per-side arrays.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Prover => 0,
            Self::Refuter => 1,
        }
    }
}

/// What a `duel_post` carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PostKind {
    /// A lemma the tool checked in Lean.
    Lemma,
    /// A frame the refuter searched to the end without a witness.
    Bound,
    /// Where the prover's approach broke: a hint for the refuter.
    Obstruction,
    /// A near miss or suspicious structure: a hint for the prover.
    Candidate,
}

impl PostKind {
    /// Reads the `kind` argument.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "lemma" => Self::Lemma,
            "bound" => Self::Bound,
            "obstruction" => Self::Obstruction,
            "candidate" => Self::Candidate,
            _ => return None,
        })
    }

    /// The kind as the argument spells it.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Lemma => "lemma",
            Self::Bound => "bound",
            Self::Obstruction => "obstruction",
            Self::Candidate => "candidate",
        }
    }

    /// Whether `side` may send this kind.
    pub(crate) const fn allowed_for(self, side: Side) -> bool {
        matches!(
            (self, side),
            (Self::Lemma, _)
                | (Self::Bound | Self::Candidate, Side::Refuter)
                | (Self::Obstruction, Side::Prover)
        )
    }
}

/// One relayed post, as the partner sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Item {
    /// Who sent it.
    pub(crate) from: Side,
    /// Whether the tool checked it.
    pub(crate) verified: bool,
    /// The line shown to the partner, already framed `VERIFIED …` or
    /// `UNVERIFIED lead: …`.
    pub(crate) line: String,
}

impl Item {
    pub(crate) fn to_json(&self) -> Value {
        json!({"from": self.from.name(), "verified": self.verified, "line": self.line})
    }
}

/// A side's verified win: the verdict file the tool wrote and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Win {
    pub(crate) side: Side,
    pub(crate) path: String,
    pub(crate) declaration: String,
    pub(crate) file_sha256: String,
    pub(crate) debt: Vec<String>,
}

/// What one side spent and how each of its turns ended.
#[derive(Debug, Clone, Default)]
pub(crate) struct SideReport {
    pub(crate) rounds: u32,
    pub(crate) usage: Usage,
    pub(crate) ended: Vec<String>,
    pub(crate) sessions: Vec<String>,
    /// A backend failure that stopped the side, if one did.
    pub(crate) failure: Option<String>,
}

impl SideReport {
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "rounds": self.rounds,
            "tool_calls": self.usage.tool_calls,
            "turns": self.usage.turns,
            "input_tokens": self.usage.input_tokens,
            "output_tokens": self.usage.output_tokens,
            "ended": self.ended,
            "sessions": self.sessions,
        })
    }
}
