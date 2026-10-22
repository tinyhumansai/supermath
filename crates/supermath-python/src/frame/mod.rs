//! [`Frame`]: the finite set of values a counterexample search covers.
//!
//! A search always states its frame, so "nothing found" says how far it
//! looked. The three shapes are the ones
//! [`py_search_counterexample`](crate::PySearchCounterexample) accepts:
//!
//! | Shape | JSON | Values |
//! | --- | --- | --- |
//! | Range | `{"kind": "range", "from": 0, "to": 100}` | the integers `from..=to`, in order |
//! | Product | `{"kind": "product", "ranges": [{"from": 0, "to": 9}, ...]}` | tuples over a box, in lexicographic order |
//! | Generator | `{"kind": "generator", "entry": "values"}` | what a function in the same file yields |
//!
//! Both ends of a range are inclusive, so a frame `0..=1000000` supports the
//! claim "no counterexample for n <= 10^6".

use serde_json::{Value, json};

use crate::error::{Error, Result};

/// The most dimensions a product frame may have.
pub const MAX_DIMENSIONS: usize = 8;

/// A validated search frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// The integers `from..=to`.
    Range {
        /// The first value.
        from: i64,
        /// The last value, inclusive.
        to: i64,
    },
    /// Tuples over a box of inclusive integer ranges.
    Product {
        /// One `(from, to)` pair per dimension.
        ranges: Vec<(i64, i64)>,
    },
    /// Values yielded by a finite generator function in the searched file.
    Generator {
        /// The generator's name.
        entry: String,
    },
}

impl Frame {
    /// Parses and validates a frame from its JSON form.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`] if the frame is not an object of a known
    /// `kind`, has missing or non-integer bounds, is empty (`from > to`), or
    /// has more than [`MAX_DIMENSIONS`] dimensions.
    pub fn parse(value: &Value) -> Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| invalid("frame must be an object with a kind"))?;
        let kind = object.get("kind").and_then(Value::as_str);
        let allowed: &[&str] = match kind {
            Some("range") => &["kind", "from", "to"],
            Some("product") => &["kind", "ranges"],
            Some("generator") => &["kind", "entry"],
            _ => {
                return Err(invalid(
                    "frame kind must be \"range\", \"product\", or \"generator\"",
                ));
            }
        };
        if let Some(extra) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
            return Err(invalid(format!("frame has an unknown field {extra:?}")));
        }
        match kind {
            Some("range") => {
                let (from, to) = bounds(value)?;
                Ok(Self::Range { from, to })
            }
            Some("product") => {
                let ranges = object
                    .get("ranges")
                    .and_then(Value::as_array)
                    .filter(|ranges| !ranges.is_empty() && ranges.len() <= MAX_DIMENSIONS)
                    .ok_or_else(|| {
                        invalid(format!(
                            "a product frame needs between 1 and {MAX_DIMENSIONS} ranges"
                        ))
                    })?;
                let ranges = ranges.iter().map(bounds).collect::<Result<Vec<_>>>()?;
                Ok(Self::Product { ranges })
            }
            _ => {
                let entry = object
                    .get("entry")
                    .and_then(Value::as_str)
                    .filter(|entry| is_identifier(entry))
                    .ok_or_else(|| invalid("a generator frame needs an entry function name"))?;
                Ok(Self::Generator {
                    entry: entry.to_owned(),
                })
            }
        }
    }

    /// How many values the frame holds, or `None` for a generator, whose size
    /// is known only after it runs. Saturates at `u128::MAX`.
    #[must_use]
    pub fn size(&self) -> Option<u128> {
        match self {
            Self::Range { from, to } => Some(span(*from, *to)),
            Self::Product { ranges } => Some(ranges.iter().fold(1_u128, |total, (from, to)| {
                total.saturating_mul(span(*from, *to))
            })),
            Self::Generator { .. } => None,
        }
    }

    /// The JSON form the service takes.
    #[must_use]
    pub fn to_wire(&self) -> Value {
        match self {
            Self::Range { from, to } => json!({"kind": "range", "from": from, "to": to}),
            Self::Product { ranges } => json!({
                "kind": "product",
                "ranges": ranges.iter().map(|(from, to)| json!({"from": from, "to": to})).collect::<Vec<_>>(),
            }),
            Self::Generator { entry } => json!({"kind": "generator", "entry": entry}),
        }
    }

    /// A one-line statement of the frame, for `coverage` and ledger claims.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Range { from, to } => format!("every integer from {from} to {to}"),
            Self::Product { ranges } => {
                let boxes: Vec<String> = ranges
                    .iter()
                    .map(|(from, to)| format!("[{from}, {to}]"))
                    .collect();
                format!("every tuple in {}", boxes.join(" x "))
            }
            Self::Generator { entry } => format!("every value yielded by {entry}()"),
        }
    }

    /// Whether the search's witness is a tuple (a product frame's value).
    #[must_use]
    pub fn yields_tuples(&self) -> bool {
        matches!(self, Self::Product { .. })
    }

    /// The part of a range frame covered after `checked` values, as
    /// `{"from", "to"}`; `None` when nothing was checked or the shape has no
    /// simple covered prefix.
    #[must_use]
    pub fn covered(&self, checked: u64) -> Option<Value> {
        match self {
            Self::Range { from, .. } if checked > 0 => {
                let last = i128::from(*from) + i128::from(checked) - 1;
                Some(json!({"from": from, "to": last.to_string().parse::<i64>().ok()?}))
            }
            _ => None,
        }
    }
}

/// Whether `name` is a Python identifier made of ASCII letters, digits, and
/// underscores that does not start with a digit.
#[must_use]
pub fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name.len() <= 128
}

fn span(from: i64, to: i64) -> u128 {
    u128::try_from(i128::from(to) - i128::from(from) + 1).unwrap_or(0)
}

fn bounds(value: &Value) -> Result<(i64, i64)> {
    let object = value
        .as_object()
        .filter(|o| o.len() == 2 || (o.len() == 3 && o.contains_key("kind")))
        .ok_or_else(|| invalid("a range needs integer from and to"))?;
    let bound = |name: &str| {
        object
            .get(name)
            .and_then(Value::as_i64)
            .ok_or_else(|| invalid(format!("range {name} must be an integer")))
    };
    let (from, to) = (bound("from")?, bound("to")?);
    if from > to {
        return Err(invalid(format!(
            "the frame is empty: from {from} is greater than to {to}"
        )));
    }
    Ok((from, to))
}

fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidInput(message.into())
}

#[cfg(test)]
mod test;
