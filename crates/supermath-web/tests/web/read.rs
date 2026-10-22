//! `web_read` against a loopback page server.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use supermath_tool::{ErrorCode, Outcome, Tool};
use supermath_web::WebToolkit;

use crate::support::{FixedClock, Request, Response, Stub, config, ctx, fixture, toolkit};

/// Serves `/robots.txt` from `robots`, then `pages` for everything else.
async fn site(
    robots: Option<&'static str>,
    pages: impl Fn(&Request) -> Response + Send + Sync + 'static,
) -> Stub {
    Stub::start(move |request| {
        if request.path == "/robots.txt" {
            match robots {
                Some(body) => Response::text(body),
                None => Response::status(404),
            }
        } else {
            pages(request)
        }
    })
    .await
}

async fn read(stub: &Stub, path: &str) -> supermath_tool::Envelope {
    let cfg = crate::support::config(&crate::support::Stub::start(|_| Response::status(404)).await);
    let mut cfg = cfg;
    cfg.contact = Some("ops@example.com".to_owned());
    toolkit(cfg)
        .read()
        .call(&ctx(), json!({"url": format!("{}{path}", stub.base)}))
        .await
}

#[tokio::test]
async fn reads_the_main_text_as_untrusted_data_with_math_as_tex() {
    let stub = site(None, |_| Response::html(&fixture("article.html", ""))).await;
    let gemini = Stub::start(|_| Response::status(404)).await;
    let toolkit =
        WebToolkit::with_clock(config(&gemini), Arc::new(FixedClock(1_792_677_780))).unwrap();
    let envelope = toolkit
        .read()
        .call(&ctx(), json!({"url": format!("{}/page", stub.base)}))
        .await;

    assert_eq!(envelope.outcome, Outcome::Ok, "{envelope:?}");
    assert_eq!(envelope.usage, None);
    let data = &envelope.data;
    assert_eq!(data["untrusted"], true);
    assert_eq!(data["title"], "Sunflowers and the Erdős–Rado conjecture");
    assert_eq!(data["class"], "other");
    assert_eq!(data["retrieved_at"], "2026-10-22T14:03:00Z");
    assert_eq!(data["math"], "tex");
    assert_eq!(data["url"], format!("{}/page", stub.base));
    let text = data["text"].as_str().unwrap();
    assert!(text.contains("$(\\log k)^{k}$"), "{text}");
    assert!(text.contains("# Sunflowers and the Erdős–Rado conjecture"));
    assert!(text.contains("- Alweiss\n- Lovett"));
    assert!(
        text.contains("IGNORE ALL PREVIOUS INSTRUCTIONS"),
        "page text is data and stays"
    );
    for gone in ["INJECTED SCRIPT", "Hidden text", "Archive", "Copyright"] {
        assert!(!text.contains(gone), "{gone}");
    }
    assert_eq!(envelope.cursor, None);
}

#[tokio::test]
async fn the_fetch_names_supermath_and_the_contact() {
    let stub = site(None, |_| Response::html("<p>hi</p>")).await;
    let envelope = read(&stub, "/").await;
    assert!(envelope.is_ok());
    let requests = stub.requests();
    let agent = &requests.iter().find(|r| r.path == "/").unwrap().headers["user-agent"];
    assert!(
        agent.starts_with("supermath/") && agent.contains("ops@example.com"),
        "{agent}"
    );
    assert!(requests.iter().any(|r| r.path == "/robots.txt"));
}

#[tokio::test]
async fn long_pages_are_paged_with_a_cursor() {
    let paragraph = format!("<p>{}</p>", "sunflower ".repeat(200));
    let html = format!("<html><body>{}</body></html>", paragraph.repeat(20));
    let stub = site(None, move |_| Response::html(&html)).await;
    let gemini = Stub::start(|_| Response::status(404)).await;
    let tool = toolkit(config(&gemini)).read();
    let url = format!("{}/long", stub.base);

    let first = tool.call(&ctx(), json!({"url": url})).await;
    assert_eq!(first.cursor.as_deref(), Some("c2"));
    let mut text = first.data["text"].as_str().unwrap().to_owned();
    assert!(text.chars().count() <= 12_000);

    let mut cursor = first.cursor.clone();
    while let Some(c) = cursor {
        let page = tool.call(&ctx(), json!({"url": url, "cursor": c})).await;
        assert!(page.is_ok());
        text.push_str(page.data["text"].as_str().unwrap());
        cursor = page.cursor.clone();
    }
    assert_eq!(text.matches("sunflower").count(), 4000);

    let past = tool
        .call(&ctx(), json!({"url": url, "cursor": "c99"}))
        .await;
    assert_eq!(past.code(), Some(ErrorCode::InvalidInput));
    let bad = tool
        .call(&ctx(), json!({"url": url, "cursor": "next"}))
        .await;
    assert_eq!(bad.code(), Some(ErrorCode::InvalidInput));
}

