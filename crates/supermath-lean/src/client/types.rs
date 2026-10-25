//! The wire forms of the Lean service's HTTP API.
//!
//! These types are the contract between this crate and `docker/lean/`. The
//! API is specified in `docs/specs/toolkit/lean/README.md#the-http-api`; a
//! unit test pins each request's serde form because a client and a service
//! that disagree about a field name fail at runtime with a decode error.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// How a service operation ended.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Status {
    /// The operation ran to completion.
    #[default]
    Ok,
    /// It hit its wall-time limit and was killed.
    Timeout,
    /// It hit its memory limit and was killed.
    Memory,
    /// Its output exceeded the size cap.
    OutputLimit,
    /// `tactics`: there is no `sorry` at the position.
    NoSorry,
    /// `goals` or `tactics`: the position is outside the file.
    BadPosition,
    /// `counterexample`: the declaration is not a decidable predicate.
    BadPredicate,
    /// The declaration is not in the environment.
    NotFound,
    /// A status this client does not know; treated as a failed operation.
    #[serde(other)]
    Other,
}

/// What `GET /health` reports: the pin every verdict is recorded against.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Health {
    /// The Lean toolchain, such as `leanprover/lean4:v4.20.0`.
    pub toolchain: String,
    /// The Mathlib commit.
    pub mathlib: String,
}

/// One message Lean printed while elaborating.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Message {
    /// `error`, `warning`, or `info`.
    pub severity: String,
    /// One-based line.
    pub line: u32,
    /// Zero-based column.
    pub column: u32,
    /// The message text.
    pub text: String,
}

/// One `sorry` in a file, with the goal it stands for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sorry {
    /// One-based line.
    pub line: u32,
    /// Zero-based column.
    pub column: u32,
    /// The goal, pretty-printed.
    pub goal: String,
}

/// `POST /check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRequest {
    /// The project slug.
    pub project: String,
    /// The project's absolute directory, when it is not `<projects>/<slug>`
    /// on the service's mount (`Workspace::service_root`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// The file, relative to the project root.
    pub path: String,
    /// Declarations to run `#print axioms` on, after elaboration.
    pub targets: Vec<String>,
    /// Also report which project declarations each target uses.
    pub uses: bool,
    /// The wall-time limit, in milliseconds.
    pub timeout_ms: u64,
}

/// The answer to `POST /check`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CheckReply {
    /// How the check ended.
    pub status: Status,
    /// The toolchain that ran it.
    pub toolchain: String,
    /// The Mathlib commit it ran against.
    pub mathlib: String,
    /// Wall time spent, in milliseconds.
    pub elapsed_ms: u64,
    /// Everything Lean printed, in order.
    pub messages: Vec<Message>,
    /// Every `sorry`, with its goal.
    pub sorries: Vec<Sorry>,
    /// Target name to its axioms; `null` when the target does not exist.
    pub axioms: BTreeMap<String, Option<Vec<String>>>,
    /// Target name to the project declarations it uses (when requested).
    pub uses: BTreeMap<String, Vec<String>>,
}

/// `POST /goals`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalsRequest {
    /// The project slug.
    pub project: String,
    /// The project's absolute directory, when it is not `<projects>/<slug>`
    /// on the service's mount (`Workspace::service_root`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// The file, relative to the project root.
    pub path: String,
    /// One-based line.
    pub line: u32,
    /// Zero-based column.
    pub column: u32,
    /// The wall-time limit, in milliseconds.
    pub timeout_ms: u64,
}

/// The answer to `POST /goals`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct GoalsReply {
    /// How the operation ended.
    pub status: Status,
    /// The goals at the position, pretty-printed; empty when there are none.
    pub goals: String,
}

/// `POST /tactics`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticsRequest {
    /// The project slug.
    pub project: String,
    /// The project's absolute directory, when it is not `<projects>/<slug>`
    /// on the service's mount (`Workspace::service_root`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// The file, relative to the project root.
    pub path: String,
    /// One-based line of the `sorry`.
    pub line: u32,
    /// Zero-based column of the `sorry`.
    pub column: u32,
    /// The tactics to try, each against the `sorry`'s goal.
    pub tactics: Vec<String>,
    /// The wall-time limit for each tactic, in milliseconds.
    pub timeout_ms: u64,
}

/// One tactic's outcome.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TacticResult {
    /// The tactic as sent.
    pub tactic: String,
    /// `closes`, `progress`, or `fails`.
    pub result: String,
    /// The goals left, when it made progress.
    pub goals: Vec<String>,
    /// Lean's error, when it failed.
    pub error: Option<String>,
    /// The term an `exact` suggestion carried (as `exact?` gives), when the
    /// tactic closed the goal; `None` for any other suggestion.
    pub term: Option<String>,
}

/// The answer to `POST /tactics`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct TacticsReply {
    /// How the operation ended.
    pub status: Status,
    /// One entry per tactic, in the order sent.
    pub results: Vec<TacticResult>,
}

/// `POST /decl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclRequest {
    /// The project slug.
    pub project: String,
    /// The project's absolute directory, when it is not `<projects>/<slug>`
    /// on the service's mount (`Workspace::service_root`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// The declaration's full name.
    pub name: String,
    /// A project file to elaborate first, when the name is a project's own.
    pub path: Option<String>,
    /// The wall-time limit, in milliseconds.
    pub timeout_ms: u64,
}

/// The answer to `POST /decl`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct DeclReply {
    /// How the operation ended.
    pub status: Status,
    /// The declaration's full name.
    pub name: String,
    /// `theorem`, `definition`, `structure`, `instance`, `axiom`, ….
    pub kind: String,
    /// The signature, without the proof.
    pub signature: String,
    /// The docstring, empty when there is none.
    pub doc: String,
    /// The module that declares it.
    pub module: String,
}

/// `POST /search`: Mathlib declarations by name fragment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRequest {
    /// The name fragment.
    pub query: String,
    /// The most rows to return.
    pub limit: usize,
    /// The wall-time limit, in milliseconds.
    pub timeout_ms: u64,
}

/// One declaration a search found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchRow {
    /// The declaration's full name.
    pub name: String,
    /// Its type, pretty-printed.
    #[serde(rename = "type")]
    pub ty: String,
    /// The module that declares it.
    pub module: String,
}

/// The answer to `POST /search`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct SearchReply {
    /// How the operation ended.
    pub status: Status,
    /// The Mathlib commit searched.
    pub mathlib: String,
    /// The matches, best first.
    pub results: Vec<SearchRow>,
}

/// `POST /counterexample`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterRequest {
    /// The project slug.
    pub project: String,
    /// The project's absolute directory, when it is not `<projects>/<slug>`
    /// on the service's mount (`Workspace::service_root`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// The file declaring the predicate, relative to the project root.
    pub path: String,
    /// The predicate's full name.
    pub predicate: String,
    /// The first value, inclusive.
    pub from: i64,
    /// The last value, inclusive.
    pub to: i64,
    /// The wall-time limit, in milliseconds.
    pub timeout_ms: u64,
}

/// The answer to `POST /counterexample`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct CounterReply {
    /// How the search ended.
    pub status: Status,
    /// `nat` or `int`: the predicate's domain.
    pub domain: String,
    /// The first value satisfying the predicate, if any.
    pub witness: Option<i64>,
    /// The largest value searched without finding one.
    pub covered_to: Option<i64>,
    /// Lean's error, for a predicate that is not decidable.
    pub message: String,
}
