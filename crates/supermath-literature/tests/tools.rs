//! The five tools through their public API, against the loopback stub.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::sync::Arc;

use serde_json::{Value, json};
use supermath_literature::{Literature, ManualClock};
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{Envelope, ErrorCode, Exposure, Outcome, Tool, ToolRegistry};
use support::{Reply, Stub, config, ctx, gzip, routes};

async fn setup() -> (Stub, Literature) {
    let stub = Stub::start(routes).await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    (stub, literature)
}

fn tool(literature: &Literature, name: &str) -> Arc<dyn Tool> {
    literature
        .tools()
        .into_iter()
        .find(|t| t.name() == name)
        .unwrap()
}

async fn call(literature: &Literature, name: &str, args: Value) -> Envelope {
    tool(literature, name).call(&ctx(), args).await
}

// --- the contract ---------------------------------------------------------

#[tokio::test]
async fn every_tool_passes_the_contract_checker() {
    let (_stub, literature) = setup().await;
    let cases: [(&str, Vec<Value>); 5] = [
        (
            "lit_search_arxiv",
            vec![
                json!({"query": "Sendov"}),
                json!({}),
                json!({"query": "ratelimited"}),
                json!({"query": "broken"}),
                json!({"query": "slow"}),
            ],
        ),
        (
            "lit_get_paper",
            vec![
                json!({"arxiv_id": "2401.00001"}),
                json!({"arxiv_id": "nonsense"}),
                json!({"arxiv_id": "2499.99999"}),
                json!({"arxiv_id": "2488.88888"}),
                json!({"arxiv_id": "2477.77777"}),
                json!({"arxiv_id": "2466.66666"}),
            ],
        ),
        (
            "lit_extract_statements",
            vec![
                json!({"arxiv_id": "2401.00001"}),
                json!({"arxiv_id": "nonsense"}),
                json!({"arxiv_id": "2499.99999"}),
                json!({"arxiv_id": "2488.88888"}),
                json!({"arxiv_id": "2477.77777"}),
                json!({"arxiv_id": "2466.66666"}),
            ],
        ),
        (
            "lit_citations",
            vec![
                json!({"arxiv_id": "2401.00001", "direction": "references"}),
                json!({"arxiv_id": "nonsense", "direction": "cited_by"}),
                json!({"arxiv_id": "2499.99999", "direction": "references"}),
                json!({"arxiv_id": "2488.88888", "direction": "references"}),
                json!({"arxiv_id": "2477.77777", "direction": "references"}),
                json!({"arxiv_id": "2466.66666", "direction": "references"}),
            ],
        ),
        (
            "lit_oeis",
            vec![
                json!({"terms": [1, 1, 2, 3, 5, 8]}),
                json!({"terms": [1, 2]}),
                json!({"query": "ratelimited"}),
                json!({"query": "broken"}),
                json!({"query": "slow"}),
            ],
        ),
    ];
    for (name, args) in cases {
        let fixtures: Vec<Fixture> = args.into_iter().map(Fixture::new).collect();
        contract::check(tool(&literature, name).as_ref(), "lit", &fixtures)
            .await
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[tokio::test]
async fn only_search_is_direct_and_every_tool_is_a_read() {
    let (_stub, literature) = setup().await;
    let mut registry = ToolRegistry::new();
    for t in literature.tools() {
        registry.register(t).unwrap();
    }
    assert_eq!(registry.len(), 5);
    for t in registry.iter() {
        let direct = t.exposure() == Exposure::Direct;
        assert_eq!(direct, t.name() == "lit_search_arxiv", "{}", t.name());
        assert_eq!(t.permission(), supermath_tool::Permission::Read);
        assert!(!t.kind().is_agentic());
    }
}

#[tokio::test]
async fn a_limit_above_fifty_is_clamped_to_fifty_not_rejected() {
    let (stub, literature) = setup().await;
    let search = call(
        &literature,
        "lit_search_arxiv",
        json!({"query": "Sendov", "limit": 500}),
    )
    .await;
    assert!(search.is_ok(), "{search:?}");
    assert_eq!(
        stub.requests()[0].query("max_results").as_deref(),
        Some("50")
    );
    let citations = call(
        &literature,
        "lit_citations",
        json!({"arxiv_id": "2401.00001", "direction": "references", "limit": 500}),
    )
    .await;
    assert!(citations.is_ok(), "{citations:?}");
    assert_eq!(stub.requests()[1].query("limit").as_deref(), Some("50"));
    let statements = call(
        &literature,
        "lit_extract_statements",
        json!({"arxiv_id": "2401.00001", "limit": 500}),
    )
    .await;
    assert!(statements.is_ok(), "{statements:?}");

    let oeis = Stub::start(oeis_pages(80)).await;
    let literature = Literature::new(&config(&oeis.base)).unwrap();
    let out = call(
        &literature,
        "lit_oeis",
        json!({"query": "many", "limit": 500}),
    )
    .await;
    assert_eq!(out.data["sequences"].as_array().unwrap().len(), 50);
    assert_eq!(out.cursor.as_deref(), Some("50"));
}

// --- lit_search_arxiv -----------------------------------------------------

#[tokio::test]
async fn search_returns_trimmed_rows_with_coverage_and_a_next_step() {
    let (stub, literature) = setup().await;
    let out = call(
        &literature,
        "lit_search_arxiv",
        json!({"query": "Sendov conjecture", "category": "math.CV", "since": "2015-01-01", "sort": "recent", "limit": 3}),
    )
    .await;
    assert_eq!(out.outcome, Outcome::Ok);
    let papers = out.data["papers"].as_array().unwrap();
    assert_eq!(papers.len(), 3);
    let first = &papers[0];
    assert_eq!(first["arxiv_id"], "2401.00001");
    assert_eq!(first["version"], 2);
    assert_eq!(
        first["title"],
        "On the Sendov conjecture for polynomials & roots"
    );
    assert_eq!(first["authors"].as_array().unwrap().len(), 3);
    assert_eq!(first["author_count"], 5);
    assert_eq!(first["primary_category"], "math.CV");
    assert_eq!(first["source"], "arxiv");
    let abstract_text = first["abstract"].as_str().unwrap();
    assert_eq!(abstract_text.chars().count(), 601);
    assert!(abstract_text.ends_with('…'));
    assert_eq!(papers[1]["arxiv_id"], "math/0301234");
    assert_eq!(
        papers[2]["abstract"],
        "Roots of <polynomials> & their derivatives."
    );

    let coverage = out.coverage.unwrap();
    assert_eq!(coverage.searched, ["arxiv"]);
    for gap in ["zbmath", "mathscinet", "journals outside arxiv", "books"] {
        assert!(coverage.not_covered.iter().any(|g| g == gap), "{gap}");
    }
    assert_eq!(out.next, [r#"lit_get_paper {"arxiv_id": "2401.00001"}"#]);
    assert_eq!(out.cursor.as_deref(), Some("3"));

    let request = &stub.requests()[0];
    let expression = request.query("search_query").unwrap();
    assert!(expression.contains("all:Sendov AND all:conjecture"));
    assert!(expression.contains("cat:math.CV"));
    assert!(expression.contains("submittedDate:[201501010000 TO"));
    assert_eq!(request.query("sortBy").as_deref(), Some("submittedDate"));
    assert_eq!(request.query("max_results").as_deref(), Some("3"));
    assert!(request.headers["user-agent"].contains("mailto:tests@example.org"));
}

#[tokio::test]
async fn search_pages_with_the_cursor() {
    let (stub, literature) = setup().await;
    let out = call(
        &literature,
        "lit_search_arxiv",
        json!({"title": "roots", "author": "Tao", "cursor": "3"}),
    )
    .await;
    assert!(out.is_ok());
    let request = &stub.requests()[0];
    assert_eq!(request.query("start").as_deref(), Some("3"));
    let expression = request.query("search_query").unwrap();
    assert!(expression.contains("ti:roots") && expression.contains("au:Tao"));
    assert_eq!(request.query("sortBy").as_deref(), Some("relevance"));
}

#[tokio::test]
async fn search_that_finds_nothing_says_only_what_it_covered() {
    let (_stub, literature) = setup().await;
    let out = call(&literature, "lit_search_arxiv", json!({"query": "nothing"})).await;
    assert_eq!(out.outcome, Outcome::Ok);
    assert_eq!(out.data["papers"].as_array().unwrap().len(), 0);
    assert!(out.next.is_empty() && out.cursor.is_none());
    assert!(out.unknowns[0].contains("says nothing about zbmath"));
    assert_eq!(out.coverage.unwrap().searched, ["arxiv"]);
}

#[tokio::test]
async fn search_rejects_bad_arguments() {
    let (stub, literature) = setup().await;
    for args in [
        json!({}),
        json!({"query": "   "}),
        json!({"query": "x", "since": "2015/01/01"}),
        json!({"query": "x", "since": "2015-13-01"}),
        json!({"query": "x", "category": "math NT"}),
        json!({"query": "x", "cursor": "abc"}),
        json!({"query": "x", "colour": "red"}),
        json!({"query": "x", "sort": "oldest"}),
    ] {
        let out = call(&literature, "lit_search_arxiv", args.clone()).await;
        assert_eq!(out.code(), Some(ErrorCode::InvalidInput), "{args}");
    }
    assert_eq!(stub.count(), 0, "invalid input never reaches the network");
}

#[tokio::test]
async fn an_arxiv_error_entry_is_invalid_input_and_a_garbled_feed_is_a_backend_failure() {
    let (_stub, literature) = setup().await;
    let bad = call(&literature, "lit_search_arxiv", json!({"query": "badq"})).await;
    assert_eq!(bad.code(), Some(ErrorCode::InvalidInput));
    let garbled = call(&literature, "lit_search_arxiv", json!({"query": "garbled"})).await;
    assert_eq!(garbled.code(), Some(ErrorCode::BackendUnavailable));
}

#[tokio::test]
async fn a_429_becomes_rate_limited_with_the_servers_retry_after() {
    let (_stub, literature) = setup().await;
    let out = call(
        &literature,
        "lit_search_arxiv",
        json!({"query": "ratelimited"}),
    )
    .await;
    assert_eq!(out.code(), Some(ErrorCode::RateLimited));
    assert_eq!(out.error.unwrap().retry_after_ms, Some(7_000));
}

#[tokio::test]
async fn a_503_is_rate_limited_and_arxiv_defaults_to_three_seconds_without_retry_after() {
    let stub = Stub::start(|_| Reply::status(503)).await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let out = call(&literature, "lit_search_arxiv", json!({"query": "x"})).await;
    assert_eq!(out.code(), Some(ErrorCode::RateLimited));
    assert_eq!(out.error.unwrap().retry_after_ms, Some(3_000));
}

#[tokio::test]
async fn a_503_with_retry_after_reports_the_servers_wait() {
    let stub = Stub::start(|_| Reply::status(503).header("Retry-After", "11")).await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let out = call(&literature, "lit_search_arxiv", json!({"query": "x"})).await;
    assert_eq!(out.code(), Some(ErrorCode::RateLimited));
    assert_eq!(out.error.unwrap().retry_after_ms, Some(11_000));
}

#[tokio::test]
async fn an_unreachable_arxiv_is_backend_unavailable() {
    let literature = Literature::new(&config("http://127.0.0.1:1")).unwrap();
    let out = call(&literature, "lit_search_arxiv", json!({"query": "x"})).await;
    assert_eq!(out.code(), Some(ErrorCode::BackendUnavailable));
}

// --- lit_get_paper --------------------------------------------------------

#[tokio::test]
async fn get_paper_returns_metadata_and_the_section_index() {
    let (stub, literature) = setup().await;
    let out = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "arXiv:2401.00001"}),
    )
    .await;
    assert_eq!(out.outcome, Outcome::Ok, "{out:?}");
    assert_eq!(out.data["arxiv_id"], "2401.00001");
    assert_eq!(out.data["version"], 2);
    assert_eq!(out.data["source"], "latex");
    assert_eq!(
        out.data["abstract"],
        "We study the roots of real polynomials."
    );
    assert_eq!(out.data["categories"], json!(["math.CV", "math.NT"]));
    let sections = out.data["sections"].as_array().unwrap();
    let numbers: Vec<_> = sections
        .iter()
        .map(|s| s["number"].as_str().unwrap())
        .collect();
    assert_eq!(numbers, ["1", "2", "A"]);
    assert_eq!(sections[1]["title"], "Main results");
    assert!(sections.iter().all(|s| s["chars"].as_u64().unwrap() > 20));
    assert_eq!(out.next.len(), 2);
    assert!(out.next[0].contains(r#""section": "1""#));
    // The e-print was fetched pinned to the version the metadata named.
    let paths: Vec<String> = stub
        .requests()
        .iter()
        .map(|r| r.path().to_owned())
        .collect();
    assert_eq!(paths, ["/api/query", "/e-print/2401.00001v2"]);
}

#[tokio::test]
async fn get_paper_returns_one_section_with_inputs_followed_and_macros_expanded() {
    let (_stub, literature) = setup().await;
    let intro = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001", "section": "1"}),
    )
    .await;
    let text = intro.data["text"].as_str().unwrap();
    assert!(text.contains("We study polynomials over $\\mathbb{R}$."));
    assert!(!text.contains("commented out"), "comments are removed");
    assert_eq!(intro.data["section"]["title"], "Introduction");
    assert!(intro.cursor.is_none());

    let main = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001v2", "section": "2"}),
    )
    .await;
    let text = main.data["text"].as_str().unwrap();
    assert!(
        text.contains("Every root is simple"),
        "main.tex is followed"
    );
    assert!(
        text.contains("Every root is rational"),
        "nested \\input is followed"
    );
    let appendix = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001", "section": "A"}),
    )
    .await;
    assert!(
        appendix.data["text"]
            .as_str()
            .unwrap()
            .contains("$\\mathbb{R}^n$")
    );
}

