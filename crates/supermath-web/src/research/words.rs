//! Keeping a research brief under its word cap.
//!
//! The cap counts the words of the findings' text. Findings are cut whole,
//! never mid-sentence, and round by round across facets (every facet's first
//! finding, then every facet's second, and so on), so a long facet cannot
//! crowd the others out. The first finding that does not fit ends the brief:
//! it and every finding ranked after it are dropped.

use crate::engine::Finding;

/// Most words of finding text in one brief.
pub(crate) const MAX_BRIEF_WORDS: usize = 1_500;

/// Truncates each facet's findings so their text totals at most `max_words`
/// words, and returns how many findings were dropped.
pub(crate) fn fit_words(facets: &mut [Vec<Finding>], max_words: usize) -> usize {
    let mut keep = vec![0; facets.len()];
    let mut words = 0;
    let rounds = facets.iter().map(Vec::len).max().unwrap_or(0);
    'fill: for round in 0..rounds {
        for (facet, findings) in facets.iter().enumerate() {
            let Some(finding) = findings.get(round) else {
                continue;
            };
            let count = finding.text.split_whitespace().count();
            if words + count > max_words {
                break 'fill;
            }
            words += count;
            keep[facet] = round + 1;
        }
    }
    let mut dropped = 0;
    for (findings, kept) in facets.iter_mut().zip(keep) {
        dropped += findings.len() - kept;
        findings.truncate(kept);
    }
    dropped
}
