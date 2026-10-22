//! Tests for Semantic Scholar rows.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{CitedPaper, Page, Raw};

fn page(json: &str) -> Page {
    serde_json::from_str(json).unwrap()
}

#[test]
fn a_reference_row_keeps_ids_and_caps_authors() {
    let parsed = page(
        r#"{"next": 5, "data": [{"citedPaper": {"paperId": "p", "title": "T", "year": 2001,
        "venue": "V", "citationCount": 3,
        "authors": [{"name": "A"}, {"name": null}, {"name": "B"}, {"name": "C"}, {"name": "D"}],
        "externalIds": {"ArXiv": "math/0301234", "DOI": "10.1/x", "MAG": "1"}}}]}"#,
    );
    assert_eq!(parsed.next, Some(5));
    let raw: Raw = parsed.data.into_iter().next().unwrap().cited.unwrap();
    let row = CitedPaper::from(raw);
    assert_eq!(row.source, "semantic_scholar");
    assert_eq!(row.arxiv_id.as_deref(), Some("math/0301234"));
    assert_eq!(row.doi.as_deref(), Some("10.1/x"));
    assert_eq!(row.authors, ["A", "B", "C"]);
    assert_eq!(row.author_count, 5);
    assert_eq!(row.year, Some(2001));
}

#[test]
fn missing_fields_become_none_and_a_blank_venue_is_dropped() {
    let parsed = page(r#"{"data": [{"citingPaper": {"venue": ""}}, {}]}"#);
    assert_eq!(parsed.next, None);
    let raw = parsed.data.into_iter().next().unwrap().citing.unwrap();
    let row = CitedPaper::from(raw);
    assert_eq!(row.venue, None);
    assert_eq!(row.title, None);
    assert!(row.authors.is_empty() && row.arxiv_id.is_none() && row.doi.is_none());
}