#[tokio::test]
async fn get_paper_pages_a_long_section_with_a_cursor() {
    let source = format!(
        "\\documentclass{{article}}\\begin{{document}}\\section{{Big}}{}\\end{{document}}",
        "ab ".repeat(10_000)
    );
    let stub = Stub::start(move |req| match req.path() {
        "/api/query" => Reply::fixture("paper_2401.00001.xml"),
        _ => Reply::ok(gzip(&source)),
    })
    .await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let args = |cursor: Option<&str>| {
        let mut a = json!({"arxiv_id": "2401.00001", "section": "1"});
        if let Some(c) = cursor {
            a["cursor"] = json!(c);
        }
        a
    };
    let first = call(&literature, "lit_get_paper", args(None)).await;
    assert_eq!(first.data["text"].as_str().unwrap().chars().count(), 12_000);
    assert_eq!(first.data["chars_total"], 29_999);
    assert_eq!(first.cursor.as_deref(), Some("12000"));
    let second = call(&literature, "lit_get_paper", args(first.cursor.as_deref())).await;
    assert_eq!(second.data["offset"], 12_000);
    assert_eq!(second.cursor.as_deref(), Some("24000"));
    let third = call(&literature, "lit_get_paper", args(second.cursor.as_deref())).await;
    assert_eq!(third.data["text"].as_str().unwrap().chars().count(), 5_999);
    assert!(third.cursor.is_none());
    let past_the_end = call(&literature, "lit_get_paper", args(Some("99999"))).await;
    assert_eq!(past_the_end.data["text"], "");
    assert!(past_the_end.cursor.is_none());
}

