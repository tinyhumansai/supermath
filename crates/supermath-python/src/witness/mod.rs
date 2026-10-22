//! The generated file a found counterexample is written to.
//!
//! `py_search_counterexample` writes the first witness to
//! `python/work/witnesses/` so it can be re-run with `py_run` (entry `check`)
//! and cited by path and digest. The name is derived from the content, so
//! finding the same witness twice writes the same file once.

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::frame::Frame;

/// The source of a witness file for `predicate` in `module`.
///
/// `witness_json` is the witness as lossless JSON text from the service (a
/// big integer stays a number). JSON has no tuples, so every place the
/// service reports in `witness_tuples` (a path of indices and keys; `[]` is
/// the witness itself) is rebuilt as a tuple, and so is a product frame's
/// witness: `check()` must call the predicate on what the search saw.
#[must_use]
pub(crate) fn render(
    module: &str,
    predicate: &str,
    witness_json: &str,
    witness_tuples: &[Value],
    frame: &Frame,
) -> String {
    let root = Value::Array(Vec::new());
    let mut paths = witness_tuples.to_vec();
    if frame.yields_tuples() && !paths.contains(&root) {
        paths.push(root.clone());
    }
    let loaded = format!("json.loads(\"{}\")", python_string(witness_json));
    let (helper, load) = match paths.as_slice() {
        [] => ("", loaded),
        [only] if *only == root => ("", format!("tuple({loaded})")),
        _ => {
            let paths = python_string(&Value::Array(paths).to_string());
            (
                WITH_TUPLES,
                format!("_with_tuples({loaded}, json.loads(\"{paths}\"))"),
            )
        }
    };
    format!(
        "\"\"\"A counterexample found by py_search_counterexample (generated; do not edit).\n\
         \n\
         Predicate: {module}.{predicate}\n\
         Frame: {frame}\n\
         \"\"\"\n\
         import json\n\
         \n\
         from {module} import {predicate}\n\
         {helper}\
         \n\
         WITNESS = {load}\n\
         \n\
         \n\
         def check():\n    \
             \"\"\"Re-runs the predicate on the witness; `predicate_holds` is False for a counterexample.\"\"\"\n    \
             return {{\"witness\": WITNESS, \"predicate_holds\": {predicate}(WITNESS)}}\n",
        frame = frame.describe(),
    )
}

/// The helper a witness file defines when tuples sit below its root.
/// Deeper paths go first, so each parent is still a list when its child is
/// replaced, and the root, if listed, goes last.
const WITH_TUPLES: &str = "\n\n\
def _with_tuples(value, paths):\n    \
\"\"\"Rebuilds the tuples the search saw at `paths`; JSON has only lists.\"\"\"\n    \
for path in sorted(paths, key=len, reverse=True):\n        \
if not path:\n            \
value = tuple(value)\n            \
continue\n        \
parent = value\n        \
for key in path[:-1]:\n            \
parent = parent[key]\n        \
parent[path[-1]] = tuple(parent[path[-1]])\n    \
return value\n\n";

/// `text` escaped for a double-quoted Python string literal.
fn python_string(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The project-relative path of the witness file for `content`:
/// `python/work/witnesses/witness_<predicate>_<12 hex>.py`.
#[must_use]
pub(crate) fn path_for(predicate: &str, content: &str) -> String {
    let stem: String = predicate
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
        .take(60)
        .collect();
    let digest = hex::encode(Sha256::digest(content.as_bytes()));
    format!("python/work/witnesses/witness_{stem}_{}.py", &digest[..12])
}

#[cfg(test)]
mod test;
