//! A live search against the real arXiv, skipped unless asked for.
//!
//! Run it with `SUPERMATH_LIVE_ARXIV=1 cargo test -p supermath-literature
//! --test live`. It searches arXiv once and honours its rate limit.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use serde_json::json;
use supermath_literature::{Literature, LiteratureConfig};

#[tokio::test]
async fn live_literature_arxiv() {
    if std::env::var_os("SUPERMATH_LIVE_ARXIV").is_none() {
        eprintln!("skipped: set SUPERMATH_LIVE_ARXIV to search arXiv");
        return;
    }
    let literature = Literature::new(&LiteratureConfig::from_env()).unwrap();
    let tools = literature.tools();
    let search = tools
        .iter()
        .find(|t| t.name() == "lit_search_arxiv")
        .unwrap();
    let envelope = search
        .call(
            &support::ctx(),
            json!({ "query": "Sendov conjecture", "limit": 3 }),
        )
        .await;
    assert!(envelope.is_ok(), "{envelope:?}");
    let papers = envelope.data["papers"].as_array().unwrap();
    assert_ne!(papers.len(), 0);
    assert!(papers[0]["arxiv_id"].is_string());
}