#[tokio::test]
async fn get_paper_reports_bad_requests() {
    let (_stub, literature) = setup().await;
    let no_section = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001", "section": "9"}),
    )
    .await;
    assert_eq!(no_section.code(), Some(ErrorCode::NotFound));
    let cursor_alone = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001", "cursor": "5"}),
    )
    .await;
    assert_eq!(cursor_alone.code(), Some(ErrorCode::InvalidInput));
    let extra = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001", "page": 2}),
    )
    .await;
    assert_eq!(extra.code(), Some(ErrorCode::InvalidInput));
    for bad in [
        "",
        "hello",
        "2401.001",
        "2401.00001v0",
        "math/12345",
        "Math.XX/0301234",
    ] {
        let out = call(&literature, "lit_get_paper", json!({"arxiv_id": bad})).await;
        assert_eq!(out.code(), Some(ErrorCode::InvalidInput), "{bad:?}");
    }
}

#[tokio::test]
async fn get_paper_lists_a_starred_section_unnumbered_and_refuses_to_return_it() {
    let source = "\\documentclass{article}\\begin{document}\\section{One}first\
                  \\section*{Acknowledgements}thanks\\section{Two}second\\end{document}";
    let stub = Stub::start(move |req| match req.path() {
        "/api/query" => Reply::fixture("paper_2401.00001.xml"),
        _ => Reply::ok(gzip(source)),
    })
    .await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let index = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001"}),
    )
    .await;
    let sections = index.data["sections"].as_array().unwrap();
    let numbers: Vec<&Value> = sections.iter().map(|s| &s["number"]).collect();
    assert_eq!(numbers, [&json!("1"), &Value::Null, &json!("2")]);
    assert_eq!(sections[1]["title"], "Acknowledgements");
    for wanted in ["Acknowledgements", "null", "*", ""] {
        let out = call(
            &literature,
            "lit_get_paper",
            json!({"arxiv_id": "2401.00001", "section": wanted}),
        )
        .await;
        assert_eq!(out.code(), Some(ErrorCode::NotFound), "{wanted:?}");
    }
    let two = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001", "section": "2"}),
    )
    .await;
    assert_eq!(
        two.data["text"], "second",
        "the starred section is not counted"
    );
}

