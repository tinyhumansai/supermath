//! `calc_gcd_lcm`: exact gcd and lcm of two integers.

use async_trait::async_trait;
use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use crate::arith::{Bad, gcd, integer};

/// The gcd and lcm of two non-negative integers, exactly.
///
/// The lcm is computed in `u128` and returned as a decimal string, so it
/// never overflows and survives JSON clients that lose precision past 2^53.
#[derive(Debug, Clone, Copy, Default)]
pub struct CalcGcdLcm;

#[async_trait]
impl Tool for CalcGcdLcm {
    fn name(&self) -> &'static str {
        "calc_gcd_lcm"
    }

    fn description(&self) -> &'static str {
        "Given two non-negative integers (JSON numbers or decimal strings up \
         to 2^64-1), returns their exact greatest common divisor, least \
         common multiple (as a string), and whether they are coprime. \
         gcd(0, 0) is 0, and the lcm with a zero argument is 0."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "a": { "type": ["integer", "string"], "minimum": 0 },
                "b": { "type": ["integer", "string"], "minimum": 0 }
            },
            "required": ["a", "b"],
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
        &[ErrorCode::InvalidInput]
    }

    async fn call(&self, _ctx: &CallContext, args: Value) -> Envelope {
        let mut values = [0u64; 2];
        for (slot, name) in values.iter_mut().zip(["a", "b"]) {
            *slot = match integer(&args, name) {
                Ok(value) => value,
                Err(Bad::NotInteger) => {
                    return Envelope::error(
                        ErrorCode::InvalidInput,
                        format!("`{name}` must be a non-negative integer"),
                    );
                }
                Err(Bad::TooLarge) => {
                    return Envelope::error(
                        ErrorCode::InvalidInput,
                        format!("`{name}` must be at most {}", u64::MAX),
                    );
                }
            };
        }
        let [a, b] = values;
        let g = gcd(a, b);
        let lcm = a
            .checked_div(g)
            .map_or(0, |a| u128::from(a) * u128::from(b));
        Envelope::ok(json!({
            "a": a,
            "b": b,
            "gcd": g,
            "lcm": lcm.to_string(),
            "coprime": g == 1,
        }))
    }
}

#[cfg(test)]
mod test;
