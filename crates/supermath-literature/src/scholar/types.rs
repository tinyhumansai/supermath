//! Semantic Scholar rows.

use serde::Serialize;

/// Which way along the citation graph to look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Papers this paper cites.
    References,
    /// Papers that cite this paper.
    CitedBy,
}

/// One paper on either end of a citation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CitedPaper {
    /// Always `semantic_scholar`.
    pub source: &'static str,
    /// Semantic Scholar's own id, when it has one.
    pub paper_id: Option<String>,
    /// The arXiv id (no version), when the paper has one.
    pub arxiv_id: Option<String>,
    /// The DOI, when the paper has one.
    pub doi: Option<String>,
    /// The title.
    pub title: Option<String>,
    /// The publication year.
    pub year: Option<u32>,
    /// The venue as Semantic Scholar names it.
    pub venue: Option<String>,
    /// Up to three authors.
    pub authors: Vec<String>,
    /// How many authors the paper has in all.
    pub author_count: usize,
    /// How many papers cite this one.
    pub citation_count: Option<u64>,
}

/// A page of citation rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationPage {
    /// The rows.
    pub papers: Vec<CitedPaper>,
    /// The offset to ask for next, if more remain.
    pub next: Option<u64>,
    /// How old the cached answer was, in seconds, if it was cached.
    pub cached_age_secs: Option<u64>,
}
