//! Tests for the shared tool helpers and the `Literature` bundle.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde::Deserialize;
use serde_json::json;
use supermath_tool::Coverage;

use super::{NOT_COVERED, cached, coverage, parse_args, parse_cursor, trim_chars};
use crate::{Error, Literature, LiteratureConfig};

#[test]
fn trimming_counts_characters_not_bytes() {
    assert_eq!(trim_chars("short", 10), "short");
    assert_eq!(trim_chars("exactly5", 8), "exactly5");
    assert_eq!(trim_chars("ünïcödé text", 7), "ünïcödé…");
    assert_eq!(trim_chars("trailing   space", 8), "trailing…");
}

#[test]
fn cursors_are_decimal_offsets() {
    assert_eq!(parse_cursor(None).unwrap(), 0);
    assert_eq!(parse_cursor(Some("42")).unwrap(), 42);
    assert!(matches!(
        parse_cursor(Some("-1")),
        Err(Error::InvalidInput(_))
    ));
}

#[test]
fn arguments_are_parsed_strictly() {
    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    struct Args {
        n: u8,
    }
    assert_eq!(parse_args::<Args>(json!({"n": 3})).unwrap().n, 3);
    assert!(parse_args::<Args>(json!({"n": 3, "m": 1})).is_err());
    assert!(parse_args::<Args>(json!({"n": 300})).is_err());
}

#[test]
fn coverage_names_the_source_and_the_gaps() {
    let c = cached(coverage("arxiv"), Some(12));
    assert_eq!(
        c,
        Coverage {
            cached_age: Some(12),
            ..Coverage::new(["arxiv"], NOT_COVERED)
        }
    );
}

#[test]
fn the_bundle_builds_five_distinct_tools() {
    let literature = Literature::new(&LiteratureConfig::default()).unwrap();
    let tools = literature.tools();
    let mut names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "lit_citations",
            "lit_extract_statements",
            "lit_get_paper",
            "lit_oeis",
            "lit_search_arxiv"
        ]
    );
    assert!(std::sync::Arc::strong_count(literature.arxiv()) > 1);
}
