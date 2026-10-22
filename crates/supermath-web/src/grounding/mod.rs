//! [`Grounding`]: what a Gemini answer is worth once its sources are read.
//!
//! Gemini's prose is a guide to its sources and nothing more. This module
//! splits the answer into sentences and keeps a sentence only when a
//! grounding support covers it, attaching the sources that support it. Every
//! other sentence is counted and dropped.

mod sentence;

use crate::gemini::GenerateResponse;

pub use sentence::split_sentences;

/// Candidate finish reasons that mean the model refused to answer.
const BLOCKING_FINISH_REASONS: [&str; 5] = [
    "SAFETY",
    "PROHIBITED_CONTENT",
    "BLOCKLIST",
    "SPII",
    "IMAGE_SAFETY",
];

/// The phrase the prompts tell Gemini to answer with when it finds nothing.
const NO_SOURCE_PHRASE: &str = "no source found";

/// A source Gemini grounded on, before its redirect is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundedSource {
    /// The grounding redirect URI.
    pub uri: String,
    /// The title Google gave it.
    pub title: String,
}

/// One answer sentence a grounding support covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundedSentence {
    /// The sentence.
    pub text: String,
    /// Indices into [`Grounding::sources`], ascending and without repeats.
    pub sources: Vec<usize>,
}

/// A grounded answer: supported sentences, their sources, and the metadata
/// Google asks to be passed through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grounding {
    /// The queries Google ran.
    pub queries: Vec<String>,
    /// The sources, indexed by chunk position; `None` for a chunk that is not
    /// a web page.
    pub sources: Vec<Option<GroundedSource>>,
    /// The sentences a source supports.
    pub sentences: Vec<GroundedSentence>,
    /// How many sentences had no supporting source and were dropped.
    pub unsupported: usize,
    /// Whether the answer said it found no source.
    pub no_source_found: bool,
    /// Google's search-suggestions widget, to pass through unchanged.
    pub search_entry_point: Option<String>,
}

/// How Gemini's reply reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    /// Gemini refused; carries the reason.
    Blocked(String),
    /// There was no answer text.
    NoAnswer,
    /// There was an answer but no grounding: a model talking, not a search
    /// result.
    Ungrounded,
    /// A grounded answer.
    Grounded(Grounding),
}

impl Grounding {
    /// Reads a response's first candidate.
    ///
    /// Support spans are byte offsets into the concatenated answer text, as
    /// Google defines them.
    #[must_use]
    pub fn parse(response: &GenerateResponse) -> Reading {
        if let Some(reason) = response
            .prompt_feedback
            .as_ref()
            .and_then(|f| f.block_reason.clone())
        {
            return Reading::Blocked(reason);
        }
        let Some(candidate) = response.candidates.first() else {
            return Reading::NoAnswer;
        };
        let answer: String = candidate
            .content
            .iter()
            .flat_map(|c| &c.parts)
            .filter_map(|p| p.text.as_deref())
            .collect();
        if answer.trim().is_empty() {
            return match candidate.finish_reason.as_deref() {
                Some(reason) if BLOCKING_FINISH_REASONS.contains(&reason) => {
                    Reading::Blocked(reason.to_owned())
                }
                _ => Reading::NoAnswer,
            };
        }
        let Some(meta) = candidate
            .grounding_metadata
            .as_ref()
            .filter(|m| !m.grounding_chunks.is_empty())
        else {
            return Reading::Ungrounded;
        };

        let sources: Vec<Option<GroundedSource>> = meta
            .grounding_chunks
            .iter()
            .map(|chunk| {
                chunk
                    .web
                    .as_ref()
                    .filter(|web| !web.uri.is_empty())
                    .map(|web| GroundedSource {
                        uri: web.uri.clone(),
                        title: web.title.clone(),
                    })
            })
            .collect();

        let no_source_found = answer.to_ascii_lowercase().contains(NO_SOURCE_PHRASE);
        let mut sentences = Vec::new();
        let mut unsupported = 0;
        for (start, end) in split_sentences(&answer) {
            let mut indices: Vec<usize> = meta
                .grounding_supports
                .iter()
                .filter(|support| {
                    let segment = &support.segment;
                    let seg_end = if segment.end_index == 0 {
                        segment.start_index + segment.text.len()
                    } else {
                        segment.end_index
                    };
                    segment.start_index < end && seg_end > start
                })
                .flat_map(|support| support.grounding_chunk_indices.iter().copied())
                .filter(|index| matches!(sources.get(*index), Some(Some(_))))
                .collect();
            indices.sort_unstable();
            indices.dedup();
            let text = answer[start..end].to_owned();
            if !indices.is_empty() {
                sentences.push(GroundedSentence {
                    text,
                    sources: indices,
                });
            } else if !text.to_ascii_lowercase().contains(NO_SOURCE_PHRASE) {
                unsupported += 1;
            }
        }

        Reading::Grounded(Self {
            queries: meta.web_search_queries.clone(),
            sources,
            sentences,
            unsupported,
            no_source_found,
            search_entry_point: meta
                .search_entry_point
                .as_ref()
                .and_then(|e| e.rendered_content.clone()),
        })
    }
}

#[cfg(test)]
mod test;