#[tokio::test]
async fn get_paper_flags_a_pdf_only_paper_in_unknowns() {
    let (_stub, literature) = setup().await;
    let index = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2402.00002"}),
    )
    .await;
    assert_eq!(index.outcome, Outcome::Ok);
    assert_eq!(index.data["source"], "pdf");
    assert_eq!(index.data["sections"], json!([]));
    assert!(index.unknowns[0].contains("PDF only"));
    let section = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2402.00002", "section": "1"}),
    )
    .await;
    assert_eq!(section.outcome, Outcome::Unknown);
    assert_eq!(section.data["title"], "A scanned note");
}

#[tokio::test]
async fn get_paper_treats_a_missing_eprint_as_pdf_only() {
    // The metadata names 2403.00003v2 nowhere on the e-print route: a 404.
    let stub = Stub::start(|req| match req.path() {
        "/api/query" => Reply::fixture("paper_2401.00001.xml"),
        _ => Reply::status(404),
    })
    .await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let out = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001"}),
    )
    .await;
    assert_eq!(out.data["source"], "pdf");
}

#[tokio::test]
async fn get_paper_reads_a_single_gzipped_tex_and_reports_unresolved_inputs() {
    let source =
        "\\documentclass{article}\\begin{document}\\section{A}\\input{gone}text\\end{document}";
    let stub = Stub::start(move |req| match req.path() {
        "/api/query" => Reply::fixture("paper_2401.00001.xml"),
        _ => Reply::ok(gzip(source)),
    })
    .await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let out = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001", "section": "1"}),
    )
    .await;
    assert_eq!(out.data["text"], "text");
    assert!(out.unknowns[0].contains("gone"));
}

#[tokio::test]
async fn a_corrupt_eprint_is_a_backend_failure() {
    let stub = Stub::start(|req| match req.path() {
        "/api/query" => Reply::fixture("paper_2401.00001.xml"),
        _ => Reply::ok(vec![0x1f, 0x8b, 0x00, 0x01, 0x02]),
    })
    .await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let out = call(
        &literature,
        "lit_get_paper",
        json!({"arxiv_id": "2401.00001"}),
    )
    .await;
    assert_eq!(out.code(), Some(ErrorCode::BackendUnavailable));
}

// --- lit_extract_statements -----------------------------------------------

