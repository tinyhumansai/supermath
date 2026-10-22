//! `web_research` against a Gemini stub that answers per facet.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::json;
use supermath_tool::{Budget, ErrorCode, Outcome, Tool};
use supermath_web::{Clock, WebToolkit};

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
    assert!(
        envelope
            .unknowns
            .iter()
            .any(|u| u == "facet status: 4 findings omitted by the limit of 8 per facet"),
        "{:?}",
        envelope.unknowns
    );
}

/// A grounded Gemini answer of `sentences`, sentence `i` supported by the
/// chunks `cites(i)`, over `uris`. The URIs are not redirect hosts, so they
/// are reported as given and never requested.
fn grounded_answer(
    sentences: &[String],
    uris: &[String],
    cites: impl Fn(usize) -> Vec<usize>,
) -> Response {
    let text = sentences.join(" ");
    let mut at = 0;
    let supports: Vec<_> = sentences
        .iter()
        .enumerate()
        .map(|(i, sentence)| {
            let start = text[at..].find(sentence.as_str()).unwrap() + at;
            at = start + sentence.len();
            json!({"segment": {"startIndex": start, "endIndex": at, "text": sentence},
                   "groundingChunkIndices": cites(i)})
        })
        .collect();
    let chunks: Vec<_> = uris
        .iter()
        .map(|uri| json!({"web": {"uri": uri, "title": "t"}}))
        .collect();
    Response::json(
        &json!({"candidates": [{"content": {"parts": [{"text": text}]},
            "groundingMetadata": {"webSearchQueries": ["q"],
                "groundingChunks": chunks, "groundingSupports": supports}}]})
        .to_string(),
    )
}

const WORDS: [&str; 8] = [
    "one", "two", "three", "four", "five", "six", "seven", "eight",
];

#[tokio::test]
async fn findings_left_without_a_source_by_the_source_limit_are_counted() {
    // Eight findings, each citing five distinct pages: forty sources, of
    // which thirty are kept, so the last two findings lose every source.
    let sentences: Vec<String> = WORDS
        .iter()
        .map(|w| format!("Claim {w} is true."))
        .collect();
    let uris: Vec<String> = (0..40)
        .map(|i| format!("https://example.org/page-{i}"))
        .collect();
    let stub =
        gemini_stub(move |_| grounded_answer(&sentences, &uris, |i| (i * 5..i * 5 + 5).collect()))
            .await;
    let envelope = toolkit(config(&stub))
        .research()
        .call(&ctx(), json!({"question": "Q", "facets": ["status"]}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok, "{envelope:?}");
    assert_eq!(envelope.data["sources"].as_array().unwrap().len(), 30);
    assert_eq!(
        envelope.data["facets"]["status"]["findings"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    assert!(
        envelope
            .unknowns
            .iter()
            .any(|u| u == "2 findings omitted by the source limit"),
        "{:?}",
        envelope.unknowns
    );
}

#[tokio::test]
async fn a_facet_whose_findings_all_lost_their_sources_says_so() {
    // `status` cites 32 preprints, `approaches` one blog post; preferring
    // preprints ranks the blog post past the thirty-source limit.
    let stub = gemini_stub(|request| {
        if facet_of(&request.body) == "status" {
            let sentences: Vec<String> = WORDS
                .iter()
                .map(|w| format!("Claim {w} is true."))
                .collect();
            let uris: Vec<String> = (0..32)
                .map(|i| format!("https://arxiv.org/abs/2401.{:05}", i + 1))
                .collect();
            grounded_answer(&sentences, &uris, |i| (i * 4..i * 4 + 4).collect())
        } else {
            let sentences = vec!["A blog sketches an approach.".to_owned()];
            let uris = vec!["https://someone.wordpress.com/post".to_owned()];
            grounded_answer(&sentences, &uris, |_| vec![0])
        }
    })
    .await;
    let envelope = toolkit(config(&stub))
        .research()
        .call(
            &ctx(),
            json!({"question": "Q", "facets": ["status", "approaches"], "prefer": ["preprint"]}),
        )
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok, "{envelope:?}");
    let sources = envelope.data["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 30);
    assert!(sources.iter().all(|s| s["class"] == "preprint"));
    let approaches = &envelope.data["facets"]["approaches"];
    assert_eq!(approaches["findings"], json!([]));
    assert_eq!(approaches["note"], "findings omitted by the source limit");
    assert!(
        envelope
            .unknowns
            .iter()
            .any(|u| u == "1 finding omitted by the source limit"),
        "{:?}",
        envelope.unknowns
    );
}

#[tokio::test]
async fn the_brief_is_cut_by_whole_findings_to_stay_under_1500_words() {
    // Five facets of eight sixty-word findings: 2,400 words.
    let filler = "alpha ".repeat(57);
    let sentences: Vec<String> = WORDS
        .iter()
        .map(|w| format!("Claim {w} {filler}holds."))
        .collect();
    assert_eq!(sentences[0].split_whitespace().count(), 60);
    let uris = vec!["https://example.org/survey".to_owned()];
    let stub = gemini_stub(move |_| grounded_answer(&sentences, &uris, |_| vec![0])).await;
    let envelope = toolkit(config(&stub))
        .research()
        .call(&ctx(), json!({"question": "Q"}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok, "{envelope:?}");
    let facets = envelope.data["facets"].as_object().unwrap();
    let mut words = 0;
    for (name, facet) in facets {
        let findings = facet["findings"].as_array().unwrap();
        // Cut round by round, so every facet keeps its leading findings.
        assert_eq!(findings.len(), 5, "{name}");
        assert!(
            findings[0]["text"]
                .as_str()
                .unwrap()
                .starts_with("Claim one")
        );
        words += findings
            .iter()
            .map(|f| f["text"].as_str().unwrap().split_whitespace().count())
            .sum::<usize>();
    }
    assert_eq!(words, 1_500);
    assert!(
        envelope
            .unknowns
            .iter()
            .any(|u| u == "15 findings omitted to keep the brief under 1,500 words"),
        "{:?}",
        envelope.unknowns
    );
}

/// A clock tests move by hand.
#[derive(Debug)]
struct StepClock(AtomicU64);

impl Clock for StepClock {
    fn now_secs(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[tokio::test]
async fn a_repeated_research_question_is_served_from_the_cache_and_says_how_old_it_is() {
    let stub = gemini_stub(|r| answer(r, "grounded.json")).await;
    let dir = tempfile::tempdir().unwrap();
    let clock = Arc::new(StepClock(AtomicU64::new(1_792_677_780)));
    let toolkit =
        WebToolkit::with_clock(config(&stub).with_cache_dir(dir.path()), clock.clone()).unwrap();
    let tool = toolkit.research();
    let args = json!({"question": "cached research", "facets": ["status", "approaches"]});
    let first = tool.call(&ctx(), args.clone()).await;
    clock.0.fetch_add(90, Ordering::SeqCst);
    let second = tool.call(&ctx(), args).await;
    assert!(first.is_ok() && second.is_ok(), "{second:?}");
    assert_eq!(stub.count("/v1beta"), 2, "the second call sent no request");
    assert_eq!(first.coverage.as_ref().unwrap().cached_age, None);
    assert_eq!(second.coverage.as_ref().unwrap().cached_age, Some(90));
    assert_eq!(first.data, second.data);
    assert_eq!(second.usage.unwrap().turns, 0);
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
