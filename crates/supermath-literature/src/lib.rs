//! Literature tools for the supermath agent: arXiv, Semantic Scholar, OEIS.
//!
//! Before attacking a conjecture, find out what is known. This crate holds the
//! five `lit_*` tools that find and read papers, and the clients and parsers
//! underneath them. No tool here calls a model; reading for meaning is a
//! sub-agent's job.
//!
//! # Entry points
//!
//! - [`Literature`] builds the shared clients from a [`LiteratureConfig`] and
//!   hands out the tools: [`LitSearchArxiv`] (`lit_search_arxiv`),
//!   [`LitGetPaper`] (`lit_get_paper`), [`LitExtractStatements`]
//!   (`lit_extract_statements`), [`LitCitations`] (`lit_citations`) and
//!   [`LitOeis`] (`lit_oeis`).
//! - [`ArxivClient`], [`SemanticScholarClient`] and [`OeisClient`] are the
//!   clients; [`RateLimiter`] and [`ResponseCache`] keep them polite.
//! - [`flatten_latex`] and [`extract_statements`] are the pure LaTeX parsers.
//!
//! # Example
//!
//! Extract a paper's statements from its LaTeX, offline:
//!
//! ```
//! use supermath_literature::{SourceTree, extract_statements, flatten_latex};
//!
//! let mut tree = SourceTree::new();
//! tree.insert(
//!     "main.tex",
//!     "\\documentclass{article}\n\
//!      \\newtheorem{theorem}{Theorem}\n\
//!      \\begin{document}\\section{Intro}\n\
//!      \\begin{theorem}\\label{thm:main} Every even $n>2$ is a sum of two primes. \\end{theorem}\n\
//!      \\end{document}"
//!         .to_owned(),
//! );
//! let flat = flatten_latex(&tree);
//! let found = extract_statements(&flat.text);
//! assert_eq!(found.statements[0].number.as_deref(), Some("1"));
//! assert_eq!(found.statements[0].label.as_deref(), Some("thm:main"));
//! ```
//!
//! # What this crate does not hold
//!
//! No model call, no PDF text extraction (a PDF-only paper is reported in
//! `unknowns`), and no coverage of zbMATH, `MathSciNet`, journals outside arXiv
//! or books: every search names them in `coverage.not_covered`. Persisting a
//! fact read from a paper is the ledger's job. The crate depends on
//! `supermath-tool` alone among the workspace crates.

mod arxiv;
mod cache;
mod clock;
mod config;
mod eprint;
mod error;
mod http;
mod latex;
mod limiter;
mod oeis;
mod scholar;
mod statements;
mod tools;

pub use arxiv::{ArxivClient, ArxivId, Paper, Read, SearchPage, SearchQuery, SortOrder};
pub use cache::{CachedBody, ResponseCache, SEARCH_TTL_MS};
pub use clock::{Clock, ManualClock, SystemClock};
pub use config::{
    DEFAULT_ARXIV_API_URL, DEFAULT_ARXIV_EPRINT_URL, DEFAULT_OEIS_URL, DEFAULT_SCHOLAR_URL,
    LiteratureConfig,
};
pub use eprint::{Source, unpack};
pub use error::{Error, Result};
pub use latex::{Flattened, Heading, Outline, SourceTree, flatten_latex, outline};
pub use limiter::RateLimiter;
pub use oeis::{OeisClient, OeisPage, Sequence};
pub use scholar::{CitationPage, CitedPaper, Direction, SemanticScholarClient};
pub use statements::{Statement, Statements, extract_statements};
pub use tools::{
    LitCitations, LitExtractStatements, LitGetPaper, LitOeis, LitSearchArxiv, Literature,
};
