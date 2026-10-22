//! Web tools for supermath: Gemini grounded search, sourced research briefs,
//! and page reading.
//!
//! Much of what is known about a conjecture never reaches arXiv:
//! `MathOverflow` answers, blogs, course notes, errata. This crate surfaces it
//! through Gemini with Google Search grounding and hands the agent the
//! **sources**, not just Gemini's prose about them.
//!
//! # Entry points
//!
//! - [`WebToolkit`] builds the three tools from a [`WebConfig`]:
//!   [`WebSearch`] (`web_search`, direct) and [`WebResearch`]
//!   (`web_research`, deferred) are agentic (Gemini) tools; [`WebRead`]
//!   (`web_read`, deferred) is a primitive.
//! - [`GeminiClient`] and [`Grounding`] are the Gemini request and the
//!   reading of its grounding metadata.
//! - [`resolve_redirect`], [`classify`], and [`extract_main_text`] are the
//!   deterministic pieces, usable on their own.
//!
//! # Example
//!
//! ```
//! use supermath_web::{WebConfig, WebToolkit};
//!
//! // Without `GEMINI_API_KEY` the Gemini tools answer BACKEND_UNAVAILABLE.
//! let toolkit = WebToolkit::new(WebConfig::default())?;
//! let names: Vec<_> = toolkit.tools().iter().map(|t| t.name()).collect();
//! assert_eq!(names, ["web_search", "web_research", "web_read"]);
//! # Ok::<(), supermath_web::Error>(())
//! ```
//!
//! # What this crate does not hold
//!
//! No other search engine: without a Gemini key there is no fallback, so a
//! missing backend is never an answer from a different one. No model reads a
//! page in `web_read`, and no ledger code: a web source may record a claim as
//! `asserted`, never `cited`, and that rule lives in the ledger crate. No
//! `OpenHuman` or MCP types; the composition crate adapts [`Tool`] at its own
//! edge.

mod cache;
mod cap;
mod clock;
mod config;
mod engine;
mod error;
mod extract;
mod gemini;
mod grounding;
mod read;
mod reader;
mod redirect;
mod research;
mod robots;
mod search;
mod source;
mod toolkit;

pub use clock::{Clock, SystemClock};
pub use config::{DEFAULT_BASE_URL, DEFAULT_DAILY_CAP, DEFAULT_MODEL, WebConfig};
pub use error::{Error, Result};
pub use extract::{Extracted, MathKind, extract_main_text};
pub use gemini::{
    Candidate, Content, GeminiClient, GenerateResponse, Generation, GroundingChunk,
    GroundingMetadata, GroundingSupport, Part, PromptFeedback, SearchEntryPoint, Segment,
    UsageMetadata, WebChunk,
};
pub use grounding::{GroundedSentence, GroundedSource, Grounding, Reading, split_sentences};
pub use read::WebRead;
pub use redirect::{MAX_REDIRECTS, redirect_client, resolve_redirect};
pub use research::WebResearch;
pub use search::WebSearch;
pub use source::{CLASS_TABLE, Source, SourceClass, arxiv_id, classify};
pub use supermath_tool::Tool;
pub use toolkit::WebToolkit;
