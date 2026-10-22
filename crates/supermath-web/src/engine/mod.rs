//! The engine the three tools share: the Gemini client, the redirect
//! resolver, the daily cap, and the grounded turn.

mod types;

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use supermath_tool::{CallContext, Demand, Envelope, Usage};
use tokio::task::JoinSet;
use url::Url;

use crate::cap::DailyCap;
use crate::clock::Clock;
use crate::config::WebConfig;
use crate::error::{Error, Result};
use crate::gemini::GeminiClient;
use crate::grounding::{Grounding, Reading};
use crate::redirect::{redirect_client, resolve_redirect};
use crate::source::Source;

pub(crate) use types::{Finding, SourceTable, Turn, TurnFinding};

/// How a bounded call ended.
pub(crate) enum Run<T> {
    /// The future finished, successfully or not.
    Done(Result<T>),
    /// The caller cancelled.
    Cancelled,
}

#[derive(Debug)]
pub(crate) struct Engine {
    pub(crate) config: WebConfig,
    pub(crate) clock: Arc<dyn Clock>,
    gemini: GeminiClient,
    probe: reqwest::Client,
    cap: DailyCap,
}

pub(crate) fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

impl Engine {
    pub(crate) fn new(config: WebConfig, clock: Arc<dyn Clock>) -> Result<Self> {
        Ok(Self {
            gemini: GeminiClient::new(&config, Arc::clone(&clock))?,
            probe: redirect_client()?,
            cap: DailyCap::new(config.daily_cap),
            config,
            clock,
        })
    }

    /// Admits a call: checks the key (for Gemini tools), the daily cap, and
    /// the host's budget. `Some` is the refusal to return.
    pub(crate) fn admit(
        &self,
        ctx: &CallContext,
        needs_key: bool,
        rows: usize,
        iterations: u32,
    ) -> Option<Envelope> {
        if needs_key && !self.gemini.has_api_key() {
            return Some(Error::MissingApiKey.into_envelope());
        }
        if let Err(error) = self.cap.acquire(ctx.project.slug(), self.clock.now_secs()) {
            return Some(error.into_envelope());
        }
        ctx.budget
            .admit(&Demand {
                rows,
                iterations,
                ..Demand::default()
            })
            .err()
            .map(Envelope::from)
    }

    /// Runs `work` within `limit`, giving up when the call is cancelled.
    pub(crate) async fn bounded<T>(
        &self,
        ctx: &CallContext,
        limit: Duration,
        work: impl Future<Output = Result<T>>,
    ) -> Run<T> {
        tokio::select! {
            () = ctx.cancel.cancelled() => Run::Cancelled,
            done = tokio::time::timeout(limit, work) => {
                Run::Done(done.unwrap_or(Err(Error::Timeout)))
            }
        }
    }

    pub(crate) fn usage(
        &self,
        turns: u32,
        queries: usize,
        tokens: (u64, u64),
        started: Instant,
    ) -> Usage {
        Usage {
            model: self.config.model.clone(),
            turns,
            tool_calls: u32::try_from(queries).unwrap_or(u32::MAX),
            input_tokens: tokens.0,
            output_tokens: tokens.1,
            elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        }
    }

    /// One grounded Gemini turn: ask, read the grounding, resolve each used
    /// source, and keep the sentences that still have a source.
    pub(crate) async fn grounded_turn(&self, prompt: &str) -> Result<Turn> {
        let generation = self.gemini.generate(prompt).await?;
        let usage = generation.response.usage_metadata.unwrap_or_default();
        let mut turn = Turn {
            input_tokens: usage.prompt_token_count,
            output_tokens: usage.candidates_token_count,
            cached_age: generation.cached_age,
            requested: generation.cached_age.is_none(),
            ..Turn::default()
        };
        let grounding = match Grounding::parse(&generation.response) {
            Reading::Blocked(reason) => {
                turn.unknown = Some(format!("gemini blocked the request ({reason})"));
                return Ok(turn);
            }
            Reading::NoAnswer => {
                turn.unknown = Some("gemini returned no answer".to_owned());
                return Ok(turn);
            }
            Reading::Ungrounded => {
                turn.unknown = Some(
                    "gemini answered without grounding, so there is no search result to report"
                        .to_owned(),
                );
                return Ok(turn);
            }
            Reading::Grounded(grounding) => grounding,
        };
        turn.queries.clone_from(&grounding.queries);
        turn.entry_point.clone_from(&grounding.search_entry_point);

        let resolved = self.resolve_all(&grounding).await;
        let mut unsupported = grounding.unsupported;
        for sentence in &grounding.sentences {
            let mut sources: Vec<Source> = Vec::new();
            for index in &sentence.sources {
                if let Some(source) = resolved.get(index)
                    && !sources.iter().any(|s| s.url == source.url)
                {
                    sources.push(source.clone());
                }
            }
            if sources.is_empty() {
                unsupported += 1;
            } else {
                turn.findings.push(TurnFinding {
                    text: sentence.text.clone(),
                    sources,
                });
            }
        }

        let used: std::collections::BTreeSet<usize> = grounding
            .sentences
            .iter()
            .flat_map(|s| s.sources.iter().copied())
            .collect();
        let unresolved = used.len() - used.iter().filter(|i| resolved.contains_key(i)).count();
        if unsupported > 0 {
            turn.notes.push(format!(
                "{} omitted",
                plural(unsupported, "unsupported sentence")
            ));
        }
        if unresolved > 0 {
            turn.notes.push(format!(
                "{} could not be resolved and was dropped",
                plural(unresolved, "source redirect")
            ));
        }
        if turn.findings.is_empty() {
            turn.unknown = Some(if grounding.no_source_found {
                "no source found".to_owned()
            } else {
                "no sentence had a resolved supporting source".to_owned()
            });
        }
        Ok(turn)
    }

    /// Resolves every source a supported sentence uses, concurrently.
    async fn resolve_all(&self, grounding: &Grounding) -> std::collections::HashMap<usize, Source> {
        let used: std::collections::BTreeSet<usize> = grounding
            .sentences
            .iter()
            .flat_map(|s| s.sources.iter().copied())
            .collect();
        let hosts = Arc::new(self.config.redirect_hosts.clone());
        let mut tasks = JoinSet::new();
        for index in used {
            let Some(Some(source)) = grounding.sources.get(index) else {
                continue;
            };
            let client = self.probe.clone();
            let hosts = Arc::clone(&hosts);
            let (uri, title) = (source.uri.clone(), source.title.clone());
            let timeout = self.config.redirect_timeout;
            tasks.spawn(async move {
                let url = resolve_redirect(&client, &hosts, &uri, timeout).await;
                (index, title, url)
            });
        }
        let mut resolved = std::collections::HashMap::new();
        while let Some(joined) = tasks.join_next().await {
            if let Ok((index, title, Ok(url))) = joined
                && is_web_url(&url)
            {
                resolved.insert(index, Source::new(&url, title));
            }
        }
        resolved
    }
}

fn is_web_url(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
}
