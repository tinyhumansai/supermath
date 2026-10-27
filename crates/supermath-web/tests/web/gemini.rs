//! The Gemini client on its own.

use std::sync::Arc;

use supermath_web::{Error, GeminiClient, SystemClock};

use crate::support::{KEY, Response, config, fixed_gemini, gemini_stub};

#[tokio::test]
async fn debug_output_redacts_the_key() {
    let stub = fixed_gemini("grounded.json").await;
    let client = GeminiClient::new(&config(&stub), Arc::new(SystemClock)).unwrap();
    assert!(client.has_api_key());
    assert_eq!(client.model(), supermath_web::DEFAULT_MODEL);
    let shown = format!("{client:?}");
    assert!(!shown.contains(KEY));
    assert!(shown.contains("<redacted>"));
}

#[tokio::test]
async fn generating_without_a_key_fails_before_any_request() {
    let stub = fixed_gemini("grounded.json").await;
    let config = supermath_web::WebConfig::default().with_base_url(stub.base.clone());
    let client = GeminiClient::new(&config, Arc::new(SystemClock)).unwrap();
    assert!(!client.has_api_key());
    assert_eq!(
        client.generate("hi").await.unwrap_err(),
        Error::MissingApiKey
    );
    assert!(stub.requests().is_empty());
}

#[tokio::test]
async fn a_404_says_the_model_may_be_retired_and_names_the_override() {
    let stub = gemini_stub(|_| Response::status(404)).await;
    let mut config = config(&stub);
    config.model = "gemini-0-retired".to_owned();
    let client = GeminiClient::new(&config, Arc::new(SystemClock)).unwrap();
    let message = client.generate("hi").await.unwrap_err().to_string();
    assert!(message.contains("gemini-0-retired"), "{message}");
    assert!(message.contains("retired"), "{message}");
    assert!(message.contains("SUPERMATH_WEB_MODEL"), "{message}");
}

#[tokio::test]
async fn a_402_says_the_key_is_out_of_credit_and_is_not_a_transient_outage() {
    let stub = gemini_stub(|_| Response::status(402)).await;
    let client = GeminiClient::new(&config(&stub), Arc::new(SystemClock)).unwrap();
    let error = client.generate("hi").await.unwrap_err();
    assert_eq!(error, Error::BillingExhausted);
    assert_eq!(error.code(), supermath_tool::ErrorCode::BudgetExceeded);
    assert!(error.to_string().contains("credit"), "{error}");
}

#[tokio::test]
async fn a_probe_sends_one_ungrounded_token_request_and_reports_a_402() {
    let stub = gemini_stub(|_| Response::status(402)).await;
    let client = GeminiClient::new(&config(&stub), Arc::new(SystemClock)).unwrap();
    assert_eq!(client.probe().await.unwrap_err(), Error::BillingExhausted);
    let requests = stub.requests();
    assert_eq!(requests.len(), 1);
    let body = &requests[0].body;
    assert!(!body.contains("google_search"), "{body}");
    assert!(body.contains("\"maxOutputTokens\":1"), "{body}");
}

#[tokio::test]
async fn a_probe_of_a_funded_key_succeeds() {
    let stub = fixed_gemini("grounded.json").await;
    let client = GeminiClient::new(&config(&stub), Arc::new(SystemClock)).unwrap();
    client.probe().await.unwrap();
}

#[tokio::test]
async fn generate_returns_the_decoded_response() {
    let stub = fixed_gemini("grounded.json").await;
    let client = GeminiClient::new(&config(&stub), Arc::new(SystemClock)).unwrap();
    let generation = client.generate("hi").await.unwrap();
    assert_eq!(generation.cached_age, None);
    let candidate = &generation.response.candidates[0];
    let meta = candidate.grounding_metadata.as_ref().unwrap();
    assert_eq!(meta.grounding_chunks.len(), 2);
    assert_eq!(meta.web_search_queries.len(), 2);
}
