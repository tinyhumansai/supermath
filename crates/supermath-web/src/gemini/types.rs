//! The slice of the Gemini `generateContent` response this crate reads.
//!
//! Field names follow Google's camelCase JSON; every field defaults, because
//! the API omits empty ones.

use serde::Deserialize;

/// A `generateContent` response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GenerateResponse {
    /// The candidate answers; the tools read the first.
    pub candidates: Vec<Candidate>,
    /// Why the prompt was blocked, if it was.
    pub prompt_feedback: Option<PromptFeedback>,
    /// Token counts for the request.
    pub usage_metadata: Option<UsageMetadata>,
}

/// One candidate answer.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Candidate {
    /// The answer's content.
    pub content: Option<Content>,
    /// Why generation stopped, e.g. `STOP` or `SAFETY`.
    pub finish_reason: Option<String>,
    /// What Google Search grounding found.
    pub grounding_metadata: Option<GroundingMetadata>,
}

/// The content of a candidate: text parts.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Content {
    /// The parts, concatenated in order to form the answer text.
    pub parts: Vec<Part>,
}

/// One part of a content.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Part {
    /// The part's text.
    pub text: Option<String>,
}

/// Feedback on the prompt, present when it was blocked.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PromptFeedback {
    /// The block reason, e.g. `SAFETY`.
    pub block_reason: Option<String>,
}

/// Token counts.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UsageMetadata {
    /// Prompt tokens.
    pub prompt_token_count: u64,
    /// Answer tokens.
    pub candidates_token_count: u64,
}

/// Google Search grounding metadata.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GroundingMetadata {
    /// The queries Google actually ran.
    pub web_search_queries: Vec<String>,
    /// The sources, referenced by index from the supports.
    pub grounding_chunks: Vec<GroundingChunk>,
    /// Which span of the answer each source supports.
    pub grounding_supports: Vec<GroundingSupport>,
    /// Google's search-suggestions widget.
    pub search_entry_point: Option<SearchEntryPoint>,
}

/// One grounding source.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct GroundingChunk {
    /// The web page, when the chunk is one.
    pub web: Option<WebChunk>,
}

/// A web grounding source.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WebChunk {
    /// A redirect URI that leads to the page.
    pub uri: String,
    /// The page's title, often only its domain.
    pub title: String,
}

/// A span of the answer and the sources behind it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GroundingSupport {
    /// The supported span.
    pub segment: Segment,
    /// Indices into `grounding_chunks`.
    pub grounding_chunk_indices: Vec<usize>,
}

/// A byte range of the answer text.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Segment {
    /// Inclusive start byte; omitted by the API when zero.
    pub start_index: usize,
    /// Exclusive end byte.
    pub end_index: usize,
    /// The span's text.
    pub text: String,
}

/// Google's search-suggestions widget.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchEntryPoint {
    /// The HTML to render alongside grounded results.
    pub rendered_content: Option<String>,
}
