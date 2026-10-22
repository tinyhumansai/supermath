//! Detecting disagreements by comparing numbers and status words.
//!
//! Disagreements are surfaced, never resolved, and no model is asked to
//! adjudicate: the tool only notices that two findings in one facet say
//! different things about what looks like the same statement.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::engine::Finding;

/// Two or more findings that disagree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Disagreement {
    pub(crate) facet: String,
    pub(crate) claims: Vec<Finding>,
}

const OPEN_WORDS: [&str; 4] = ["open", "unsolved", "unresolved", "unproven"];
const SETTLED_WORDS: [&str; 6] = [
    "proved",
    "proven",
    "solved",
    "resolved",
    "disproved",
    "refuted",
];

fn tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|t| {
            t.trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|t| !t.is_empty())
        .collect()
}

fn has_any(tokens: &[String], words: &[&str]) -> bool {
    tokens.iter().any(|t| words.contains(&t.as_str()))
}

fn is_number(token: &str) -> bool {
    token.bytes().any(|b| b.is_ascii_digit())
}

/// Disagreements within one facet's findings.
pub(crate) fn within_facet(facet: &str, findings: &[Finding]) -> Vec<Disagreement> {
    let tokenized: Vec<Vec<String>> = findings.iter().map(|f| tokens(&f.text)).collect();
    let mut found = Vec::new();

    // Status words: some findings call it open, others call it settled.
    let open: Vec<usize> = (0..findings.len())
        .filter(|&i| has_any(&tokenized[i], &OPEN_WORDS) && !has_any(&tokenized[i], &SETTLED_WORDS))
        .collect();
    let settled: Vec<usize> = (0..findings.len())
        .filter(|&i| has_any(&tokenized[i], &SETTLED_WORDS) && !has_any(&tokenized[i], &OPEN_WORDS))
        .collect();
    if !open.is_empty() && !settled.is_empty() {
        let mut both: Vec<usize> = open.into_iter().chain(settled).collect();
        both.sort_unstable();
        found.push(claims(facet, findings, &both));
    }

    // Numbers: the same sentence with different figures.
    let mut reported = BTreeSet::new();
    for i in 0..findings.len() {
        for j in (i + 1)..findings.len() {
            let (a, b) = (&tokenized[i], &tokenized[j]);
            let shape = |t: &[String]| -> Vec<String> {
                t.iter()
                    .map(|w| {
                        if is_number(w) {
                            "#".to_owned()
                        } else {
                            w.clone()
                        }
                    })
                    .collect()
            };
            let figures = |t: &[String]| -> Vec<String> {
                t.iter().filter(|w| is_number(w)).cloned().collect()
            };
            if a.len() >= 3
                && shape(a) == shape(b)
                && figures(a) != figures(b)
                && reported.insert((i, j))
            {
                found.push(claims(facet, findings, &[i, j]));
            }
        }
    }
    found
}

fn claims(facet: &str, findings: &[Finding], indices: &[usize]) -> Disagreement {
    Disagreement {
        facet: facet.to_owned(),
        claims: indices.iter().map(|&i| findings[i].clone()).collect(),
    }
}
