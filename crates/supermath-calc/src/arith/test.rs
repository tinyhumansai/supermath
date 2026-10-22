//! Unit tests for the arithmetic and the integer parser.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::{Bad, FACTOR_LIMIT, factorize, gcd, integer};

#[test]
fn factorize_is_exact_and_reconstructs_its_input() {
    assert_eq!(factorize(2), vec![(2, 1)]);
    assert_eq!(factorize(360), vec![(2, 3), (3, 2), (5, 1)]);
    assert_eq!(factorize(999_983), vec![(999_983, 1)]);
    // The defining invariant: the product of the factorization is the input.
    for n in [2_u64, 97, 1_024, 600_851_475_143, FACTOR_LIMIT] {
        let product: u64 = factorize(n).iter().map(|(p, e)| p.pow(*e)).product();
        assert_eq!(product, n);
    }
}

#[test]
fn gcd_matches_its_definition() {
    assert_eq!(gcd(0, 0), 0);
    assert_eq!(gcd(0, 7), 7);
    assert_eq!(gcd(12, 18), 6);
    assert_eq!(gcd(17, 5), 1);
}

#[test]
fn integer_reads_numbers_and_decimal_strings() {
    assert_eq!(integer(&json!({"n": 7}), "n"), Ok(7));
    assert_eq!(integer(&json!({"n": "007"}), "n"), Ok(7));
    assert_eq!(
        integer(&json!({"n": "18446744073709551615"}), "n"),
        Ok(u64::MAX)
    );
}

#[test]
fn integer_rejects_everything_else() {
    for value in [
        json!(-4),
        json!(1.5),
        json!("-4"),
        json!("+4"),
        json!("4.0"),
        json!(" 4"),
        json!(""),
        json!(null),
        json!([4]),
        json!(true),
    ] {
        assert_eq!(
            integer(&json!({"n": value}), "n"),
            Err(Bad::NotInteger),
            "{value}"
        );
    }
    assert_eq!(integer(&json!({}), "n"), Err(Bad::NotInteger));
}

#[test]
fn integer_flags_digit_strings_too_large_for_u64() {
    assert_eq!(
        integer(&json!({"n": "18446744073709551616"}), "n"),
        Err(Bad::TooLarge)
    );
}

#[test]
fn integer_flags_json_numbers_too_large_for_u64() {
    // Past u64, serde_json reads a bare number as an f64; it is still an
    // integer, just too large, exactly as the string form is.
    for text in [
        r#"{"n": 99999999999999999999999}"#,
        r#"{"n": 18446744073709551616}"#,
        r#"{"n": 1e300}"#,
    ] {
        let arguments: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(integer(&arguments, "n"), Err(Bad::TooLarge), "{text}");
    }
}

#[test]
fn integer_keeps_fractional_and_negative_floats_not_integers() {
    for text in [
        r#"{"n": 2.5}"#,
        r#"{"n": 4.0}"#,
        r#"{"n": -99999999999999999999999}"#,
        r#"{"n": 1e19}"#,
    ] {
        let arguments: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(integer(&arguments, "n"), Err(Bad::NotInteger), "{text}");
    }
}
