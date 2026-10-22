//! The exact arithmetic behind the tools, and the integer argument parser.

use serde_json::Value;

/// The largest integer `calc_factor` accepts.
///
/// Trial division runs to the square root, so this bounds one call at a
/// million divisions. A larger input is refused with the bound named, rather
/// than answered slowly or not at all.
pub const FACTOR_LIMIT: u64 = 1_000_000_000_000;

/// The prime factorization of `n` as `(prime, exponent)` pairs, ascending.
///
/// `n` must be at least 2; the caller enforces [`FACTOR_LIMIT`].
///
/// # Examples
///
/// ```
/// assert_eq!(supermath_calc::factorize(12), vec![(2, 2), (3, 1)]);
/// ```
#[must_use]
pub fn factorize(mut n: u64) -> Vec<(u64, u32)> {
    let mut factors = Vec::new();
    let mut divisor = 2u64;
    while divisor.saturating_mul(divisor) <= n {
        let mut exponent = 0;
        while n.is_multiple_of(divisor) {
            n /= divisor;
            exponent += 1;
        }
        if exponent > 0 {
            factors.push((divisor, exponent));
        }
        divisor += if divisor == 2 { 1 } else { 2 };
    }
    if n > 1 {
        factors.push((n, 1));
    }
    factors
}

/// The greatest common divisor of `a` and `b`; `gcd(0, 0)` is 0.
///
/// # Examples
///
/// ```
/// assert_eq!(supermath_calc::gcd(84, 36), 12);
/// ```
#[must_use]
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Why an argument is not a usable integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Bad {
    /// Missing, negative, fractional, or not a number or decimal string.
    NotInteger,
    /// A non-negative integer too large for `u64`: a decimal string, or a
    /// JSON number past `u64` (which `serde_json` reads as an `f64`).
    TooLarge,
}

/// 2^64, the first value an `f64` holds that no `u64` does. Every `f64` this
/// large is an integer.
const PAST_U64: f64 = 18_446_744_073_709_551_616.0;

/// Reads argument `name` as a non-negative integer, from a JSON number or a
/// string of decimal digits.
pub(crate) fn integer(arguments: &Value, name: &str) -> Result<u64, Bad> {
    match arguments.get(name) {
        Some(Value::Number(number)) => number.as_u64().ok_or_else(|| {
            if number.as_f64().is_some_and(|value| value >= PAST_U64) {
                Bad::TooLarge
            } else {
                Bad::NotInteger
            }
        }),
        Some(Value::String(text))
            if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) =>
        {
            text.parse().map_err(|_| Bad::TooLarge)
        }
        _ => Err(Bad::NotInteger),
    }
}

#[cfg(test)]
mod test;
