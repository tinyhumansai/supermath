//! Tests for facets, argument handling, and disagreement detection.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::Request;
use super::disagree::within_facet;
use super::facet::{DEFAULT_FACETS, Facet};
use crate::engine::Finding;

fn finding(text: &str, source: usize) -> Finding {
    Finding {
        text: text.to_owned(),
        sources: vec![source],
    }
}

#[test]
fn every_facet_has_a_distinct_name_that_parses_back() {
    for facet in Facet::ALL {
        assert_eq!(Facet::parse(facet.name()), Some(facet));
        assert!(
            facet
                .prompt("The Sendov conjecture")
                .contains("Problem: The Sendov conjecture")
        );
    }
    assert_eq!(Facet::parse("vibes"), None);
    assert_eq!(DEFAULT_FACETS.len(), 5);
    assert!(!DEFAULT_FACETS.contains(&Facet::Formalization));
}

#[test]
fn the_same_arguments_write_the_same_prompts() {
    let a = Facet::Status.prompt("Q");
    assert_eq!(a, Facet::Status.prompt("Q"));
    assert_ne!(a, Facet::Approaches.prompt("Q"));
}

#[test]
fn parses_defaults_and_dedupes_facets() {
    let request = Request::parse(json!({"question": "Q"})).unwrap();
    assert_eq!(request.facets, DEFAULT_FACETS);
    let request = Request::parse(json!({
        "question": "Q", "facets": ["status", "status", "formalization"], "prefer": ["qa"]
    }))
    .unwrap();
    assert_eq!(request.facets, [Facet::Status, Facet::Formalization]);
    assert_eq!(request.prefer.len(), 1);
}

#[test]
fn refuses_bad_arguments() {
    let long = "x".repeat(501);
    for args in [
        json!({}),
        json!({"question": ""}),
        json!({"question": long}),
        json!({"question": "Q", "facets": ["nope"]}),
        json!({"question": "Q", "facets": []}),
        json!({"question": "Q", "prefer": ["nope"]}),
        json!({"question": "Q", "bogus": true}),
    ] {
        assert!(Request::parse(args.clone()).is_err(), "{args}");
    }
}

#[test]
fn flags_open_against_settled() {
    let findings = [
        finding("The conjecture remains open in general.", 0),
        finding("The conjecture was proved by Smith in 2020.", 1),
        finding("Degree 9 was checked by computer.", 2),
    ];
    let found = within_facet("status", &findings);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].facet, "status");
    assert_eq!(found[0].claims.len(), 2);
}

#[test]
fn a_finding_that_says_both_is_not_a_disagreement() {
    let findings = [
        finding("Proved for degree 9 and open in general.", 0),
        finding("It is open.", 1),
    ];
    assert_eq!(within_facet("status", &findings).len(), 0);
}

#[test]
fn flags_the_same_sentence_with_different_figures() {
    let findings = [
        finding("The best known bound is (log k)^{2019} due to Smith.", 0),
        finding("The best known bound is (log k)^{2021} due to Smith.", 1),
        finding("The best known bound is (log k)^{2021} due to Smith.", 2),
    ];
    let found = within_facet("partial_results", &findings);
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|d| d.claims.len() == 2));
}

#[test]
fn short_or_unrelated_sentences_do_not_disagree() {
    let findings = [
        finding("Bound 3.", 0),
        finding("Bound 4.", 1),
        finding("Another thing entirely.", 2),
    ];
    assert_eq!(within_facet("approaches", &findings).len(), 0);
}
