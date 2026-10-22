//! `calc_factor`: exact prime factorization of one integer.

use async_trait::async_trait;
use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use crate::arith::{Bad, FACTOR_LIMIT, factorize, integer};

/// Factors one integer between 2 and [`FACTOR_LIMIT`] by trial division.
///
/// Below 2 or not an integer is `INVALID_INPUT`; above the limit is
/// `BUDGET_EXCEEDED`, never a partial factorization.
#[derive(Debug, Clone, Copy, Default)]
pub struct CalcFactor;

fn too_large() -> Envelope {
    Envelope::error(
        ErrorCode::BudgetExceeded,
        format!("`n` must be at most {FACTOR_LIMIT}"),
    )
    .with_next(["math.find when connected", "py_run with sympy.factorint(n)"])
}

#[async_trait]
impl Tool for CalcFactor {
    fn name(&self) -> &'static str {
        "calc_factor"
    }

    fn description(&self) -> &'static str {
        "Given an integer between 2 and 10^12 (a JSON number or decimal \
         string), returns its exact prime factorization, whether it is \
         prime, and how many divisors it has."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "n": { "type": ["integer", "string"], "minimum": 2 } },
            "required": ["n"],
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }

    fn permission(&self) -> Permission {
        Permission::Read
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[ErrorCode::InvalidInput, ErrorCode::BudgetExceeded]
    }

    async fn call(&self, _ctx: &CallContext, args: Value) -> Envelope {
        let n = match integer(&args, "n") {
            Ok(n) => n,
            Err(Bad::TooLarge) => return too_large(),
            Err(Bad::NotInteger) => {
                return Envelope::error(
                    ErrorCode::InvalidInput,
                    "`n` must be an integer (a JSON number or decimal string)",
                );
            }
        };
        if n < 2 {
            return Envelope::error(
                ErrorCode::InvalidInput,
                format!("`n` must be at least 2; got {n}"),
            );
        }
        if n > FACTOR_LIMIT {
            return too_large();
        }
        let factors = factorize(n);
        let divisors: u64 = factors.iter().map(|(_, e)| u64::from(*e) + 1).product();
        let rendered = factors
            .iter()
            .map(|(p, e)| {
                if *e == 1 {
                    p.to_string()
                } else {
                    format!("{p}^{e}")
                }
            })
            .collect::<Vec<_>>()
            .join(" * ");
        Envelope::ok(json!({
            "n": n,
            "factorization": rendered,
            "factors": factors
                .iter()
                .map(|(p, e)| json!({"prime": p, "exponent": e}))
                .collect::<Vec<_>>(),
            "is_prime": factors == [(n, 1)],
            "divisor_count": divisors,
        }))
    }
}

#[cfg(test)]
mod test;
