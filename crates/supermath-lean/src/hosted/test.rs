//! Unit tests for reading the hosted indexes' replies. The HTTP behaviour is
//! covered by `tests/search.rs` against a loopback stub.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use serde_json::json;

use super::{Index, dotted, leansearch_rows, loogle_rows};
use crate::error::Error;

#[test]
fn loogle_hits_become_rows() {
    let rows = loogle_rows(&json!({"count": 1, "hits": [
        {"name": "add_sq", "type": "(a + b) ^ 2 = a ^ 2 + 2 * a * b + b ^ 2",
         "module": "Mathlib.Algebra.Ring.Commute", "doc": "..."}]}))
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "add_sq");
    assert_eq!(rows[0].module, "Mathlib.Algebra.Ring.Commute");
}

#[test]
fn a_loogle_error_is_invalid_input_and_no_hits_is_empty() {
    let error = loogle_rows(&json!({"error": "parse error"})).unwrap_err();
    assert!(matches!(error, Error::InvalidInput(_)));
    assert!(loogle_rows(&json!({"count": 0})).unwrap().is_empty());
}

#[test]
fn leansearch_results_are_flattened() {
    let rows = leansearch_rows(&json!([[
        {"result": {"name": ["Nat", "Prime", "two_le"], "module_name": ["Mathlib", "Prime"],
                    "signature": "(hp : p.Prime) : 2 ≤ p"}, "distance": 0.1},
        {"result": {"name": "Plain.name", "type": "T"}, "distance": 0.2}
    ]]));
    assert_eq!(rows[0].name, "Nat.Prime.two_le");
    assert_eq!(rows[0].module, "Mathlib.Prime");
    assert_eq!(rows[0].ty, "(hp : p.Prime) : 2 ≤ p");
    assert_eq!(rows[1].name, "Plain.name");
    assert_eq!(rows[1].ty, "T");
    assert!(leansearch_rows(&json!({"unexpected": true})).is_empty());
}

#[test]
fn dotted_handles_every_shape() {
    assert_eq!(dotted(None), "");
    assert_eq!(dotted(Some(&json!(3))), "");
    assert_eq!(dotted(Some(&json!("a.b"))), "a.b");
    assert_eq!(dotted(Some(&json!(["a", "b"]))), "a.b");
}

#[test]
fn indexes_have_names() {
    assert_eq!(Index::Loogle.name(), "loogle");
    assert_eq!(Index::LeanSearch.name(), "leansearch");
}
