//! [`GeminiClient`]: one grounded `generateContent` request.
//!
//! ```text
//! POST {base}/v1beta/models/{model}:generateContent
//! x-goog-api-key: $GEMINI_API_KEY
//! {"contents": [...], "tools": [{"google_search": {}}], "generationConfig": {"temperature": 0}}
//! ```
//!
//! The client adds the 24-hour response cache in front of the request and
//! never logs, prints, or returns the key.

mod types;

use std::sync::Arc;

use serde_json::{Value, json};

use crate::cache::ResponseCache;
use crate::clock::Clock;
use crate::config::WebConfig;
use crate::error::{Error, Result};

pub use types::{
    Candidate, Content, GenerateResponse, GroundingChunk, GroundingMetadata, GroundingSupport,
    Part, PromptFeedback, SearchEntryPoint, Segment, UsageMetadata, WebChunk,
};

/// Retry delay assumed for a 429 that names none.
const DEFAULT_RETRY_AFTER_MS: u64 = 30_000;

/// What one request produced.
#[derive(Debug, Clone)]
pub struct Generation {
    /// The decoded response.
    pub response: GenerateResponse,
    /// How old the cached response is, in seconds; `None` for a fresh one.
    pub cached_age: Option<u64>,
}

/// A client for the Gemini API with Google Search grounding.
#[derive(Clone)]
pub struct GeminiClient {
    http: reqwest::Client,
    api_key: Option<String>,
    base_url: String,
    model: String,
    cache: Option<ResponseCache>,
    clock: Arc<dyn Clock>,
}

impl std::fmt::Debug for GeminiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiClient")
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("cached", &self.cache.is_some())
            .finish_non_exhaustive()
    }
}

impl GeminiClient {
    /// Builds a client from `config`; `clock` times cache entries.
    ///
    /// # Errors
    ///
    /// [`Error::Client`] if the HTTP client cannot be built.
    pub fn new(config: &WebConfig, clock: Arc<dyn Clock>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(config.user_agent())
            .build()
            .map_err(|e| Error::Client(e.without_url().to_string()))?;
        Ok(Self {
            http,
            api_key: config.api_key().map(str::to_owned),
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            cache: config.cache_dir.clone().map(ResponseCache::new),
            clock,
        })
    }

    /// The model this client asks.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Whether an API key is configured.
    #[must_use]
    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    /// Asks Gemini `prompt` with Google Search grounding at temperature 0.
    ///
    /// A cached response younger than 24 hours is returned without a
    /// request.
    ///
    /// # Errors
    ///
    /// - [`Error::MissingApiKey`] without `GEMINI_API_KEY`.
    /// - [`Error::RateLimited`] on a 429, with the `Retry-After` it sent.
    /// - [`Error::ModelNotFound`] on a 404: the model name is likely retired.
    /// - [`Error::BillingExhausted`] on a 402: the key is out of credit.
    /// - [`Error::Unavailable`] on any other failure status or a failed
    ///   request.
    /// - [`Error::Timeout`] if the request itself times out.
    /// - [`Error::Decode`] if the body is not a Gemini response.
    pub async fn generate(&self, prompt: &str) -> Result<Generation> {
        let key = self.api_key.as_deref().ok_or(Error::MissingApiKey)?;
        let cache_key = ResponseCache::key(&self.model, prompt);
        let now = self.clock.now_secs();
        if let Some(cache) = &self.cache
            && let Some(hit) = cache.get(&cache_key, now).await
            && let Ok(response) = serde_json::from_value(hit.response)
        {
            return Ok(Generation {
                response,
                cached_age: Some(hit.age_secs),
            });
        }

        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        );
        let body = json!({
            "contents": [{"role": "user", "parts": [{"text": prompt}]}],
            "tools": [{"google_search": {}}],
            "generationConfig": {"temperature": 0},
        });
        let reply = self
            .http
            .post(url)
            .header("x-goog-api-key", key)
            .json(&body)
            .send()
            .await
            .map_err(request_error)?;

        let status = reply.status();
        if status.as_u16() == 429 {
            let retry_after_ms = reply
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map_or(DEFAULT_RETRY_AFTER_MS, |secs| secs.saturating_mul(1000));
            return Err(Error::RateLimited { retry_after_ms });
        }
        if status.as_u16() == 404 {
            return Err(Error::ModelNotFound {
                model: self.model.clone(),
            });
        }
        if status.as_u16() == 402 {
            return Err(Error::BillingExhausted);
        }
        if !status.is_success() {
            return Err(Error::Unavailable(format!(
                "gemini answered with status {}",
                status.as_u16()
            )));
        }
        let raw: Value = reply.json().await.map_err(|e| {
            if e.is_timeout() {
                Error::Timeout
            } else {
                Error::Decode(e.without_url().to_string())
            }
        })?;
        let response =
            serde_json::from_value(raw.clone()).map_err(|e| Error::Decode(e.to_string()))?;
        if let Some(cache) = &self.cache {
            cache.put(&cache_key, &raw, now).await;
        }
        Ok(Generation {
            response,
            cached_age: None,
        })
    }
}

fn request_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout
    } else {
        Error::Unavailable(error.without_url().to_string())
    }
}
