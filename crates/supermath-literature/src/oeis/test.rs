//! Tests for OEIS answers.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::{Raw, Sequence, parse};

#[test]
fn a_sequence_row_is_capped_and_ids_are_zero_padded() {
    let data: Vec<String> = (0..40).map(|i| i.to_string()).collect();
    let raw: Raw = serde_json::from_value(json!({
        "number": 45, "name": "Fibonacci", "data": data.join(","),
        "formula": ["1","2","3","4","5","6"], "reference": ["1","2","3","4","5","6","7"]
    }))
    .unwrap();
    let row = Sequence::from(raw);
    assert_eq!(row.id, "A000045");
    assert_eq!(row.terms.len(), 30);
    assert_eq!(row.formula.len(), 5);
    assert_eq!(row.references.len(), 5);
}

#[test]
fn huge_and_negative_terms_keep_their_exact_form() {
    let raw: Raw = serde_json::from_value(json!({
        "number": 7, "data": "-3, 12345678901234567890123, ,4"
    }))
    .unwrap();
    let row = Sequence::from(raw);
    assert_eq!(
        row.terms,
        [json!(-3), json!("12345678901234567890123"), json!(4)]
    );
    assert_eq!(row.name, "");
}

#[test]
fn the_three_answer_shapes_parse() {
    assert_eq!(parse(b"null").unwrap().1, Some(0));
    let (rows, total) = parse(br#"[{"number": 1}]"#).unwrap();
    assert_eq!((rows.len(), total), (1, None));
    let (rows, total) = parse(br#"{"count": 9, "results": [{"number": 1}]}"#).unwrap();
    assert_eq!((rows.len(), total), (1, Some(9)));
    let (rows, total) = parse(br#"{"count": 0, "results": null}"#).unwrap();
    assert_eq!((rows.len(), total), (0, Some(0)));
}

#[test]
fn undecodable_answers_are_decode_errors() {
    for bytes in [
        &b"{oops"[..],
        br#"[{"name": "no number"}]"#,
        br#"{"count": "x"}"#,
    ] {
        assert!(matches!(parse(bytes), Err(crate::Error::Decode { .. })));
    }
}
