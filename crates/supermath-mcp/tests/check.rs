//! `check` against loopback stubs: each backend reported, secrets never.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use supermath_mcp::check::{Status, run};
use support::{Env, LeanService, Reply, Stub, cortex};

const EMPTY_FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xmlns:opensearch="http://a9.com/-/spec/opensearch/1.1/">
<opensearch:totalResults>0</opensearch:totalResults>
<opensearch:startIndex>0</opensearch:startIndex>
<opensearch:itemsPerPage>1</opensearch:itemsPerPage>
</feed>"#;

async fn arxiv() -> Stub {
    Stub::start(|_| Reply::atom(EMPTY_FEED)).await
}

async fn python() -> Stub {
    Stub::start(|r| match r.path.as_str() {
        "/health" => Reply::json(
            &serde_json::json!({"status": "ok", "python": "3.13.1", "lock_sha256": "abc123"}),
        ),
        _ => Reply::status(404),
    })
    .await
}

fn line<'a>(
    report: &'a supermath_mcp::check::Report,
    name: &str,
) -> &'a supermath_mcp::check::Line {
    report.lines.iter().find(|l| l.name == name).unwrap()
}

#[tokio::test]
async fn every_reachable_backend_is_reported_with_what_it_found() {
    let lean = LeanService::start().await;
    let python = python().await;
    let arxiv = arxiv().await;
    let cortex = cortex().await;
    let env = Env::new()
        .set("LEAN_URL", &lean.stub.base)
        .set("PYTHON_URL", &python.base)
        .set("SUPERMATH_ARXIV_API_URL", &arxiv.base)
        .set("CORTEX_API_URL", cortex.url())
        .set("CORTEX_API_KEY", "c-secret")
        .set("GEMINI_API_KEY", "g-secret");
    let config = env.config();
    let report = run(&env.toolkit(), &config).await;
    assert!(report.passed(), "{report}");
    assert!(
        line(&report, "lean")
            .detail
            .contains("leanprover/lean4:v4.20.0")
    );
    assert!(line(&report, "lean").detail.contains("4f2c9e1ab07d"));
    assert!(line(&report, "python").detail.contains("3.13.1"));
    assert!(line(&report, "python").detail.contains("abc123"));
    assert_eq!(line(&report, "gemini").status, Status::Ok);
    assert_eq!(line(&report, "arxiv").status, Status::Ok);
    let shown = report.to_string();
    assert!(
        !shown.contains("g-secret") && !shown.contains("c-secret"),
        "{shown}"
    );
}

#[tokio::test]
async fn unset_backends_are_skipped_and_do_not_fail_the_check() {
    let arxiv = arxiv().await;
    let env = Env::new().set("SUPERMATH_ARXIV_API_URL", &arxiv.base);
    let report = run(&env.toolkit(), &env.config()).await;
    assert!(report.passed(), "{report}");
    for name in ["lean", "python", "cortexdb", "gemini"] {
        assert_eq!(line(&report, name).status, Status::Skipped, "{name}");
    }
}

#[tokio::test]
async fn a_configured_backend_that_does_not_answer_fails_the_check() {
    let arxiv = Stub::start(|_| Reply::status(500)).await;
    let env = Env::new()
        .set("LEAN_URL", "http://127.0.0.1:1")
        .set("PYTHON_URL", "http://127.0.0.1:1")
        .set("CORTEX_API_URL", "http://127.0.0.1:1")
        .set("CORTEX_API_KEY", "k")
        .set("SUPERMATH_ARXIV_API_URL", &arxiv.base);
    let report = run(&env.toolkit(), &env.config()).await;
    assert!(!report.passed());
    for name in ["lean", "python", "cortexdb", "arxiv"] {
        assert_eq!(
            line(&report, name).status,
            Status::Failed,
            "{name}: {report}"
        );
    }
}
