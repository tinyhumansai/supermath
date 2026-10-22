//! [`Provenance`]: everything needed to re-run a result, and the canonical
//! JSON form its argument digest is taken over.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// What a run was, so it can be repeated and its staleness judged.
///
/// `file_sha256` and `imports_sha256` come from the project workspace;
/// `python`, `lock_sha256`, `elapsed_ms`, and `peak_rss_mb` from the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// The project file that ran, e.g. `python/work/sunflower_search.py`.
    pub path: String,
    /// The lowercase hex SHA-256 of that file's bytes.
    pub file_sha256: String,
    /// The digest over every `work.*` module the file imports, transitively.
    pub imports_sha256: String,
    /// The function that was called (the predicate, for a search).
    pub entry: String,
    /// The SHA-256 of the canonical JSON of the arguments (of the frame, for a
    /// search); see [`canonical_json`].
    pub args_sha256: String,
    /// The seed the run was given.
    pub seed: u64,
    /// The Python version that ran it.
    pub python: String,
    /// The digest of the service's `uv.lock`, or `None` if it has none.
    pub lock_sha256: Option<String>,
    /// How long the run took, in milliseconds.
    pub elapsed_ms: u64,
    /// The run's peak resident memory in MiB, when the run reported it.
    pub peak_rss_mb: Option<u64>,
}

/// The canonical JSON text of `value`: object keys sorted, no whitespace, and
/// numbers and strings as `serde_json` prints them.
///
/// Two argument objects that differ only in key order have the same text, so
/// they have the same `args_sha256`.
///
/// ```
/// use serde_json::json;
/// use supermath_python::canonical_json;
///
/// assert_eq!(
///     canonical_json(&json!({"b": [1, 2], "a": "x"})),
///     r#"{"a":"x","b":[1,2]}"#,
/// );
/// ```
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                write_canonical(&map[key], out);
            }
            out.push('}');
        }
        scalar => {
            // Writing to a String cannot fail.
            let _ = write!(out, "{scalar}");
        }
    }
}

/// The lowercase hex SHA-256 of [`canonical_json`] of `value`.
#[must_use]
pub fn args_sha256(value: &Value) -> String {
    hex::encode(Sha256::digest(canonical_json(value).as_bytes()))
}

#[cfg(test)]
mod test;
