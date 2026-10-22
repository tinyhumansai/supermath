//! Declared theorem environments and extracted statements.

use serde::Serialize;

/// One numbered statement of a paper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Statement {
    /// The first `\label` inside the statement, if it has one.
    pub label: Option<String>,
    /// The environment's title, lowercased: `theorem`, `lemma`, `remark`.
    pub kind: String,
    /// The environment name used in `\begin{...}`.
    pub env: String,
    /// The number LaTeX prints (`2.3`), or `None` for a starred environment.
    pub number: Option<String>,
    /// The number of the section it sits in, if any.
    pub section: Option<String>,
    /// The statement's LaTeX, without its `\label`.
    pub statement: String,
    /// The labels the statement and its proof `\ref`, in order, without
    /// repeats and without its own label.
    pub refs: Vec<String>,
}

/// What [`extract_statements`](crate::extract_statements) found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Statements {
    /// The statements in document order.
    pub statements: Vec<Statement>,
    /// Environments the parser saw but could not number.
    pub unknowns: Vec<String>,
}

/// A `\newtheorem` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Declaration {
    pub(super) env: String,
    pub(super) title: String,
    /// `\newtheorem*`: no number.
    pub(super) starred: bool,
    /// `[counter]` in `\newtheorem{lemma}[theorem]{Lemma}`.
    pub(super) shared: Option<String>,
    /// `[section]` in `\newtheorem{theorem}{Theorem}[section]`.
    pub(super) within: Option<String>,
}
