//! The three-valued outcome of a run.
//!
//! A run either proved the statement, refuted it, or did not establish
//! either. "Not established" is its own outcome with a reason, and is never
//! collapsed into "refuted": a model that ran out of ideas has not shown the
//! statement false. The research behind this rule is in
//! `docs/research/rea-and-jacobian/implications-for-supermath.md`.
//!
//! The agent states its outcome on the last non-blank line of its reply:
//!
//! ```text
//! VERDICT: proved
//! VERDICT: refuted
//! VERDICT: not-established
//! ```
//!
//! Anything else is read as not established, never as a positive answer.

/// The prefix of the line that carries the agent's verdict.
pub const VERDICT_PREFIX: &str = "VERDICT:";

/// What a run established about its problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The agent claims a proof of the statement.
    Proved,
    /// The agent claims the statement is false, with a counterexample or a
    /// disproof in its reply.
    Refuted,
    /// Neither was established.
    NotEstablished(Reason),
}

/// Why a run did not establish its statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// The agent said it could not settle the statement.
    Stated,
    /// The reply carried no verdict line.
    NoVerdict,
    /// The verdict line named an outcome this harness does not know.
    UnknownVerdict(String),
}

impl Verdict {
    /// Reads the verdict from the last non-blank line of `reply`.
    ///
    /// Matching is case-insensitive and ignores surrounding whitespace and
    /// Markdown emphasis, so `**VERDICT: Proved**` reads as proved.
    #[must_use]
    pub fn parse(reply: &str) -> Self {
        let Some(last) = reply.lines().rev().map(str::trim).find(|l| !l.is_empty()) else {
            return Self::NotEstablished(Reason::NoVerdict);
        };
        let line = last.trim_matches(|c: char| c == '*' || c == '_' || c == '`');
        let Some(prefix) = line.get(..VERDICT_PREFIX.len()) else {
            return Self::NotEstablished(Reason::NoVerdict);
        };
        if !prefix.eq_ignore_ascii_case(VERDICT_PREFIX) {
            return Self::NotEstablished(Reason::NoVerdict);
        }
        let word = line[VERDICT_PREFIX.len()..]
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        match word.as_str() {
            "proved" => Self::Proved,
            "refuted" => Self::Refuted,
            "not-established" | "not established" => Self::NotEstablished(Reason::Stated),
            _ => Self::NotEstablished(Reason::UnknownVerdict(word)),
        }
    }

    /// The verdict as one lowercase word, as written to memory.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Proved => "proved",
            Self::Refuted => "refuted",
            Self::NotEstablished(_) => "not-established",
        }
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotEstablished(Reason::NoVerdict) => {
                f.write_str("not established (the reply carried no verdict line)")
            }
            Self::NotEstablished(Reason::UnknownVerdict(word)) => {
                write!(f, "not established (unknown verdict `{word}`)")
            }
            Self::NotEstablished(Reason::Stated) => f.write_str("not established"),
            other => f.write_str(other.label()),
        }
    }
}

#[cfg(test)]
mod test;
