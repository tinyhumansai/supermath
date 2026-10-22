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

/// Where a refusal suggests writing the Python fallback.
const FALLBACK_PATH: &str = "python/work/calc_factor.py";

/// The Python fallback: sympy's exact factorization, for `n` as a string.
const FALLBACK_SOURCE: &str = "from sympy import factorint\n\n\n\
def factor(n):\n    \
\"\"\"The exact prime factorization of n, given as a decimal string.\"\"\"\n    \
factors = sorted(factorint(int(n)).items())\n    \
return {\"n\": n, \"factors\": [[p, e] for p, e in factors], \"exact\": True}\n";

/// The refusal above [`FACTOR_LIMIT`], with `next` naming the calls that
/// factor `n` in Python instead.
///
/// `digits` is `n` exactly when the caller sent it as a decimal string (or a
/// `u64`); a JSON number past `u64` arrived as an `f64` that lost its digits,
/// so then only the file is suggested and the message asks for a string.
fn too_large(digits: Option<String>) -> Envelope {
    let write = json!({"path": FALLBACK_PATH, "content": FALLBACK_SOURCE});
    let mut next = vec![format!("file_write {write}")];
    let message = match digits {
        Some(digits) => {
            let run = json!({"path": FALLBACK_PATH, "entry": "factor", "args": {"n": digits}});
            next.push(format!("py_run {run}"));
            format!("`n` must be at most {FACTOR_LIMIT}")
        }
        None => format!(
            "`n` must be at most {FACTOR_LIMIT}; send larger values as a decimal string \
             so no digit is lost"
        ),
    };
    Envelope::error(ErrorCode::BudgetExceeded, message).with_next(next)
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
            Err(Bad::TooLarge) => {
                return too_large(args.get("n").and_then(Value::as_str).map(str::to_owned));
            }
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
            return too_large(Some(n.to_string()));
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
