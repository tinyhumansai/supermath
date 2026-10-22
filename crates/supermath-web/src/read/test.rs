//! Tests for cursors and pagination.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{PAGE_CHARS, paginate, parse_cursor};

#[test]
fn cursors_name_pages_from_two() {
    assert_eq!(parse_cursor(None), Ok(1));
    assert_eq!(parse_cursor(Some("c2")), Ok(2));
    assert_eq!(parse_cursor(Some("c17")), Ok(17));
    for bad in ["c1", "c0", "2", "cx", "", "c-3"] {
        assert!(parse_cursor(Some(bad)).is_err(), "{bad}");
    }
}

#[test]
fn short_text_is_one_page() {
    assert_eq!(paginate("hello"), ["hello"]);
    assert_eq!(paginate(""), [""]);
    let exact = "x".repeat(PAGE_CHARS);
    assert_eq!(paginate(&exact).len(), 1);
}

#[test]
fn long_text_splits_at_line_ends_and_loses_nothing() {
    let line = format!("{}\n", "word ".repeat(40));
    let text = line.repeat(200);
    let pages = paginate(&text);
    assert!(pages.len() > 1);
    assert!(pages.iter().all(|p| p.chars().count() <= PAGE_CHARS));
    assert!(pages[..pages.len() - 1].iter().all(|p| p.ends_with('\n')));
    assert_eq!(pages.concat(), text);
}

#[test]
fn text_without_line_ends_splits_on_the_limit_even_for_multibyte_characters() {
    let text = "é".repeat(PAGE_CHARS * 2 + 5);
    let pages = paginate(&text);
    assert_eq!(pages.len(), 3);
    assert_eq!(pages[0].chars().count(), PAGE_CHARS);
    assert_eq!(pages.concat(), text);
}
