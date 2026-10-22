//! `web_research` against a Gemini stub that answers per facet.

use std::time::Duration;

use serde_json::json;
use supermath_tool::{Budget, ErrorCode, Outcome, Tool};

use crate::support::{Response, assert_no_key, config, ctx, fixture, gemini_stub, toolkit};

/// The facet a request's prompt asks about.
fn facet_of(body: &str) -> &'static str {
    [
        "failed_attempts",
        "partial_results",
        "related_problems",
        "approaches",
        "formalization",
        "status",
    ]
    .into_iter()
    .find(|f| body.contains(&format!("Facet: {f} ")))
    .unwrap_or("unknown")
}

fn answer(request: &crate::support::Request, name: &str) -> Response {
    let host = request.headers.get("host").cloned().unwrap_or_default();
    Response::json(&fixture(name, &format!("http://{host}")))
}

#[tokio::test]
async fn runs_one_request_per_facet_and_shares_sources_across_them() {
    let stub = gemini_stub(|r| answer(r, "grounded.json")).await;
    let envelope = toolkit(config(&stub))
        .research()
        .call(&ctx(), json!({"question": "The sunflower conjecture"}))
        .await;

    assert_eq!(envelope.outcome, Outcome::Ok, "{envelope:?}");
    assert_eq!(stub.count("/v1beta"), 5);
    let facets = envelope.data["facets"].as_object().unwrap();
    assert_eq!(
        facets.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "approaches",
            "failed_attempts",
            "partial_results",
            "related_problems",
            "status"
        ]
    );
    for facet in facets.values() {
        assert_eq!(facet["findings"].as_array().unwrap().len(), 2);
    }
    // Five facets cite the same two pages, numbered once.
    assert_eq!(envelope.data["sources"].as_array().unwrap().len(), 2);
    assert_eq!(envelope.data["untrusted"], true);
    assert_eq!(
        envelope.data["search_entry_points"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        envelope.next,
        [r#"lit_get_paper {"arxiv_id": "1908.08483"}"#]
    );

    let coverage = envelope.coverage.as_ref().unwrap();
    assert_eq!(coverage.searched.len(), 2);
    let usage = envelope.usage.as_ref().unwrap();
    assert_eq!(usage.turns, 5);
    assert_eq!((usage.input_tokens, usage.output_tokens), (600, 225));
    assert_no_key(&envelope);
}

#[tokio::test]
async fn facets_are_chosen_by_the_caller_and_formalization_is_opt_in() {
    let stub = gemini_stub(|r| answer(r, "grounded.json")).await;
    let tool = toolkit(config(&stub)).research();
    let envelope = tool
        .call(
            &ctx(),
            json!({"question": "Q", "facets": ["status", "formalization"], "prefer": ["preprint"]}),
        )
        .await;
    assert!(envelope.is_ok());
    assert_eq!(stub.count("/v1beta"), 2);
    let facets = envelope.data["facets"].as_object().unwrap();
    assert!(facets.contains_key("formalization") && facets.contains_key("status"));
    // Preferred class first.
    assert_eq!(envelope.data["sources"][0]["class"], "preprint");
    let prompts: Vec<String> = stub.requests().into_iter().map(|r| r.body).collect();
    assert!(prompts.iter().any(|p| facet_of(p) == "formalization"));
}

#[tokio::test]
async fn a_facet_that_fails_is_reported_and_the_others_are_returned() {
    let stub = gemini_stub(|r| {
        if facet_of(&r.body) == "failed_attempts" {
            Response::status(429)
        } else {
            answer(r, "grounded.json")
        }
    })
    .await;
    let envelope = toolkit(config(&stub))
        .research()
        .call(&ctx(), json!({"question": "Q"}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert!(
        envelope
            .unknowns
            .iter()
            .any(|u| u.starts_with("facet failed_attempts:"))
    );
    let failed = &envelope.data["facets"]["failed_attempts"];
    assert_eq!(failed["findings"], json!([]));
    assert!(failed["note"].as_str().unwrap().contains("facet failed"));
    assert_eq!(
        envelope.data["facets"]["status"]["findings"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn when_every_facet_fails_the_call_is_an_error() {
    let stub = gemini_stub(|_| Response::status(429).with_header("retry-after", "3")).await;
    let envelope = toolkit(config(&stub))
        .research()
        .call(&ctx(), json!({"question": "Q"}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::RateLimited));
    assert_eq!(envelope.error.unwrap().retry_after_ms, Some(3000));
    assert!(envelope.usage.is_some());
}

#[tokio::test]
async fn a_facet_that_outlives_the_deadline_times_out_alone() {
    let stub = gemini_stub(|r| {
        if facet_of(&r.body) == "approaches" {
            answer(r, "grounded.json").with_delay(Duration::from_secs(3))
        } else {
            answer(r, "grounded.json")
        }
    })
    .await;
    let mut config = config(&stub);
    config.research_timeout = Duration::from_millis(400);
    let envelope = toolkit(config)
        .research()
        .call(&ctx(), json!({"question": "Q"}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert!(
        envelope
            .unknowns
            .iter()
            .any(|u| u.starts_with("facet approaches:"))
    );
}

#[tokio::test]
async fn when_every_facet_times_out_the_call_is_a_timeout() {
    let stub = gemini_stub(|r| answer(r, "grounded.json").with_delay(Duration::from_secs(3))).await;
    let mut config = config(&stub);
    config.research_timeout = Duration::from_millis(100);
    let envelope = toolkit(config)
        .research()
        .call(&ctx(), json!({"question": "Q", "facets": ["status"]}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::Timeout));
}

#[tokio::test]
async fn ungrounded_and_empty_facets_say_no_source_found() {
    let stub = gemini_stub(|r| answer(r, "ungrounded.json")).await;
    let envelope = toolkit(config(&stub))
        .research()
        .call(
            &ctx(),
            json!({"question": "Q", "facets": ["status", "approaches"]}),
        )
        .await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
    assert_eq!(envelope.data["facets"]["status"]["note"], "no source found");
    assert_eq!(envelope.data["sources"], json!([]));
    assert!(
        envelope
            .unknowns
            .iter()
            .any(|u| u.contains("without grounding"))
    );
}

#[tokio::test]
async fn contradictory_status_claims_are_listed_not_resolved() {
    let stub = gemini_stub(|r| answer(r, "disagreement.json")).await;
    let envelope = toolkit(config(&stub))
        .research()
        .call(&ctx(), json!({"question": "Q", "facets": ["status"]}))
        .await;
    let disagreements = envelope.data["disagreements"].as_array().unwrap();
    assert_eq!(disagreements.len(), 1);
    assert_eq!(disagreements[0]["facet"], "status");
    assert_eq!(disagreements[0]["claims"].as_array().unwrap().len(), 2);
    // Both claims stay in the facet's findings.
    assert_eq!(
        envelope.data["facets"]["status"]["findings"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn the_per_facet_and_total_caps_hold() {
    let many = (0..12)
        .map(|i| format!("Claim number {i} is true."))
        .collect::<Vec<_>>()
        .join(" ");
    let stub = gemini_stub(move |r| {
        let host = r.headers.get("host").cloned().unwrap_or_default();
        let supports: Vec<_> = (0..12)
            .map(|i| {
                let sentence = format!("Claim number {i} is true.");
                let start = many.find(&sentence).unwrap();
                json!({"segment": {"startIndex": start, "endIndex": start + sentence.len(), "text": sentence},
                       "groundingChunkIndices": [0]})
            })
            .collect();
        Response::json(
            &json!({"candidates": [{"content": {"parts": [{"text": many}]},
                "groundingMetadata": {"webSearchQueries": ["q"],
                    "groundingChunks": [{"web": {"uri": format!("http://{host}/r/arxiv"), "title": "t"}}],
                    "groundingSupports": supports}}]})
            .to_string(),
        )
    })
    .await;
    let envelope = toolkit(config(&stub))
        .research()
        .call(&ctx(), json!({"question": "Q", "facets": ["status"]}))
        .await;
    assert_eq!(
        envelope.data["facets"]["status"]["findings"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
}

#[tokio::test]
async fn admission_errors_cost_no_request() {
    let stub = gemini_stub(|r| answer(r, "grounded.json")).await;
    let tool = toolkit(config(&stub)).research();

    // Five facets against a two-iteration budget.
    let mut context = ctx();
    context.budget = Budget::agentic(2, 10_000);
    let refused = tool.call(&context, json!({"question": "Q"})).await;
    assert_eq!(refused.code(), Some(ErrorCode::BudgetExceeded));

    for args in [
        json!({}),
        json!({"question": "Q", "facets": ["nope"]}),
        json!({"question": "Q", "facets": []}),
        json!({"question": "Q", "prefer": ["nope"]}),
    ] {
        let envelope = tool.call(&ctx(), args).await;
        assert_eq!(envelope.code(), Some(ErrorCode::InvalidInput));
        assert!(envelope.usage.is_some());
    }
    assert!(stub.requests().is_empty());
}

#[tokio::test]
async fn without_a_key_and_when_cancelled_and_over_the_daily_cap() {
    let stub = gemini_stub(|r| answer(r, "grounded.json").with_delay(Duration::from_secs(5))).await;

    let keyless = supermath_web::WebConfig::default().with_base_url(stub.base.clone());
    let envelope = toolkit(keyless)
        .research()
        .call(&ctx(), json!({"question": "Q"}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::BackendUnavailable));

    let context = ctx();
    context.cancel.cancel();
    let cancelled = toolkit(config(&stub))
        .research()
        .call(&context, json!({"question": "Q"}))
        .await;
    assert_eq!(cancelled.outcome, Outcome::Unknown);

    let mut capped = config(&stub);
    capped.daily_cap = 0;
    let refused = toolkit(capped)
        .research()
        .call(&ctx(), json!({"question": "Q"}))
        .await;
    assert_eq!(refused.code(), Some(ErrorCode::BudgetExceeded));
}
