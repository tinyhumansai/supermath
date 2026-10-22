//! The generated file a found counterexample is written to.
//!
//! `py_search_counterexample` writes the first witness to
//! `python/work/witnesses/` so it can be re-run with `py_run` (entry `check`)
//! and cited by path and digest. The name is derived from the content, so
//! finding the same witness twice writes the same file once.

use sha2::{Digest, Sha256};

use crate::frame::Frame;

/// The source of a witness file for `predicate` in `module`.
///
/// `witness_json` is the witness as lossless JSON text from the service (a
/// big integer stays a number). A product frame's witness is rebuilt as a
/// tuple, because the predicate takes one.
#[must_use]
pub(crate) fn render(module: &str, predicate: &str, witness_json: &str, frame: &Frame) -> String {
    let literal = witness_json.replace('\\', "\\\\").replace('"', "\\\"");
    let load = if frame.yields_tuples() {
        format!("tuple(json.loads(\"{literal}\"))")
    } else {
        format!("json.loads(\"{literal}\")")
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
