//! Reading `CortexDB` answers: recall packs, write barriers, and refusals.
//!
//! Pure functions over JSON, kept apart from the HTTP client so every shape
//! the server can send is testable without a server.

use serde_json::Value;

use crate::error::{Error, Result};

/// The longest a single recalled passage may run, in characters.
const PASSAGE_CLIP: usize = 1_200;

/// The singular a `per_layer_limits` key is spelled with.
///
/// `include` is plural and the budget map is singular. An unrecognised key is
/// ignored by the server, so a wrong spelling silently drops the bound.
pub(super) fn singular(layer: &str) -> &str {
    match layer {
        "events" => "event",
        "episodes" => "episode",
        "facts" => "fact",
        "beliefs" => "belief",
        other => other,
    }
}

/// Renders the requested `layers` of a recall pack as Markdown.
///
/// Returns `None` when every requested layer is empty or absent.
#[must_use]
pub fn render_pack(pack: &Value, layers: &[&str]) -> Option<String> {
    let sections: Vec<String> = layers
        .iter()
        .filter_map(|layer| {
            let entries = pack
                .pointer(&format!("/layers/{layer}"))
                .and_then(Value::as_array)
                .filter(|entries| !entries.is_empty())?;
            let rendered = entries
                .iter()
                .enumerate()
                .map(|(index, entry)| format!("{}. {}", index + 1, render_entry(entry)))
                .collect::<Vec<_>>()
                .join("\n");
            Some(format!("### {}\n\n{rendered}", heading(layer)))
        })
        .collect();
    (!sections.is_empty()).then(|| sections.join("\n\n"))
}

fn heading(layer: &str) -> &str {
    match layer {
        "events" => "Notes recorded",
        "facts" => "Facts extracted",
        "beliefs" => "What memory currently holds",
        other => other,
    }
}

/// Renders one entry by the first text field it carries.
///
/// Layers do not share a text field: an event has `content.text`, a fact a
/// `statement`, a belief a `summary`. An entry with none of them is printed
/// whole rather than dropped, because guessing would lose the answer.
fn render_entry(entry: &Value) -> String {
    let text = entry
        .pointer("/content/text")
        .or_else(|| entry.get("statement"))
        .or_else(|| entry.get("summary"))
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map_or_else(|| entry.to_string(), str::to_string);
    clip(text.trim(), PASSAGE_CLIP)
}

fn clip(text: &str, limit: usize) -> String {
    match text.char_indices().nth(limit) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_string(),
    }
}

/// Checks that a write reached `barrier`.
///
/// A response with no `stages_completed` list is accepted: the list is the
/// server's own reporting, and refusing on its absence would fail every
/// write against a build that stopped sending it.
///
/// # Errors
///
/// Returns [`Error::MemoryNotIndexed`] when the list is present and does
/// not contain `barrier`.
pub fn reached_barrier(answer: &Value, barrier: &str) -> Result<()> {
    let Some(stages) = answer.get("stages_completed").and_then(Value::as_array) else {
        return Ok(());
    };
    let reached: Vec<&str> = stages.iter().filter_map(Value::as_str).collect();
    if reached.contains(&barrier) {
        return Ok(());
    }
    Err(Error::MemoryNotIndexed {
        completed: if reached.is_empty() {
            "nothing".to_string()
        } else {
            reached.join(", ")
        },
    })
}

/// The server's `error_code: message`, or the raw body when it is not the
/// documented refusal shape.
pub(super) fn refusal_detail(body: &str) -> String {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let code = parsed
        .as_ref()
        .and_then(|value| value.get("error_code"))
        .and_then(Value::as_str);
    let message = parsed
        .as_ref()
        .and_then(|value| value.get("message"))
        .and_then(Value::as_str);
    match (code, message) {
        (Some(code), Some(message)) => format!("{code}: {message}"),
        (Some(code), None) => code.to_string(),
        _ => clip(body.trim(), 300),
    }
}

/// Reads a `/v1/admin/ready` report: `Ok` when ready and not degraded,
/// otherwise the failing checks as `name: detail`.
pub(super) fn readiness(report: &Value) -> std::result::Result<(), String> {
    if report.get("ready").and_then(Value::as_bool) == Some(false) {
        let failing: Vec<String> = report
            .get("checks")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter(|(_, check)| check.get("ok").and_then(Value::as_bool) == Some(false))
            .map(|(name, check)| {
                let detail = check
                    .get("detail")
                    .and_then(Value::as_str)
                    .unwrap_or("failed");
                format!("{name}: {detail}")
            })
            .collect();
        return Err(if failing.is_empty() {
            "not ready".to_string()
        } else {
            failing.join("; ")
        });
    }
    if report.get("degraded").and_then(Value::as_bool) == Some(true) {
        let provider = report
            .get("embedding_provider")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        return Err(format!("degraded (embedding provider `{provider}`)"));
    }
    Ok(())
}
