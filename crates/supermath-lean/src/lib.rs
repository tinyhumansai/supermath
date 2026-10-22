//! Lean tools for the supermath agent: check a file, find Mathlib facts, work
//! a proof one goal at a time, and get a verdict the ledger can trust.
//!
//! Lean is the toolkit's arbiter. Every tool here works on a **file in the
//! project's Lean package** (see `supermath-files`), never on source passed as
//! an argument, so each answer names a path and a digest anyone can re-check.
//! The Lean toolchain and Mathlib live in a pinned service (`docker/lean/`);
//! this crate speaks its HTTP API and reduces what Lean said to a [`Verdict`].
//!
//! # Entry points
//!
//! - [`Lean`] builds the clients from a [`LeanConfig`] and a
//!   [`supermath_files::Workspace`] and hands out the eight tools
//!   ([`LeanCheck`], [`LeanSearch`], [`LeanDecl`], [`LeanGoals`],
//!   [`LeanTryTactics`], [`LeanSearchCounterexample`], [`LeanBuild`],
//!   [`LeanBlueprint`]) and the [`LeanObserver`] for `file_list`.
//! - [`LeanChecker`] is the checking API without the tool wrapper:
//!   [`LeanChecker::verify`] takes a declaration at a path and returns a
//!   [`CheckReport`] (verdict and [`Provenance`]), and
//!   [`LeanChecker::is_current`] says whether a stored provenance still
//!   matches the files on disk. `supermath-mcp` adapts it to the ledger's
//!   evidence port.
//! - [`LeanClient`] is the HTTP client; [`HostedSearch`] reaches Loogle and
//!   `LeanSearch`.
//!
//! # Example
//!
//! The verdict rules are pure; the declarations a file makes need no service:
//!
//! ```
//! use supermath_lean::{Verdict, declarations};
//!
//! let source = "namespace Demo\ntheorem two_add_two : 2 + 2 = 4 := by\n  sorry\nend Demo\n";
//! let found = declarations(source);
//! assert_eq!(found[0].name, "Demo.two_add_two");
//! assert!(found[0].has_sorry);
//! assert_eq!(Verdict::CheckedWithSorry.as_str(), "checked_with_sorry");
//! ```
//!
//! # What this crate does not hold
//!
//! No Lean toolchain and no Mathlib: those are the service's, so a stack with
//! no `LEAN_URL` answers `BACKEND_UNAVAILABLE` instead of guessing. No ledger:
//! turning a verdict into a `formalised` claim is the ledger's rule, and it
//! reaches this crate only through an adapter. No sandbox: the service's
//! container is the boundary for untrusted Lean source. Depends on
//! `supermath-tool` and `supermath-files` among the workspace crates.

mod checker;
mod client;
mod config;
mod declarations;
mod error;
mod hosted;
mod observer;
mod tools;
mod verdict;

pub use checker::{CheckOutcome, LeanChecker};
pub use client::{
    CheckReply, CheckRequest, CounterReply, CounterRequest, DeclReply, DeclRequest, GoalsReply,
    GoalsRequest, Health, LeanClient, Message, SearchReply, SearchRequest, SearchRow, Sorry,
    Status, TacticResult, TacticsReply, TacticsRequest,
};
pub use config::{
    DEFAULT_LEANSEARCH_URL, DEFAULT_LOOGLE_URL, LEAN_URL_ENV, LEANSEARCH_URL_ENV, LOOGLE_URL_ENV,
    LeanConfig,
};
pub use declarations::{Declaration, declarations};
pub use error::{Error, Result};
pub use hosted::{HostedSearch, Index};
pub use observer::LeanObserver;
pub use tools::{
    ClaimIndex, Lean, LeanBlueprint, LeanBuild, LeanCheck, LeanDecl, LeanGoals, LeanSearch,
    LeanSearchCounterexample, LeanTryTactics,
};
pub use verdict::{
    ALLOWED_AXIOMS, CheckReport, DEBT_AXIOMS, Diagnostic, MAX_ERRORS, MAX_MESSAGE_CHARS,
    Provenance, SORRY_AXIOM, UNTRUSTED_AXIOM, UnknownReason, Verdict,
};