#[tokio::test]
async fn extract_statements_numbers_labels_and_links_the_paper() {
    let (_stub, literature) = setup().await;
    let out = call(
        &literature,
        "lit_extract_statements",
        json!({"arxiv_id": "2401.00001"}),
    )
    .await;
    assert_eq!(out.outcome, Outcome::Ok, "{out:?}");
    assert_eq!(
        out.data["locator"],
        json!({"arxiv_id": "2401.00001", "version": 2})
    );
    let rows = out.data["statements"].as_array().unwrap();
    let summary: Vec<(String, String, Option<String>, Option<String>)> = rows
        .iter()
        .map(|r| {
            (
                r["kind"].as_str().unwrap().to_owned(),
                r["number"].as_str().unwrap_or("-").to_owned(),
                r["label"].as_str().map(str::to_owned),
                r["section"].as_str().map(str::to_owned),
            )
        })
        .collect();
    let expected: Vec<(&str, &str, Option<&str>, Option<&str>)> = vec![
        ("definition", "1", Some("def:root"), Some("1")),
        ("theorem", "1.1", None, Some("1")),
        ("lemma", "1.2", Some("lem:small"), Some("1")),
        ("remark", "-", None, Some("1")),
        ("theorem", "2.1", Some("thm:main"), Some("2")),
        ("conjecture", "1", Some("conj:strong"), Some("2")),
        ("claim", "1", None, Some("2")),
        ("lemma", "A.1", Some("lem:app"), Some("A")),
    ];
    assert_eq!(summary.len(), expected.len(), "{summary:?}");
    for (got, want) in summary.iter().zip(&expected) {
        assert_eq!(
            (
                got.0.as_str(),
                got.1.as_str(),
                got.2.as_deref(),
                got.3.as_deref()
            ),
            *want
        );
    }
    // Refs from the statement and from its proof; the proof titled for the
    // main theorem is not the conjecture's.
    assert_eq!(rows[4]["refs"], json!(["lem:small", "def:root", "eq:key"]));
    assert_eq!(rows[5]["refs"], json!([]));
    assert_eq!(
        rows[0]["statement"],
        "A \\emph{root} of $p$ is a zero of $p$ in $\\mathbb{R}$."
    );
    // The proposition has no \newtheorem, so it is listed, not guessed.
    assert!(
        out.unknowns.iter().any(|u| u.contains("proposition")),
        "{:?}",
        out.unknowns
    );
    assert_eq!(out.data["total"], 8);
    assert!(out.cursor.is_none());
}

#[tokio::test]
async fn extract_statements_pages_its_rows() {
    let (_stub, literature) = setup().await;
    let first = call(
        &literature,
        "lit_extract_statements",
        json!({"arxiv_id": "2401.00001", "limit": 3}),
    )
    .await;
    assert_eq!(first.data["statements"].as_array().unwrap().len(), 3);
    assert_eq!(first.cursor.as_deref(), Some("3"));
    let rest = call(
        &literature,
        "lit_extract_statements",
        json!({"arxiv_id": "2401.00001", "limit": 50, "cursor": "3"}),
    )
    .await;
    assert_eq!(rest.data["statements"].as_array().unwrap().len(), 5);
    assert!(rest.cursor.is_none());
    let bad = call(
        &literature,
        "lit_extract_statements",
        json!({"arxiv_id": "2401.00001", "cursor": "x"}),
    )
    .await;
    assert_eq!(bad.code(), Some(ErrorCode::InvalidInput));
}

#[tokio::test]
async fn extract_statements_is_unknown_for_a_pdf_only_paper() {
    let (_stub, literature) = setup().await;
    let out = call(
        &literature,
        "lit_extract_statements",
        json!({"arxiv_id": "2402.00002"}),
    )
    .await;
    assert_eq!(out.outcome, Outcome::Unknown);
    assert_eq!(out.data["statements"], json!([]));
    let data = out.data.as_object().unwrap();
    assert_eq!(
        data.get("total"),
        Some(&Value::Null),
        "total is present and null: nothing was counted, which is not zero"
    );
    assert_eq!(
        out.data["locator"],
        json!({"arxiv_id": "2402.00002", "version": 1})
    );
    assert!(out.unknowns[0].contains("PDF only"));
}

#[tokio::test]
async fn extract_statements_says_so_when_a_paper_declares_none() {
    let stub = Stub::start(|req| match req.path() {
        "/api/query" => Reply::fixture("paper_2401.00001.xml"),
        _ => Reply::ok(gzip(
            "\\documentclass{article}\\begin{document}Hi.\\end{document}",
        )),
    })
    .await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let out = call(
        &literature,
        "lit_extract_statements",
        json!({"arxiv_id": "2401.00001"}),
    )
    .await;
    assert_eq!(out.outcome, Outcome::Ok);
    assert_eq!(out.data["total"], 0);
    assert!(out.unknowns[0].contains("no statements"));
}

// --- lit_citations --------------------------------------------------------