#[tokio::test]
async fn plain_text_is_returned_as_is_and_stack_exchange_answers_by_score() {
    let stub = site(None, |request| {
        if request.path == "/mo" {
            Response::html(&fixture("mathoverflow.html", ""))
        } else {
            Response::text("  just text  ")
        }
    })
    .await;
    let plain = read(&stub, "/notes.txt").await;
    assert_eq!(plain.data["text"], "just text");
    assert_eq!(plain.data["math"], "none");

    let qa = read(&stub, "/mo").await;
    let text = qa.data["text"].as_str().unwrap();
    assert!(text.find("by ALWZ").unwrap() < text.find("small k").unwrap());
    assert!(text.contains("## Answer (score 25, accepted)"));
    assert_eq!(
        qa.data["title"],
        "Is every sunflower-free family small? - MathOverflow"
    );
}

#[tokio::test]
async fn follows_redirects_and_reports_the_final_url() {
    let stub = site(None, |request| match request.path.as_str() {
        "/old" => Response::status(301).with_header("location", "/new"),
        "/new" => Response::html("<p>moved</p>"),
        "/loop" => Response::status(302).with_header("location", "/loop"),
        "/nowhere" => Response::status(302),
        _ => Response::status(404),
    })
    .await;
    let moved = read(&stub, "/old").await;
    assert_eq!(moved.data["url"], format!("{}/new", stub.base));
    assert_eq!(
        read(&stub, "/loop").await.code(),
        Some(ErrorCode::BackendUnavailable)
    );
    assert_eq!(
        read(&stub, "/nowhere").await.code(),
        Some(ErrorCode::BackendUnavailable)
    );
}

#[tokio::test]
async fn http_errors_map_to_their_codes() {
    let stub = site(None, |request| match request.path.as_str() {
        "/gone" => Response::status(410),
        "/forbidden" => Response::status(403),
        "/login" => Response::status(401),
        "/busy" => Response::status(429).with_header("retry-after", "12"),
        "/busy-no-header" => Response::status(429),
        "/boom" => Response::status(502),
        _ => Response::status(404),
    })
    .await;
    assert_eq!(
        read(&stub, "/missing").await.code(),
        Some(ErrorCode::NotFound)
    );
    assert_eq!(read(&stub, "/gone").await.code(), Some(ErrorCode::NotFound));
    assert_eq!(
        read(&stub, "/forbidden").await.code(),
        Some(ErrorCode::Forbidden)
    );
    assert_eq!(
        read(&stub, "/login").await.code(),
        Some(ErrorCode::Forbidden)
    );
    let busy = read(&stub, "/busy").await;
    assert_eq!(busy.code(), Some(ErrorCode::RateLimited));
    assert_eq!(busy.error.unwrap().retry_after_ms, Some(12_000));
    assert_eq!(
        read(&stub, "/busy-no-header")
            .await
            .error
            .unwrap()
            .retry_after_ms,
        Some(30_000)
    );
    assert_eq!(
        read(&stub, "/boom").await.code(),
        Some(ErrorCode::BackendUnavailable)
    );
}

#[tokio::test]
async fn pdfs_other_types_and_oversized_pages_are_invalid_input() {
    let stub = site(None, |request| match request.path.as_str() {
        "/paper.pdf" => Response::status(200)
            .with_header("content-type", "application/pdf")
            .with_body(b"%PDF-1.7"),
        "/image.png" => Response::status(200)
            .with_header("content-type", "image/png")
            .with_body(b"\x89PNG"),
        "/huge-declared" => Response::html("x").with_header("x-note", "declared"),
        "/huge-body" => Response::html(&"a".repeat(5 * 1024 * 1024 + 1)),
        _ => Response::status(404),
    })
    .await;
    let pdf = read(&stub, "/paper.pdf").await;
    assert_eq!(pdf.code(), Some(ErrorCode::InvalidInput));
    assert!(pdf.error.unwrap().message.contains("PDF"));
    assert_eq!(
        read(&stub, "/image.png").await.code(),
        Some(ErrorCode::InvalidInput)
    );
    assert_eq!(
        read(&stub, "/huge-body").await.code(),
        Some(ErrorCode::InvalidInput)
    );
}

#[tokio::test]
async fn robots_txt_is_honoured() {
    let stub = site(Some("User-agent: *\nDisallow: /private\n"), |_| {
        Response::html("<p>ok</p>")
    })
    .await;
    assert!(read(&stub, "/public").await.is_ok());
    assert_eq!(
        read(&stub, "/private/page").await.code(),
        Some(ErrorCode::Forbidden)
    );
    // The disallowed page itself was never requested.
    assert_eq!(stub.count("/private"), 0);

    let broken = Stub::start(|request| {
        if request.path == "/robots.txt" {
            Response::status(500)
        } else {
            Response::html("<p>ok</p>")
        }
    })
    .await;
    assert_eq!(read(&broken, "/").await.code(), Some(ErrorCode::Forbidden));
}

