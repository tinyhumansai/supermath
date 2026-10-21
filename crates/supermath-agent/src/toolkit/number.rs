//! Exact integer tools: `factor_integer` and `gcd_lcm`.
//!
//! Both are deferred: the model does not see them until it asks
//! `tool_search` for something they answer, and Jev decides whether they do.
//! Inputs are typed JSON integers, never expressions, so nothing the model
//! writes is evaluated.

use openhuman_embed::{Tool, ToolExposure, ToolPolicy, ToolResult};
use serde_json::{Value, json};

/// The largest integer `factor_integer` accepts.
///
/// Trial division runs to the square root, so this bounds one call at a
/// million divisions. A larger input is refused with the bound named, rather
/// than answered slowly or not at all.
pub const FACTOR_LIMIT: u64 = 1_000_000_000_000;

/// The prime factorization of `n` as `(prime, exponent)` pairs, ascending.
///
/// `n` must be at least 2; the caller enforces [`FACTOR_LIMIT`].
#[must_use]
pub fn factorize(mut n: u64) -> Vec<(u64, u32)> {
    let mut factors = Vec::new();
    let mut divisor = 2u64;
    while divisor.saturating_mul(divisor) <= n {
        let mut exponent = 0;
        while n % divisor == 0 {
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
#[must_use]
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Reads argument `name` as a non-negative integer that fits in `u64`.
fn integer(arguments: &Value, name: &str) -> Result<u64, ToolResult> {
    arguments
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| ToolResult::error(format!("`{name}` must be a non-negative integer")))
}

/// Factors one integer exactly.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FactorTool;

#[async_trait::async_trait]
impl Tool for FactorTool {
    fn name(&self) -> &'static str {
        "factor_integer"
    }

    fn description(&self) -> &'static str {
        "Exact prime factorization of one integer between 2 and 10^12, \
         with whether it is prime and how many divisors it has."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "n": { "type": "integer", "minimum": 2, "maximum": FACTOR_LIMIT } },
            "required": ["n"],
            "additionalProperties": false
        })
    }

    fn policy(&self) -> ToolPolicy {
        ToolPolicy::read_only()
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::Deferred
    }

    async fn execute(&self, arguments: Value) -> anyhow::Result<ToolResult> {
        let n = match integer(&arguments, "n") {
            Ok(n) => n,
            Err(refusal) => return Ok(refusal),
        };
        if !(2..=FACTOR_LIMIT).contains(&n) {
            return Ok(ToolResult::error(format!(
                "`n` must be between 2 and {FACTOR_LIMIT}; got {n}"
            )));
        }
        let factors = factorize(n);
        let divisors: u64 = factors.iter().map(|(_, e)| u64::from(*e) + 1).product();
        let rendered = factors
            .iter()
            .map(|(p, e)| if *e == 1 { p.to_string() } else { format!("{p}^{e}") })
            .collect::<Vec<_>>()
            .join(" * ");
        Ok(ToolResult::json(json!({
            "n": n,
            "factorization": rendered,
            "factors": factors.iter().map(|(p, e)| json!([p, e])).collect::<Vec<_>>(),
            "is_prime": factors == [(n, 1)],
            "divisor_count": divisors,
        })))
    }
}

/// The gcd and lcm of two integers, exactly.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GcdTool;

#[async_trait::async_trait]
impl Tool for GcdTool {
    fn name(&self) -> &'static str {
        "gcd_lcm"
    }

    fn description(&self) -> &'static str {
        "Exact greatest common divisor and least common multiple of two \
         non-negative integers, and whether they are coprime."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "a": { "type": "integer", "minimum": 0 },
                "b": { "type": "integer", "minimum": 0 }
            },
            "required": ["a", "b"],
            "additionalProperties": false
        })
    }

    fn policy(&self) -> ToolPolicy {
        ToolPolicy::read_only()
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::Deferred
    }

    async fn execute(&self, arguments: Value) -> anyhow::Result<ToolResult> {
        let (a, b) = match (integer(&arguments, "a"), integer(&arguments, "b")) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(refusal), _) | (_, Err(refusal)) => return Ok(refusal),
        };
        let g = gcd(a, b);
        // In u128 so the lcm of two u64 values cannot overflow; sent as a
        // string because JSON numbers lose precision past 2^53.
        let lcm = if g == 0 {
            0
        } else {
            u128::from(a / g) * u128::from(b)
        };
        Ok(ToolResult::json(json!({
            "a": a,
            "b": b,
            "gcd": g,
            "lcm": lcm.to_string(),
            "coprime": g == 1,
        })))
    }
}
