//! The five `agent_*` tools.
//!
//! | Tool | Child | Returns |
//! | --- | --- | --- |
//! | [`AgentDigestPaper`] | reader | ledger ids recorded from a paper |
//! | [`AgentFormalize`] | formalizer | Lean files with verdicts the tool computed |
//! | [`AgentReviewAlignment`] | reviewer | `accepted` or `disputed` |
//! | [`AgentRefute`] | refuter | a Lean-confirmed counterexample, or the frames searched |
//! | [`AgentSettle`] | duel prover and duel refuter, in parallel | `proved`, `refuted`, or `unknown` with bounds |
//!
//! They share one shape: parse and admit the call, check what it names
//! exists, run the child on a belt of primitives, then **re-run the
//! primitives that decide the answer** (`lean_check`, `ledger_claims`,
//! `file_read`) and report those. A child's reply is passed through only as
//! clearly labelled untrusted prose, in a few words.

mod digest;
mod formalize;
mod refute;
mod review;
mod settle;
mod shared;
mod verify;

pub use digest::AgentDigestPaper;
pub use formalize::AgentFormalize;
pub use refute::AgentRefute;
pub use review::AgentReviewAlignment;
pub use settle::AgentSettle;
pub(crate) use refute::{frame_of, witness_of};
pub(crate) use shared::{Agentic, Shared, parse_args};
pub(crate) use verify::describe;

#[cfg(test)]
pub(crate) use formalize::valid_lean_path as valid_lean_path_for_tests;

#[cfg(test)]
mod test;
