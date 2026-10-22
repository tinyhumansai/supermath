//! Tests for sentence splitting and for reading a grounded answer.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::{Grounding, Reading, split_sentences};
use crate::gemini::GenerateResponse;

fn sentences(text: &str) -> Vec<&str> {
    split_sentences(text)
        .into_iter()
        .map(|(s, e)| &text[s..e])
        .collect()
}

#[test]
fn splits_on_terminal_punctuation_and_line_breaks() {
    assert_eq!(
        sentences("First one. Second one! Third? Fourth\nFifth."),
        ["First one.", "Second one!", "Third?", "Fourth", "Fifth."]
    );
}

#[test]
fn keeps_decimals_abbreviations_and_list_numbers_together() {
    assert_eq!(
        sentences("The bound is 1.5 times larger. Alweiss et al. improved it."),
        [
            "The bound is 1.5 times larger.",
            "Alweiss et al. improved it."
        ]
    );
    assert_eq!(
        sentences("1. First item.\n2. Second item."),
        ["1. First item.", "2. Second item."]
    );
}

#[test]
fn strips_bullet_markers_and_skips_empty_lines() {
    let text = "* Alpha is true.\n\n- Beta is false.\n\u{2022} Gamma.\n# Heading\n***\n";
    assert_eq!(
        sentences(text),
        ["Alpha is true.", "Beta is false.", "Gamma.", "Heading"]
    );
}

#[test]
fn spans_are_byte_offsets_on_multibyte_text() {
    let text = "Erdős proved it. Szemerédi extended it.";
    let spans = split_sentences(text);
    assert_eq!(&text[spans[1].0..spans[1].1], "Szemerédi extended it.");
}

fn response(value: serde_json::Value) -> GenerateResponse {
    serde_json::from_value(value).unwrap()
}

fn grounded(answer: &str, supports: &serde_json::Value) -> GenerateResponse {
    response(json!({
        "candidates": [{
            "content": {"parts": [{"text": answer}]},
            "finishReason": "STOP",
            "groundingMetadata": {
                "webSearchQueries": ["q1"],
                "groundingChunks": [
                    {"web": {"uri": "https://r/0", "title": "a.org"}},
                    {"web": {"uri": "https://r/1", "title": "b.org"}},
                    {"retrievedContext": {}}
                ],
                "groundingSupports": supports,
                "searchEntryPoint": {"renderedContent": "<div>widget</div>"}
            }
        }],
        "usageMetadata": {"promptTokenCount": 3, "candidatesTokenCount": 4}
    }))
}

#[test]
fn keeps_only_sentences_a_support_covers() {
    let answer = "Alpha holds. Beta holds. Gamma holds.";
    let response = grounded(
        answer,
        &json!([
            {"segment": {"startIndex": 0, "endIndex": 12, "text": "Alpha holds."},
             "groundingChunkIndices": [1, 0, 1]},
            {"segment": {"startIndex": 25, "endIndex": 37, "text": "Gamma holds."},
             "groundingChunkIndices": [2, 9]}
        ]),
    );
    let Reading::Grounded(g) = Grounding::parse(&response) else {
        panic!("expected a grounded reading");
    };
    assert_eq!(g.sentences.len(), 1);
    assert_eq!(g.sentences[0].text, "Alpha holds.");
    assert_eq!(g.sentences[0].sources, [0, 1]);
    assert_eq!(g.unsupported, 2);
    assert_eq!(g.queries, ["q1"]);
    assert_eq!(g.search_entry_point.as_deref(), Some("<div>widget</div>"));
    assert!(g.sources[2].is_none());
    assert!(!g.no_source_found);
}

#[test]
fn a_support_without_an_end_index_uses_its_text_length() {
    let response = grounded(
        "Alpha holds. Beta holds.",
        &json!([{"segment": {"text": "Alpha holds."}, "groundingChunkIndices": [0]}]),
    );
    let Reading::Grounded(g) = Grounding::parse(&response) else {
        panic!("expected a grounded reading");
    };
    assert_eq!(g.sentences.len(), 1);
    assert_eq!(g.unsupported, 1);
}

#[test]
fn a_no_source_sentence_is_not_counted_as_unsupported() {
    let response = grounded("No source found.", &json!([]));
    let Reading::Grounded(g) = Grounding::parse(&response) else {
        panic!("expected a grounded reading");
    };
    assert!(g.no_source_found);
    assert_eq!(g.unsupported, 0);
    assert_eq!(g.sentences.len(), 0);
}

#[test]
fn reads_blocks_empty_answers_and_ungrounded_answers() {
    let blocked = response(json!({"promptFeedback": {"blockReason": "SAFETY"}}));
    assert_eq!(
        Grounding::parse(&blocked),
        Reading::Blocked("SAFETY".into())
    );

    let stopped = response(json!({"candidates": [{"finishReason": "PROHIBITED_CONTENT"}]}));
    assert_eq!(
        Grounding::parse(&stopped),
        Reading::Blocked("PROHIBITED_CONTENT".into())
    );

    let none = response(json!({}));
    assert_eq!(Grounding::parse(&none), Reading::NoAnswer);
    let empty = response(
        json!({"candidates": [{"content": {"parts": [{"text": " "}]}, "finishReason": "STOP"}]}),
    );
    assert_eq!(Grounding::parse(&empty), Reading::NoAnswer);

    let ungrounded =
        response(json!({"candidates": [{"content": {"parts": [{"text": "A claim."}]}}]}));
    assert_eq!(Grounding::parse(&ungrounded), Reading::Ungrounded);
    let no_chunks = response(json!({"candidates": [{
        "content": {"parts": [{"text": "A claim."}]},
        "groundingMetadata": {"webSearchQueries": ["q"]}
    }]}));
    assert_eq!(Grounding::parse(&no_chunks), Reading::Ungrounded);
}
