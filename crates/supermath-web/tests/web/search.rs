//! `web_search` against a Gemini stub.

use std::time::Duration;

use serde_json::json;
use supermath_tool::{Budget, ErrorCode, Outcome, Tool};
use supermath_web::WebConfig;

use crate::support::{
    KEY, Response, assert_no_key, config, ctx, fixed_gemini, fixture, gemini_stub, toolkit,
};

#[tokio::test]
async fn a_grounded_answer_becomes_findings_with_resolved_sources() {
    let stub = fixed_gemini("grounded.json").await;
    let tool = toolkit(config(&stub)).search();
    let envelope = tool
        .call(
            &ctx(),
            json!({"question": "What is the best known bound in the sunflower conjecture?"}),
        )
        .await;

    assert_eq!(envelope.outcome, Outcome::Ok, "{envelope:?}");
    let data = &envelope.data;
    assert_eq!(data["untrusted"], true);
    let findings = data["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 2);
    assert!(findings[0]["text"].as_str().unwrap().starts_with("Alweiss"));

    // The blog supports both findings, so it ranks first; redirects are gone.
    let sources = data["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2);
    assert_eq!(
        sources[0]["url"],
        "https://terrytao.wordpress.com/2019/10/01/sunflowers/"
    );
    assert_eq!(sources[0]["class"], "blog");
    assert_eq!(sources[1]["url"], "https://arxiv.org/abs/1908.08483v2");
    assert_eq!(sources[1]["class"], "preprint");
    assert_eq!(sources[1]["arxiv_id"], "1908.08483");
    assert_eq!(findings[0]["sources"], json!([0, 1]));
    assert_eq!(findings[1]["sources"], json!([0]));
    for source in sources {
        assert!(!source["url"].as_str().unwrap().contains("127.0.0.1"));
    }

    // Google's widget is passed through untouched.
    assert_eq!(
        data["search_entry_point"],
        "<div class=\"container\">Search Suggestions</div>"
    );
    let coverage = envelope.coverage.as_ref().unwrap();
    assert_eq!(
        coverage.searched,
        [
            "google: sunflower conjecture best bound",
            "google: sunflower lemma improvement 2021"
        ]
    );
    assert_eq!(coverage.cached_age, None);
    assert_eq!(
        envelope.next,
        [r#"lit_get_paper {"arxiv_id": "1908.08483"}"#]
    );
    assert_eq!(envelope.unknowns.len(), 0);

    let usage = envelope.usage.as_ref().unwrap();
    assert_eq!(usage.model, "gemini-2.5-flash");
    assert_eq!((usage.turns, usage.tool_calls), (1, 2));
    assert_eq!((usage.input_tokens, usage.output_tokens), (120, 45));
    assert_no_key(&envelope);
}

#[tokio::test]
async fn the_request_asks_for_grounding_at_temperature_zero_and_sends_the_key_as_a_header() {
    let stub = fixed_gemini("grounded.json").await;
    let tool = toolkit(config(&stub)).search();
    tool.call(
        &ctx(),
        json!({"question": "sunflowers", "prefer": ["qa"], "since": "2019-01-01"}),
    )
    .await;

    let requests = stub.requests();
    let generate = requests
        .iter()
        .find(|r| r.path.contains(":generateContent"))
        .unwrap();
    assert_eq!(generate.method, "POST");
    assert_eq!(
        generate.path,
        "/v1beta/models/gemini-2.5-flash:generateContent"
    );
    assert_eq!(generate.headers["x-goog-api-key"], KEY);
    assert!(!generate.path.contains(KEY) && !generate.body.contains(KEY));
    let body: serde_json::Value = serde_json::from_str(&generate.body).unwrap();
    assert_eq!(body["tools"], json!([{"google_search": {}}]));
    assert_eq!(body["generationConfig"]["temperature"], 0);
    let prompt = body["contents"][0]["parts"][0]["text"].as_str().unwrap();
    assert!(prompt.contains("Question: sunflowers"));
    assert!(prompt.contains("MathOverflow"));
    assert!(prompt.contains("2019-01-01"));
}

#[tokio::test]
async fn preferred_classes_come_first_and_the_limit_cuts_the_rest() {
    let stub = fixed_gemini("grounded.json").await;
    let tool = toolkit(config(&stub)).search();
    let envelope = tool
        .call(
            &ctx(),
            json!({"question": "q", "prefer": ["preprint"], "limit": 1}),
        )
        .await;
    let sources = envelope.data["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["class"], "preprint");
    // The second finding was supported only by the blog, which the limit cut.
    assert_eq!(envelope.data["findings"].as_array().unwrap().len(), 1);
    assert!(
        envelope
            .unknowns
            .iter()
            .any(|u| u == "1 finding omitted by the source limit")
    );
}

#[tokio::test]
async fn a_since_date_is_flagged_as_unchecked() {
    let stub = fixed_gemini("grounded.json").await;
    let envelope = toolkit(config(&stub))
        .search()
        .call(&ctx(), json!({"question": "q", "since": "2020-05-01"}))
        .await;
    assert!(envelope.unknowns.iter().any(|u| u.contains("2020-05-01")));
}

#[tokio::test]
async fn unsupported_sentences_and_unresolvable_sources_are_dropped_and_counted() {
    let stub = fixed_gemini("unsupported.json").await;
    let envelope = toolkit(config(&stub))
        .search()
        .call(&ctx(), json!({"question": "q"}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    let findings = envelope.data["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["text"], "The conjecture is open.");
    assert!(
        envelope
            .unknowns
            .contains(&"2 unsupported sentences omitted".to_owned())
    );
    assert!(
        envelope
            .unknowns
            .contains(&"1 source redirect could not be resolved and was dropped".to_owned())
    );
    let text = serde_json::to_string(&envelope.data).unwrap();
    assert!(!text.contains("Everyone believes") && !text.contains("dead link"));
}

#[tokio::test]
async fn an_ungrounded_answer_is_unknown_with_no_findings() {
    let stub = fixed_gemini("ungrounded.json").await;
    let envelope = toolkit(config(&stub))
        .search()
        .call(&ctx(), json!({"question": "q"}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
    assert_eq!(envelope.data["findings"], json!([]));
    assert!(
        !serde_json::to_string(&envelope)
            .unwrap()
            .contains("probably true")
    );
    assert!(envelope.usage.is_some());
}

#[tokio::test]
async fn no_source_found_is_unknown_and_keeps_the_queries() {
    let stub = fixed_gemini("no_source.json").await;
    let envelope = toolkit(config(&stub))
        .search()
        .call(&ctx(), json!({"question": "q"}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
    assert_eq!(envelope.unknowns[0], "no source found");
    assert_eq!(
        envelope.coverage.unwrap().searched,
        ["google: obscure query"]
    );
}

#[tokio::test]
async fn a_safety_block_is_unknown_with_the_reason_not_an_error() {
    let stub = fixed_gemini("safety_block.json").await;
    let envelope = toolkit(config(&stub))
        .search()
        .call(&ctx(), json!({"question": "q"}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
    assert!(envelope.unknowns[0].contains("SAFETY"));
}

#[tokio::test]
async fn a_429_is_rate_limited_with_the_retry_after() {
    let stub = gemini_stub(|_| {
        Response::status(429)
            .with_header("retry-after", "7")
            .with_body(fixture("rate_limited.json", "").as_bytes())
    })
    .await;
    let envelope = toolkit(config(&stub))
        .search()
        .call(&ctx(), json!({"question": "q"}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::RateLimited));
    assert_eq!(envelope.error.unwrap().retry_after_ms, Some(7000));
}

#[tokio::test]
async fn a_429_without_retry_after_still_says_when_to_retry() {
    let stub = gemini_stub(|_| Response::status(429)).await;
    let envelope = toolkit(config(&stub))
        .search()
        .call(&ctx(), json!({"question": "q"}))
        .await;
    assert_eq!(envelope.error.unwrap().retry_after_ms, Some(30_000));
}

#[tokio::test]
async fn a_server_error_or_garbage_body_is_backend_unavailable() {
    let down = gemini_stub(|_| Response::status(503)).await;
    let garbage = gemini_stub(|_| Response::json("this is not json")).await;
    let wrong_shape = gemini_stub(|_| Response::json(r#"{"candidates": "nope"}"#)).await;
    for stub in [&down, &garbage, &wrong_shape] {
        let envelope = toolkit(config(stub))
            .search()
            .call(&ctx(), json!({"question": "q"}))
            .await;
        assert_eq!(envelope.code(), Some(ErrorCode::BackendUnavailable));
        assert_no_key(&envelope);
    }
}

#[tokio::test]
async fn a_404_for_the_model_is_backend_unavailable_and_names_the_model_and_override() {
    let stub = gemini_stub(|_| Response::status(404)).await;
    let envelope = toolkit(config(&stub))
        .search()
        .call(&ctx(), json!({"question": "q"}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::BackendUnavailable));
    let message = envelope.error.as_ref().unwrap().message.clone();
    assert!(message.contains(supermath_web::DEFAULT_MODEL), "{message}");
    assert!(message.contains("may be retired"), "{message}");
    assert!(message.contains("SUPERMATH_WEB_MODEL"), "{message}");
    assert_no_key(&envelope);
}

#[tokio::test]
async fn an_unreachable_gemini_is_backend_unavailable() {
    let stub = fixed_gemini("grounded.json").await;
    let mut config = config(&stub);
    config.base_url = "http://127.0.0.1:1".to_owned();
    let envelope = toolkit(config)
        .search()
        .call(&ctx(), json!({"question": "q"}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::BackendUnavailable));
}

#[tokio::test]
async fn without_a_key_the_tool_answers_backend_unavailable_and_asks_nobody() {
    let stub = fixed_gemini("grounded.json").await;
    let config = WebConfig::default().with_base_url(stub.base.clone());
    let envelope = toolkit(config)
        .search()
        .call(&ctx(), json!({"question": "q"}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::BackendUnavailable));
    assert!(envelope.usage.is_some());
    assert!(stub.requests().is_empty());
}

#[tokio::test]
async fn the_daily_cap_refuses_the_call_after_it() {
    let stub = fixed_gemini("grounded.json").await;
    let mut config = config(&stub);
    config.daily_cap = 1;
    let tool = toolkit(config).search();
    assert!(tool.call(&ctx(), json!({"question": "q"})).await.is_ok());
    let second = tool.call(&ctx(), json!({"question": "q"})).await;
    assert_eq!(second.code(), Some(ErrorCode::BudgetExceeded));
    assert_eq!(stub.count("/v1beta"), 1);
}

#[tokio::test]
async fn the_hosts_budget_is_enforced_before_the_request() {
    let stub = fixed_gemini("grounded.json").await;
    let tool = toolkit(config(&stub)).search();
    let mut context = ctx();
    context.budget = Budget {
        rows: 0,
        ..Budget::default()
    };
    let envelope = tool.call(&context, json!({"question": "q"})).await;
    assert_eq!(envelope.code(), Some(ErrorCode::BudgetExceeded));
    assert!(stub.requests().is_empty());
}

#[tokio::test]
async fn a_slow_answer_times_out_as_timeout() {
    let stub = gemini_stub(|_| Response::json("{}").with_delay(Duration::from_millis(600))).await;
    let mut config = config(&stub);
    config.search_timeout = Duration::from_millis(80);
    let envelope = toolkit(config)
        .search()
        .call(&ctx(), json!({"question": "q"}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::Timeout));
}

#[tokio::test]
async fn a_cancelled_call_is_unknown() {
    let stub = gemini_stub(|_| Response::json("{}").with_delay(Duration::from_secs(5))).await;
    let context = ctx();
    context.cancel.cancel();
    let envelope = toolkit(config(&stub))
        .search()
        .call(&context, json!({"question": "q"}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
}

#[tokio::test]
async fn bad_arguments_are_invalid_input_with_usage_and_cost_nothing() {
    let stub = fixed_gemini("grounded.json").await;
    let mut config = config(&stub);
    config.daily_cap = 1;
    let tool = toolkit(config).search();
    for args in [
        json!({}),
        json!({"question": ""}),
        json!({"question": "q", "prefer": ["podcast"]}),
        json!({"question": "q", "since": "yesterday"}),
        json!({"question": "q", "extra": true}),
    ] {
        let envelope = tool.call(&ctx(), args).await;
        assert_eq!(envelope.code(), Some(ErrorCode::InvalidInput));
        assert!(envelope.usage.is_some());
    }
    // None of those spent the daily cap.
    assert!(tool.call(&ctx(), json!({"question": "q"})).await.is_ok());
}

#[tokio::test]
async fn a_repeated_question_is_served_from_the_cache_and_says_how_old_it_is() {
    let stub = fixed_gemini("grounded.json").await;
    let dir = tempfile::tempdir().unwrap();
    let config = config(&stub).with_cache_dir(dir.path());
    let tool = toolkit(config).search();
    let first = tool
        .call(&ctx(), json!({"question": "cached question"}))
        .await;
    let second = tool
        .call(&ctx(), json!({"question": "cached question"}))
        .await;
    assert!(first.is_ok() && second.is_ok());
    assert_eq!(stub.count("/v1beta"), 1);
    assert_eq!(first.coverage.as_ref().unwrap().cached_age, None);
    assert!(second.coverage.as_ref().unwrap().cached_age.is_some());
    assert_eq!(first.data, second.data);
    assert_eq!(second.usage.unwrap().turns, 0);
    let other = tool
        .call(&ctx(), json!({"question": "a different question"}))
        .await;
    assert!(other.is_ok());
    assert_eq!(stub.count("/v1beta"), 2);
}
