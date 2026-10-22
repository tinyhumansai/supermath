//! Small exact arithmetic tools for the supermath agent.
//!
//! [`CalcFactor`] (`calc_factor`) factors an integer up to [`FACTOR_LIMIT`];
//! [`CalcGcdLcm`] (`calc_gcd_lcm`) returns the gcd and lcm of two integers.
//! Both implement [`supermath_tool::Tool`], are deferred, and are read-only.
//! [`factorize`] and [`gcd`] are the plain arithmetic underneath.
//!
//! # Example
//!
//! ```
//! use serde_json::json;
//! use supermath_calc::{CalcGcdLcm, factorize, gcd};
//! use supermath_tool::Tool;
//!
//! assert_eq!(factorize(360), vec![(2, 3), (3, 2), (5, 1)]);
//! assert_eq!(gcd(84, 36), 12);
//! assert_eq!(CalcGcdLcm.name(), "calc_gcd_lcm");
//! # let _ = json!({});
//! ```
//!
//! # What this crate does not hold
//!
//! No big-integer or symbolic arithmetic: anything beyond a `u64` belongs to
//! `py_run`, which has a real computer-algebra system behind it. No service
//! client and no configuration either; that is why the crate depends on
//! `supermath-tool` alone.

mod arith;
mod factor;
mod gcd_lcm;

pub use arith::{FACTOR_LIMIT, factorize, gcd};
pub use factor::CalcFactor;
pub use gcd_lcm::CalcGcdLcm;
