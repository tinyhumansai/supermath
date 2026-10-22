//! The one HTTP path every source goes through.
//!
//! [`Fetcher::get`] checks the cache, waits for the arXiv rate limit when the
//! caller gives one, sends the request with the supermath `User-Agent`, maps
//! the status onto the crate [`Error`], and caches a success. A 429 or 503
//! becomes [`Error::RateLimited`] with the server's `Retry-After`; nothing is
//! retried silently.

use std::sync::Arc;
use std::time::Duration;

use crate::cache::ResponseCache;
use crate::error::{Error, Result};
use crate::{Clock, LiteratureConfig, RateLimiter};

/// What one request needs beyond its URL.
#[derive(Debug)]
pub(crate) struct Policy<'a> {
    /// The source's name for errors and coverage, such as `arxiv`.
    pub(crate) source: &'static str,
    /// How long a cached answer is trusted; `None` means forever.
    pub(crate) ttl_ms: Option<u64>,
    /// The politeness limiter to wait on before a network request.
    pub(crate) limiter: Option<&'a RateLimiter>,
    /// Extra request headers, such as an API key.
    pub(crate) headers: Vec<(&'static str, String)>,
    /// The wait to report when a 429 carries no `Retry-After`.
    pub(crate) default_retry_ms: u64,
}

/// A response body and, when it came from the cache, its age.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Body {
    pub(crate) bytes: Vec<u8>,
    pub(crate) cached_age_ms: Option<u64>,
}

/// An HTTP client with a cache in front of it.
#[derive(Debug, Clone)]
pub(crate) struct Fetcher {
    client: reqwest::Client,
    cache: ResponseCache,
}

impl Fetcher {
    pub(crate) fn new(config: &LiteratureConfig, clock: Arc<dyn Clock>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(config.user_agent())
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| Error::Unavailable {
                source_name: "http client".into(),
                reason: e.to_string(),
            })?;
        Ok(Self {
            client,
            cache: ResponseCache::new(config.cache_dir.clone(), clock),
        })
    }

    pub(crate) async fn get(&self, url: &str, policy: &Policy<'_>) -> Result<Body> {
        if let Some(hit) = self.cache.get(url, policy.ttl_ms).await {
            return Ok(Body {
                bytes: hit.body,
                cached_age_ms: Some(hit.age_ms),
            });
        }
        if let Some(limiter) = policy.limiter {
            limiter.acquire().await;
        }
        let mut request = self.client.get(url);
        for (name, value) in &policy.headers {
            request = request.header(*name, value);
        }
        let response = request.send().await.map_err(|e| transport(policy, &e))?;
        let status = response.status();
        if status.as_u16() == 429 || status.as_u16() == 503 {
            let retry_after_ms = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map_or(policy.default_retry_ms, |secs| secs.saturating_mul(1000));
            return Err(Error::RateLimited {
                source_name: policy.source.into(),
                retry_after_ms,
            });
        }
        if status.as_u16() == 404 {
            return Err(Error::NotFound(format!(
                "{} has no such resource",
                policy.source
            )));
        }
        if !status.is_success() {
            return Err(Error::Upstream {
                source_name: policy.source.into(),
                status: status.as_u16(),
            });
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|e| transport(policy, &e))?
            .to_vec();
        self.cache.put(url, &bytes).await;
        Ok(Body {
            bytes,
            cached_age_ms: None,
        })
    }
}

fn transport(policy: &Policy<'_>, error: &reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout {
            source_name: policy.source.into(),
        }
    } else {
        Error::Unavailable {
            source_name: policy.source.into(),
            reason: if error.is_connect() {
                "connection failed".into()
            } else {
                "request failed".into()
            },
        }
    }
}

/// Converts a cache age to whole seconds for `Coverage::cached_age`.
pub(crate) fn age_secs(age_ms: Option<u64>) -> Option<u64> {
    age_ms.map(|ms| ms / 1000)
}
