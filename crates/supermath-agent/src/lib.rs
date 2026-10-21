//! A math problem-solving agent harness built on the embedded `OpenHuman`
//! runtime, with durable memory in `CortexDB`.
//!
//! The harness reads one problem, recalls what earlier runs recorded about it,
//! runs an `OpenHuman` agent turn, reads a three-valued [`Verdict`] from the
//! reply, and writes the outcome back to memory. The behavior and its
//! constraints are specified in `docs/specs/solving-harness.md`.
//!
//! # Entry points
//!
//! - [`Config::from_env`] reads the model route and memory settings.
//! - [`Problem::from_file`] loads a statement; its file stem names its memory.
//! - [`Harness::start`] boots the runtime; [`Harness::solve`] works one
//!   problem and returns a [`Report`].
//! - [`Toolkit`] is the set of host tools the solver agent gets.
//! - [`Memory`] is the `CortexDB` client, usable on its own.
//!
//! # Example
//!
//! ```
//! use supermath_agent::{Problem, Reason, Verdict};
//!
//! let problem = Problem::new("Odd sums", "Show 1+3+...+(2n-1) = n^2.")?;
//! assert_eq!(problem.slug(), "odd-sums");
//!
//! assert_eq!(Verdict::parse("...\nVERDICT: proved"), Verdict::Proved);
//! // A reply with no verdict line is never read as a positive answer.
//! assert_eq!(
//!     Verdict::parse("I believe it is true."),
//!     Verdict::NotEstablished(Reason::NoVerdict),
//! );
//! # Ok::<(), supermath_agent::Error>(())
//! ```
//!
//! # What this crate does not hold
//!
//! No agent loop, tool runtime, or provider client of its own: those are
//! `OpenHuman`'s. This crate owns only what is specific to solving
//! mathematics: the problem, the verdict, the prompt, and per-problem memory.

mod config;
mod error;
mod harness;
mod memory;
mod problem;
mod toolkit;
mod verdict;

pub use config::{
    Config, DEFAULT_CORTEX_URL, DEFAULT_EMBEDDING_DIMS, DEFAULT_EMBEDDING_MODEL,
    DEFAULT_LADDER_URL, DEFAULT_MODEL, DEFAULT_SCOPE_ROOT, DEFAULT_WORKSPACE,
};
pub use error::{Error, Result};
pub use harness::{Harness, Report};
pub use memory::{Memory, WRITE_BARRIER, reached_barrier, render_pack};
pub use problem::{Problem, slugify};
pub use toolkit::{FACTOR_LIMIT, RECALL_TOOL, Toolkit, factorize, gcd};
pub use verdict::{Reason, VERDICT_PREFIX, Verdict};
