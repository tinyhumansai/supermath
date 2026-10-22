//! Types shared by the grounded tools.

use std::collections::HashMap;

use serde::Serialize;

use crate::source::{Source, SourceClass};

/// A sentence with the sources that support it, by position in the call's
/// source list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Finding {
    pub(crate) text: String,
    pub(crate) sources: Vec<usize>,
}

/// A supported sentence with its resolved sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TurnFinding {
    pub(crate) text: String,
    pub(crate) sources: Vec<Source>,
}

/// What one grounded Gemini turn produced.
#[derive(Debug, Clone, Default)]
pub(crate) struct Turn {
    /// Sentences with a resolved supporting source.
    pub(crate) findings: Vec<TurnFinding>,
    /// Why there are no findings, when there are none.
    pub(crate) unknown: Option<String>,
    /// Things the turn could not establish, such as omitted sentences.
    pub(crate) notes: Vec<String>,
    /// The queries Google ran.
    pub(crate) queries: Vec<String>,
    /// Google's search-suggestions widget.
    pub(crate) entry_point: Option<String>,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    /// Age in seconds of a cached response.
    pub(crate) cached_age: Option<u64>,
    /// Whether a request went out (false for a cache hit).
    pub(crate) requested: bool,
}

/// Sources gathered across findings, deduplicated by URL and ranked.
#[derive(Debug, Default)]
pub(crate) struct SourceTable {
    sources: Vec<Source>,
    support: Vec<usize>,
    slots: HashMap<String, usize>,
}

/// The ranked source list and how first-seen slots map into it.
#[derive(Debug)]
pub(crate) struct Ranking {
    pub(crate) sources: Vec<Source>,
    remap: Vec<Option<usize>>,
}

impl SourceTable {
    /// Registers a finding's sources and returns their slots.
    pub(crate) fn register(&mut self, finding: &TurnFinding) -> Vec<usize> {
        let mut slots = Vec::new();
        for source in &finding.sources {
            let slot = *self.slots.entry(source.url.clone()).or_insert_with(|| {
                self.sources.push(source.clone());
                self.support.push(0);
                self.sources.len() - 1
            });
            if !slots.contains(&slot) {
                self.support[slot] += 1;
                slots.push(slot);
            }
        }
        slots
    }

    /// Sorts preferred classes first, then by findings supported, then by
    /// first appearance, and keeps the first `limit`.
    pub(crate) fn rank(self, prefer: &[SourceClass], limit: usize) -> Ranking {
        let mut order: Vec<usize> = (0..self.sources.len()).collect();
        order.sort_by_key(|&slot| {
            let class = self.sources[slot].class;
            let rank = prefer
                .iter()
                .position(|p| *p == class)
                .unwrap_or(prefer.len());
            (rank, std::cmp::Reverse(self.support[slot]), slot)
        });
        order.truncate(limit);
        let mut remap = vec![None; self.sources.len()];
        for (new, &old) in order.iter().enumerate() {
            remap[old] = Some(new);
        }
        let sources = order.iter().map(|&old| self.sources[old].clone()).collect();
        Ranking { sources, remap }
    }
}

impl Ranking {
    /// Maps first-seen slots to ranked positions, dropping those cut by the
    /// limit.
    pub(crate) fn remap(&self, slots: &[usize]) -> Vec<usize> {
        let mut mapped: Vec<usize> = slots.iter().filter_map(|&s| self.remap[s]).collect();
        mapped.sort_unstable();
        mapped
    }
}
