//! Tests for frame parsing, sizing, and description.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::{Frame, is_identifier};

fn parse(value: &serde_json::Value) -> crate::Result<Frame> {
    Frame::parse(value)
}

#[test]
fn a_range_is_inclusive_at_both_ends() {
    let frame = parse(&json!({"kind": "range", "from": 0, "to": 9})).unwrap();
    assert_eq!(frame, Frame::Range { from: 0, to: 9 });
    assert_eq!(frame.size(), Some(10));
    assert_eq!(frame.describe(), "every integer from 0 to 9");
    assert_eq!(
        frame.to_wire(),
        json!({"kind": "range", "from": 0, "to": 9})
    );
    assert!(!frame.yields_tuples());
}

#[test]
fn a_one_value_range_is_not_empty() {
    let frame = parse(&json!({"kind": "range", "from": 5, "to": 5})).unwrap();
    assert_eq!(frame.size(), Some(1));
}

#[test]
fn the_widest_range_does_not_overflow() {
    let frame = Frame::Range {
        from: i64::MIN,
        to: i64::MAX,
    };
    assert_eq!(frame.size(), Some(u128::from(u64::MAX) + 1));
}

#[test]
fn a_product_multiplies_its_dimensions() {
    let frame = parse(&json!({
        "kind": "product",
        "ranges": [{"from": 0, "to": 9}, {"from": -2, "to": 2}]
    }))
    .unwrap();
    assert_eq!(frame.size(), Some(50));
    assert!(frame.yields_tuples());
    assert_eq!(frame.describe(), "every tuple in [0, 9] x [-2, 2]");
    assert_eq!(
        frame.to_wire(),
        json!({"kind": "product", "ranges": [{"from": 0, "to": 9}, {"from": -2, "to": 2}]})
    );
}

#[test]
fn a_huge_product_saturates_instead_of_overflowing() {
    let wide = json!({"from": i64::MIN, "to": i64::MAX});
    let frame = parse(&json!({"kind": "product", "ranges": vec![wide; 8]})).unwrap();
    assert_eq!(frame.size(), Some(u128::MAX));
}

#[test]
fn a_generator_has_no_known_size() {
    let frame = parse(&json!({"kind": "generator", "entry": "values"})).unwrap();
    assert_eq!(frame.size(), None);
    assert_eq!(frame.describe(), "every value yielded by values()");
    assert_eq!(
        frame.to_wire(),
        json!({"kind": "generator", "entry": "values"})
    );
}

#[test]
fn malformed_frames_are_invalid_input() {
    let cases = [
        json!("range"),
        json!({}),
        json!({"kind": "bogus"}),
        json!({"kind": "range", "from": 0}),
        json!({"kind": "range", "from": 0, "to": "9"}),
        json!({"kind": "range", "from": 0, "to": 1.5}),
        json!({"kind": "range", "from": 0, "to": 9, "step": 2}),
        json!({"kind": "range", "from": 9, "to": 0}),
        json!({"kind": "product"}),
        json!({"kind": "product", "ranges": []}),
        json!({"kind": "product", "ranges": [[0, 9]]}),
        json!({"kind": "product", "ranges": [{"from": 0}]}),
        json!({"kind": "product", "ranges": vec![json!({"from": 0, "to": 1}); 9]}),
        json!({"kind": "generator"}),
        json!({"kind": "generator", "entry": "not a name"}),
    ];
    for case in cases {
        let error = Frame::parse(&case).unwrap_err();
        assert!(
            matches!(error, crate::Error::InvalidInput(_)),
            "{case} gave {error}"
        );
    }
}

#[test]
fn covered_reports_the_prefix_of_a_range() {
    let frame = Frame::Range { from: 10, to: 100 };
    assert_eq!(frame.covered(0), None);
    assert_eq!(frame.covered(5), Some(json!({"from": 10, "to": 14})));
    assert_eq!(Frame::Generator { entry: "g".into() }.covered(5), None);
}

#[test]
fn covered_gives_up_rather_than_overflow() {
    let frame = Frame::Range {
        from: i64::MAX,
        to: i64::MAX,
    };
    assert_eq!(frame.covered(5), None);
}

#[test]
fn identifiers_follow_python_ascii_rules() {
    assert!(is_identifier("is_prime_value"));
    assert!(is_identifier("_x1"));
    assert!(!is_identifier("1x"));
    assert!(!is_identifier(""));
    assert!(!is_identifier("a-b"));
    assert!(!is_identifier(&"a".repeat(129)));
}
