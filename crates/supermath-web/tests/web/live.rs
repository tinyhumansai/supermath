//! A live search against the real Gemini API.

use serde_json::json;
use supermath_tool::Tool;
use supermath_web::{WebConfig, WebToolkit};

use crate::support::ctx;

/// Runs one grounded search and asserts at least one resolved source.
///
/// Skips unless `GEMINI_API_KEY` and `SUPERMATH_LIVE_WEB` are both set.
#[tokio::test]
async fn live_web_search() {
    let live = std::env::var("SUPERMATH_LIVE_WEB").is_ok_and(|v| !v.is_empty());
    let config = WebConfig::from_env().unwrap();
    if !live || !config.has_api_key() {
        eprintln!("skipping live_web_search: set GEMINI_API_KEY and SUPERMATH_LIVE_WEB");
        return;
    }
    let tool = WebToolkit::new(config).unwrap().search();
    let envelope = tool
        .call(
            &ctx(),
            json!({"question": "Who proved the sunflower lemma bound (log k)^k in 2019?", "limit": 5}),
        )
        .await;
    assert!(envelope.is_ok(), "{envelope:?}");
    let sources = envelope.data["sources"].as_array().unwrap();
    assert!(!sources.is_empty(), "no resolved source");
    for source in sources {
        let url = source["url"].as_str().unwrap();
        assert!(url.starts_with("http"));
        assert!(
            !url.contains("vertexaisearch.cloud.google.com"),
            "unresolved redirect {url}"
        );
    }
}
