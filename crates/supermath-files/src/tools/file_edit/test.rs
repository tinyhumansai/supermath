//! Exact-string edits and the changed-line ranges they report.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::splice::{Edit, Fault, apply};

fn edit(old: &str, new: &str) -> Edit {
    Edit {
        old: old.into(),
        new: new.into(),
    }
}

#[test]
fn replaces_a_single_occurrence_and_reports_its_line() {
    let applied = apply("a\nsorry\nc\n", &[edit("sorry", "simp")]).unwrap();
    assert_eq!(applied.text, "a\nsimp\nc\n");
    assert_eq!(applied.changed_lines(), [(2, 2)]);
}

#[test]
fn refuses_zero_two_and_empty_matches_naming_the_edit() {
    let zero = apply("abc", &[edit("b", "x"), edit("zzz", "y")])
        .err()
        .unwrap();
    assert_eq!(
        zero,
        Fault::NotOnce {
            index: 1,
            occurrences: 0
        }
    );
    let two = apply("a a", &[edit("a", "b")]).err().unwrap();
    assert_eq!(
        two,
        Fault::NotOnce {
            index: 0,
            occurrences: 2
        }
    );
    let overlapping = apply("aaa", &[edit("aa", "b")]).err().unwrap();
    assert_eq!(
        overlapping,
        Fault::NotOnce {
            index: 0,
            occurrences: 2
        }
    );
    let empty = apply("abc", &[edit("b", "x"), edit("", "b")]).err().unwrap();
    assert_eq!(empty, Fault::EmptyOld { index: 1 });
}

#[test]
fn edits_apply_in_order_so_a_later_edit_sees_an_earlier_one() {
    let applied = apply("one", &[edit("one", "two"), edit("two", "three")]).unwrap();
    assert_eq!(applied.text, "three");
}

#[test]
fn multi_byte_text_is_matched_on_character_boundaries() {
    let applied = apply("ℕ → ℕℕ", &[edit("→", "->"), edit("ℕℕ", "ℤ")]).unwrap();
    assert_eq!(applied.text, "ℕ -> ℤ");
    assert!(apply("éé", &[edit("é", "e")]).is_err());
}

#[test]
fn line_ranges_follow_edits_that_shift_earlier_ones() {
    let text = "l1\nl2\nl3\nl4\nl5\n";
    // The first edit grows by two lines; the second sits above it.
    let applied = apply(text, &[edit("l4", "x\ny\nz"), edit("l1", "first")]).unwrap();
    assert_eq!(applied.text, "first\nl2\nl3\nx\ny\nz\nl5\n");
    assert_eq!(applied.changed_lines(), [(1, 1), (4, 6)]);
}

#[test]
fn a_later_edit_below_an_earlier_one_shifts_its_range() {
    let applied = apply("a\nb\nc\nd\n", &[edit("a", "a\na2"), edit("d", "D")]).unwrap();
    assert_eq!(applied.changed_lines(), [(1, 2), (5, 5)]);
}

#[test]
fn overlapping_and_adjacent_changes_merge() {
    let applied = apply("aXb\nc\n", &[edit("X", "12"), edit("12b", "9")]).unwrap();
    assert_eq!(applied.text, "a9\nc\n");
    assert_eq!(applied.changed_lines(), [(1, 1)]);
    let adjacent = apply("a\nb\nc\n", &[edit("a", "A"), edit("b", "B")]).unwrap();
    assert_eq!(adjacent.changed_lines(), [(1, 2)]);
}

#[test]
fn a_later_edit_inside_an_earlier_replacement_merges_spans() {
    let applied = apply("a\nb\n", &[edit("a", "xyz\nuvw"), edit("yz\nu", "Q")]).unwrap();
    assert_eq!(applied.text, "xQvw\nb\n");
    assert_eq!(applied.changed_lines(), [(1, 1)]);
}

#[test]
fn a_deletion_reports_the_line_where_text_was_removed() {
    let applied = apply("a\nb\nc\n", &[edit("b\n", "")]).unwrap();
    assert_eq!(applied.text, "a\nc\n");
    assert_eq!(applied.changed_lines(), [(2, 2)]);
    let at_end = apply("a\nb", &[edit("\nb", "")]).unwrap();
    assert_eq!(at_end.changed_lines(), [(1, 1)]);
    let everything = apply("a", &[edit("a", "")]).unwrap();
    assert_eq!(everything.changed_lines(), [(1, 1)]);
}

#[test]
fn an_edit_that_swallows_an_earlier_span_widens_it() {
    let applied = apply("ab\ncd\n", &[edit("b", "BB"), edit("BB\nc", "z")]).unwrap();
    assert_eq!(applied.text, "azd\n");
    assert_eq!(applied.changed_lines(), [(1, 1)]);
}
