//! Unit tests for the generated witness file.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::{witness_module_name, witness_source};

#[test]
fn module_names_capitalise_each_segment() {
    assert_eq!(witness_module_name("Euler.notPrime"), "EulerNotPrime");
    assert_eq!(witness_module_name("p"), "P");
    assert_eq!(witness_module_name("a.b'.c_d"), "ABCd");
    assert_eq!(witness_module_name("_x"), "X");
    assert_eq!(witness_module_name("αβ"), "P");
}

#[test]
fn a_natural_witness_is_decided() {
    let text = witness_source("Supermath.Euler.Search", "Euler.notPrime", "nat", 91);
    assert_eq!(
        text,
        "import Supermath.Euler.Search\n\ntheorem Euler.notPrime_witness : ∃ n : ℕ, Euler.notPrime n := ⟨91, by decide⟩\n"
    );
}

#[test]
fn an_integer_witness_is_ascribed() {
    let text = witness_source("Supermath.A", "P", "int", -3);
    assert!(text.contains("∃ n : ℤ, P n := ⟨(-3 : ℤ), by decide⟩"));
}