#[tokio::test]
async fn a_slow_page_times_out() {
    let stub = site(None, |_| {
        Response::html("<p>slow</p>").with_delay(Duration::from_secs(2))
    })
    .await;
    let gemini = Stub::start(|_| Response::status(404)).await;
    let mut cfg = config(&gemini);
    cfg.read_timeout = Duration::from_millis(100);
    let envelope = toolkit(cfg)
        .read()
        .call(&ctx(), json!({"url": format!("{}/", stub.base)}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::Timeout));

    let context = ctx();
    context.cancel.cancel();
    let cancelled = toolkit(config(&gemini))
        .read()
        .call(&context, json!({"url": format!("{}/", stub.base)}))
        .await;
    assert_eq!(cancelled.outcome, Outcome::Unknown);
}

#[tokio::test]
async fn private_addresses_are_refused_after_dns_resolution() {
    let stub = site(None, |_| Response::html("<p>secret</p>")).await;
    let mut cfg = supermath_web::WebConfig::default();
    cfg.allow_private_hosts = false;
    let tool = toolkit(cfg).read();
    let port = stub.base.rsplit(':').next().unwrap().to_owned();
    for url in [
        stub.base.clone(),
        format!("http://localhost:{port}/"),
        "http://[::1]:9/".to_owned(),
        "https://169.254.169.254/latest/meta-data".to_owned(),
        "http://10.0.0.5/".to_owned(),
    ] {
        let envelope = tool.call(&ctx(), json!({"url": url})).await;
        assert_eq!(envelope.code(), Some(ErrorCode::InvalidInput), "{url}");
    }
    assert!(stub.requests().is_empty());
}

#[tokio::test]
async fn a_hostname_is_resolved_and_fetched_when_private_hosts_are_allowed() {
    let stub = site(None, |_| Response::html("<p>by name</p>")).await;
    let port = stub.base.rsplit(':').next().unwrap().to_owned();
    let gemini = Stub::start(|_| Response::status(404)).await;
    let envelope = toolkit(config(&gemini))
        .read()
        .call(&ctx(), json!({"url": format!("http://localhost:{port}/")}))
        .await;
    assert!(envelope.is_ok(), "{envelope:?}");
}

#[tokio::test]
async fn arxiv_urls_are_sent_to_lit_get_paper() {
    let gemini = Stub::start(|_| Response::status(404)).await;
    let tool = toolkit(config(&gemini)).read();
    let abs = tool
        .call(&ctx(), json!({"url": "https://arxiv.org/abs/1908.08483v2"}))
        .await;
    assert_eq!(abs.code(), Some(ErrorCode::InvalidInput));
    assert_eq!(abs.next, [r#"lit_get_paper {"arxiv_id": "1908.08483"}"#]);
    let listing = tool
        .call(
            &ctx(),
            json!({"url": "https://export.arxiv.org/list/math/new"}),
        )
        .await;
    assert_eq!(listing.code(), Some(ErrorCode::InvalidInput));
    assert_eq!(listing.next.len(), 0);
    assert_eq!(gemini.requests().len(), 0);
}

#[tokio::test]
async fn bad_arguments_and_the_daily_cap() {
    let gemini = Stub::start(|_| Response::status(404)).await;
    let mut cfg = config(&gemini);
    cfg.daily_cap = 1;
    let tool = toolkit(cfg).read();
    for args in [
        json!({}),
        json!({"url": "not a url"}),
        json!({"url": "ftp://example.com/x"}),
        json!({"url": "https://example.com/", "cursor": "zzz"}),
        json!({"url": "https://example.com/", "extra": 1}),
    ] {
        assert_eq!(
            tool.call(&ctx(), args).await.code(),
            Some(ErrorCode::InvalidInput)
        );
    }
    // Bad arguments spend nothing: the first fetch gets the one call.
    let first = tool
        .call(&ctx(), json!({"url": "http://127.0.0.1:1/"}))
        .await;
    assert_eq!(first.code(), Some(ErrorCode::BackendUnavailable));
    let second = tool
        .call(&ctx(), json!({"url": "http://127.0.0.1:1/"}))
        .await;
    assert_eq!(second.code(), Some(ErrorCode::BudgetExceeded));
}

#[tokio::test]
async fn reading_needs_no_gemini_key() {
    let stub = site(None, |_| Response::html("<p>free</p>")).await;
    let cfg = supermath_web::WebConfig::default();
    let mut cfg = cfg;
    cfg.allow_private_hosts = true;
    let envelope = toolkit(cfg)
        .read()
        .call(&ctx(), json!({"url": format!("{}/", stub.base)}))
        .await;
    assert!(envelope.is_ok());
}