#[tokio::test]
async fn citations_lists_references_with_arxiv_ids() {
    let (stub, literature) = setup().await;
    let out = call(
        &literature,
        "lit_citations",
        json!({"arxiv_id": "2401.00001v2", "direction": "references", "limit": 3}),
    )
    .await;
    assert_eq!(out.outcome, Outcome::Ok, "{out:?}");
    let papers = out.data["papers"].as_array().unwrap();
    assert_eq!(papers.len(), 2, "a null citedPaper is dropped");
    assert_eq!(papers[0]["arxiv_id"], "math/0301234");
    assert_eq!(papers[0]["doi"], "10.1000/xyz");
    assert_eq!(papers[0]["authors"].as_array().unwrap().len(), 3);
    assert_eq!(papers[0]["author_count"], 4);
    assert_eq!(papers[0]["citation_count"], 42);
    assert_eq!(papers[0]["source"], "semantic_scholar");
    assert_eq!(papers[1]["arxiv_id"], Value::Null);
    assert_eq!(papers[1]["venue"], Value::Null);
    assert_eq!(out.cursor.as_deref(), Some("3"));
    assert_eq!(out.coverage.unwrap().searched, ["semantic_scholar"]);
    assert!(out.unknowns[0].contains("incomplete"));
    assert_eq!(out.next, [r#"lit_get_paper {"arxiv_id": "math/0301234"}"#]);
    let request = &stub.requests()[0];
    assert_eq!(
        request.path(),
        "/graph/v1/paper/arXiv:2401.00001/references"
    );
    assert_eq!(request.query("limit").as_deref(), Some("3"));
    assert!(!request.headers.contains_key("x-api-key"));
}

#[tokio::test]
async fn citations_cited_by_reads_the_citing_side_and_sends_the_api_key() {
    let stub = Stub::start(routes).await;
    let mut cfg = config(&stub.base);
    cfg.scholar_api_key = Some("secret-key".into());
    let literature = Literature::new(&cfg).unwrap();
    let out = call(
        &literature,
        "lit_citations",
        json!({"arxiv_id": "2401.00001", "direction": "cited_by", "cursor": "10"}),
    )
    .await;
    assert_eq!(out.data["direction"], "cited_by");
    assert_eq!(out.data["papers"][0]["title"], "Building on Sendov");
    assert!(out.cursor.is_none());
    let request = &stub.requests()[0];
    assert!(request.path().ends_with("/citations"));
    assert_eq!(request.query("offset").as_deref(), Some("10"));
    assert_eq!(request.headers["x-api-key"], "secret-key");
}

#[tokio::test]
async fn citations_maps_a_bare_429_to_a_default_retry_and_rejects_bad_arguments() {
    let (_stub, literature) = setup().await;
    let limited = call(
        &literature,
        "lit_citations",
        json!({"arxiv_id": "2488.88888", "direction": "references"}),
    )
    .await;
    assert_eq!(limited.error.unwrap().retry_after_ms, Some(1_000));
    for args in [
        json!({"arxiv_id": "2401.00001"}),
        json!({"arxiv_id": "2401.00001", "direction": "sideways"}),
        json!({"direction": "references"}),
        json!({"arxiv_id": "2401.00001", "direction": "references", "cursor": "?"}),
    ] {
        let out = call(&literature, "lit_citations", args.clone()).await;
        assert_eq!(out.code(), Some(ErrorCode::InvalidInput), "{args}");
    }
}

#[tokio::test]
async fn citations_suggest_no_next_step_when_no_row_has_an_arxiv_id() {
    let stub = Stub::start(|_| {
        Reply::ok(
            json!({"data": [
                {"citedPaper": {"paperId": "a", "title": "Journal only", "externalIds": {"DOI": "10.1/a"}}},
                {"citedPaper": {"paperId": "b", "title": "No ids"}}
            ]})
            .to_string(),
        )
    })
    .await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let out = call(
        &literature,
        "lit_citations",
        json!({"arxiv_id": "2401.00001", "direction": "references"}),
    )
    .await;
    assert_eq!(out.outcome, Outcome::Ok, "{out:?}");
    assert_eq!(out.data["papers"].as_array().unwrap().len(), 2);
    assert!(out.next.is_empty(), "{:?}", out.next);
}

#[tokio::test]
async fn citations_suggest_reading_a_versioned_arxiv_row_without_its_version() {
    let stub = Stub::start(|_| {
        Reply::ok(
            json!({"data": [
                {"citingPaper": {"paperId": "c", "externalIds": {"ArXiv": "2311.04567v3"}}}
            ]})
            .to_string(),
        )
    })
    .await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let out = call(
        &literature,
        "lit_citations",
        json!({"arxiv_id": "2401.00001", "direction": "cited_by"}),
    )
    .await;
    assert_eq!(out.data["papers"][0]["arxiv_id"], "2311.04567");
    assert_eq!(out.next, [r#"lit_get_paper {"arxiv_id": "2311.04567"}"#]);
}

#[tokio::test]
async fn a_garbled_scholar_answer_is_a_backend_failure() {
    let stub = Stub::start(|_| Reply::ok("[1, 2")).await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let out = call(
        &literature,
        "lit_citations",
        json!({"arxiv_id": "2401.00001", "direction": "references"}),
    )
    .await;
    assert_eq!(out.code(), Some(ErrorCode::BackendUnavailable));
}

// --- lit_oeis -------------------------------------------------------------

#[tokio::test]
async fn oeis_by_terms_caps_terms_formulas_and_reports_the_subsequence_caveat() {
    let (stub, literature) = setup().await;
    let out = call(
        &literature,
        "lit_oeis",
        json!({"terms": [1, 1, 2, 3, 5, 8]}),
    )
    .await;
    assert_eq!(out.outcome, Outcome::Ok, "{out:?}");
    let sequences = out.data["sequences"].as_array().unwrap();
    assert_eq!(sequences[0]["id"], "A000045");
    assert_eq!(sequences[0]["terms"].as_array().unwrap().len(), 30);
    assert_eq!(sequences[0]["terms"][2], 1);
    assert_eq!(sequences[0]["formula"].as_array().unwrap().len(), 5);
    assert_eq!(
        sequences[0]["references"],
        json!(["Lecture notes, Fibonacci Quarterly."])
    );
    assert_eq!(sequences[0]["source"], "oeis");
    // A term too large for a JSON integer stays exact as a string.
    assert_eq!(sequences[1]["terms"][5], "99999999999999999999999999");
    assert_eq!(out.data["total"], 2);
    assert!(out.unknowns[0].contains("lead, not an identification"));
    let coverage = out.coverage.unwrap();
    assert_eq!(coverage.searched, ["oeis"]);
    assert_eq!(coverage.not_covered, ["sequences not in the oeis"]);
    assert_eq!(
        stub.requests()[0].query("q").as_deref(),
        Some("1,1,2,3,5,8")
    );
    assert_eq!(stub.requests()[0].query("fmt").as_deref(), Some("json"));
}

#[tokio::test]
async fn oeis_by_query_pages_and_accepts_the_legacy_array_shape() {
    let (stub, literature) = setup().await;
    let page = call(
        &literature,
        "lit_oeis",
        json!({"query": "fibonacci", "limit": 1}),
    )
    .await;
    assert_eq!(page.data["sequences"].as_array().unwrap().len(), 1);
    assert_eq!(page.cursor.as_deref(), Some("1"));
    assert!(
        page.unknowns.is_empty(),
        "no subsequence caveat for a text query"
    );
    let next = call(
        &literature,
        "lit_oeis",
        json!({"query": "fibonacci", "limit": 1, "cursor": "1"}),
    )
    .await;
    assert_eq!(
        next.data["sequences"][0]["id"], "A000045",
        "the stub ignores start"
    );
    assert_eq!(stub.requests()[1].query("start").as_deref(), Some("1"));
    let legacy = call(&literature, "lit_oeis", json!({"query": "legacy"})).await;
    assert_eq!(legacy.data["sequences"][0]["id"], "A000040");
    assert_eq!(legacy.data["total"], Value::Null);
    assert!(legacy.cursor.is_none());
}

/// An OEIS stub holding `total` sequences A000001.., serving ten per page
/// from the `start` it is asked for, as the real search does.
fn oeis_pages(total: u64) -> impl Fn(&support::Recorded) -> Reply {
    move |req| {
        let start: u64 = req.query("start").unwrap().parse().unwrap();
        let results: Vec<Value> = (start..total.min(start + 10))
            .map(|i| json!({"number": i + 1, "name": format!("sequence {}", i + 1), "data": "1,2,3"}))
            .collect();
        Reply::ok(json!({"count": total, "start": start, "results": results}).to_string())
    }
}

#[tokio::test]
async fn oeis_fetches_further_pages_to_honour_a_limit_above_ten() {
    let stub = Stub::start(oeis_pages(25)).await;
    let literature = Literature::new(&config(&stub.base)).unwrap();
    let out = call(
        &literature,
        "lit_oeis",
        json!({"query": "many", "limit": 15}),
    )
    .await;
    assert_eq!(out.outcome, Outcome::Ok, "{out:?}");
    let ids: Vec<&str> = out.data["sequences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 15);
    assert_eq!(ids[0], "A000001");
    assert_eq!(ids[14], "A000015");
    assert_eq!(out.data["total"], 25);
    assert_eq!(out.cursor.as_deref(), Some("15"));
    let starts: Vec<String> = stub
        .requests()
        .iter()
        .map(|r| r.query("start").unwrap())
        .collect();
    assert_eq!(starts, ["0", "10"]);

    let rest = call(
        &literature,
        "lit_oeis",
        json!({"query": "many", "limit": 50, "cursor": "15"}),
    )
    .await;
    let ids: Vec<&str> = rest.data["sequences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.first(), Some(&"A000016"));
    assert_eq!(ids.last(), Some(&"A000025"));
    assert_eq!(ids.len(), 10);
    assert!(rest.cursor.is_none(), "every match was shown");
    assert_eq!(
        stub.count(),
        3,
        "no page is fetched past the count OEIS reported"
    );
}

#[tokio::test]
async fn oeis_rate_limit_carries_the_servers_retry_after() {
    let (_stub, literature) = setup().await;
    let out = call(&literature, "lit_oeis", json!({"query": "ratelimited"})).await;
    assert_eq!(out.code(), Some(ErrorCode::RateLimited));
    assert_eq!(out.error.unwrap().retry_after_ms, Some(2_000));
}

#[tokio::test]
async fn oeis_with_no_match_says_so_for_the_oeis_only() {
    let (_stub, literature) = setup().await;
    let out = call(&literature, "lit_oeis", json!({"query": "nothing"})).await;
    assert_eq!(out.outcome, Outcome::Ok);
    assert_eq!(out.data["sequences"].as_array().unwrap().len(), 0);
    assert!(out.unknowns[0].contains("does not hold"));
}

#[tokio::test]
async fn oeis_rejects_bad_arguments_and_garbled_answers() {
    let (stub, literature) = setup().await;
    for args in [
        json!({}),
        json!({"terms": [1, 2, 3]}),
        json!({"terms": [1, 2, 3, 4], "query": "x"}),
        json!({"query": "  "}),
        json!({"terms": vec![1; 61]}),
        json!({"terms": [1, 2, 3, "four"]}),
        json!({"query": "x", "cursor": "-"}),
    ] {
        let out = call(&literature, "lit_oeis", args.clone()).await;
        assert_eq!(out.code(), Some(ErrorCode::InvalidInput), "{args}");
    }
    assert_eq!(stub.count(), 0);
    let garbled = call(&literature, "lit_oeis", json!({"query": "garbled"})).await;
    assert_eq!(garbled.code(), Some(ErrorCode::BackendUnavailable));
}

// --- politeness: the rate limiter and the cache ----------------------------

#[tokio::test]
async fn consecutive_arxiv_requests_wait_for_the_rate_limit_on_a_fake_clock() {
    let stub = Stub::start(routes).await;
    let clock = Arc::new(ManualClock::new(1_000_000));
    let mut cfg = config(&stub.base);
    cfg.arxiv_interval_ms = 3_000;
    let literature = Literature::with_clock(&cfg, clock.clone()).unwrap();
    for query in ["one", "two", "three"] {
        let out = call(&literature, "lit_search_arxiv", json!({"query": query})).await;
        assert!(out.is_ok());
    }
    assert_eq!(clock.sleeps(), vec![3_000, 3_000]);
    // Semantic Scholar and OEIS are not arXiv: no waiting.
    call(&literature, "lit_oeis", json!({"query": "fibonacci"})).await;
    call(
        &literature,
        "lit_citations",
        json!({"arxiv_id": "2401.00001", "direction": "references"}),
    )
    .await;
    assert_eq!(clock.sleeps().len(), 2);
}

#[tokio::test]
async fn responses_are_cached_on_disk_a_day_for_searches_and_forever_for_a_version() {
    let stub = Stub::start(routes).await;
    let dir = tempfile::tempdir().unwrap();
    let clock = Arc::new(ManualClock::new(5_000_000_000));
    let mut cfg = config(&stub.base);
    cfg.cache_dir = Some(dir.path().join("literature"));
    let literature = Literature::with_clock(&cfg, clock.clone()).unwrap();

    let search = json!({"query": "Sendov"});
    call(&literature, "lit_search_arxiv", search.clone()).await;
    clock.advance(90_000); // 90 s
    let again = call(&literature, "lit_search_arxiv", search.clone()).await;
    assert_eq!(stub.count(), 1, "the second search came from the cache");
    assert_eq!(again.coverage.unwrap().cached_age, Some(90));
    clock.advance(25 * 60 * 60 * 1000);
    call(&literature, "lit_search_arxiv", search).await;
    assert_eq!(stub.count(), 2, "a search older than a day is refetched");

    // An unversioned id resolves to "latest" and expires; the pinned e-print
    // of a version does not.
    let paper = json!({"arxiv_id": "2401.00001"});
    call(&literature, "lit_get_paper", paper.clone()).await;
    assert_eq!(stub.count(), 4);
    clock.advance(400 * 24 * 60 * 60 * 1000);
    let again = call(&literature, "lit_get_paper", paper.clone()).await;
    assert_eq!(stub.count(), 5, "only the metadata was refetched");
    assert_eq!(again.coverage.unwrap().cached_age, Some(400 * 24 * 60 * 60));
    let versioned = json!({"arxiv_id": "2401.00001v2"});
    call(&literature, "lit_get_paper", versioned.clone()).await;
    assert_eq!(stub.count(), 6, "a versioned id's metadata is fetched once");
    clock.advance(400 * 24 * 60 * 60 * 1000);
    call(&literature, "lit_get_paper", versioned).await;
    assert_eq!(stub.count(), 6, "and then kept forever");
}

#[tokio::test]
async fn a_failed_request_is_never_cached() {
    let stub = Stub::start(routes).await;
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = config(&stub.base);
    cfg.cache_dir = Some(dir.path().to_path_buf());
    let literature = Literature::new(&cfg).unwrap();
    for _ in 0..2 {
        let out = call(&literature, "lit_search_arxiv", json!({"query": "broken"})).await;
        assert_eq!(out.code(), Some(ErrorCode::BackendUnavailable));
    }
    assert_eq!(stub.count(), 2);
}
