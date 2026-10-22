//! Every tool passes the shared contract checker.

use std::time::Duration;

use serde_json::json;
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{Backend, Budget, Exposure, Kind, Permission, Tool, ToolRegistry};
use supermath_web::WebToolkit;

use crate::support::{Response, Stub, config, ctx, fixture, toolkit};

fn no_rows() -> supermath_tool::CallContext {
    let mut context = ctx();
    context.budget = Budget {
        rows: 0,
        ..Budget::default()
    };
    context
}

/// How long a fixture meant to hang is held: far past any timeout below.
const HANG: Duration = Duration::from_secs(60);

/// How long a call may take before it is a timeout. Generous, so a loaded
/// machine (coverage instrumentation, a busy CI runner) never turns a fast
/// answer into a timeout; the hanging fixtures outlast it regardless.
const TIMEOUT: Duration = Duration::from_secs(2);

/// One stub for both Gemini and pages; the request decides what happens.
async fn stub() -> Stub {
    stub_with_latency(Duration::ZERO).await
}

/// The same stub, with every answer that is not meant to hang held back by
/// `latency` — what a loaded machine does to a loopback server.
async fn stub_with_latency(latency: Duration) -> Stub {
    Stub::start(move |request| {
        let host = request.headers.get("host").cloned().unwrap_or_default();
        let base = format!("http://{host}");
        let response = answer(request, &base);
        if response.is_instant() {
            response.with_delay(latency)
        } else {
            response
        }
    })
    .await
}

fn answer(request: &crate::support::Request, base: &str) -> Response {
    if request.path.contains(":generateContent") {
        if request.body.contains("trigger-429") {
            Response::status(429)
        } else if request.body.contains("trigger-500") {
            Response::status(500)
        } else if request.body.contains("trigger-slow") {
            Response::json("{}").with_delay(HANG)
        } else {
            Response::json(&fixture("grounded.json", base))
        }
    } else if request.path == "/robots.txt" {
        Response::status(404)
    } else if let Some(route) = crate::support::redirect_route(&request.path) {
        route
    } else {
        match request.path.as_str() {
            "/page" => Response::html("<p>hello</p>"),
            "/forbidden" => Response::status(403),
            "/busy" => Response::status(429),
            "/boom" => Response::status(500),
            "/slow" => Response::html("x").with_delay(HANG),
            _ => Response::status(404),
        }
    }
}

fn toolkit_for(stub: &Stub) -> WebToolkit {
    let mut config = config(stub);
    config.search_timeout = TIMEOUT;
    config.research_timeout = TIMEOUT;
    config.read_timeout = TIMEOUT;
    toolkit(config)
}

#[tokio::test]
async fn web_search_keeps_the_tool_contract() {
    let stub = stub().await;
    let tool = toolkit_for(&stub).search();
    let fixtures = [
        Fixture::new(json!({"question": "fine"})),
        Fixture::new(json!({"question": ""})),
        Fixture::new(json!({"question": "trigger-429"})),
        Fixture::new(json!({"question": "trigger-500"})),
        Fixture::new(json!({"question": "trigger-slow"})),
        Fixture::with_ctx(no_rows(), json!({"question": "fine"})),
    ];
    contract::check(&tool, "web", &fixtures).await.unwrap();
    assert_eq!(tool.exposure(), Exposure::Direct);
    assert_eq!(tool.permission(), Permission::Read);
    assert_eq!(tool.kind(), Kind::Agentic(Backend::Gemini));
    assert!(tool.description().starts_with("Runs a"));
}

#[tokio::test]
async fn web_research_keeps_the_tool_contract() {
    let stub = stub().await;
    let tool = toolkit_for(&stub).research();
    let fixtures = [
        Fixture::new(json!({"question": "fine", "facets": ["status"]})),
        Fixture::new(json!({"question": ""})),
        Fixture::new(json!({"question": "trigger-429", "facets": ["status"]})),
        Fixture::new(json!({"question": "trigger-500", "facets": ["status"]})),
        Fixture::new(json!({"question": "trigger-slow", "facets": ["status"]})),
        Fixture::with_ctx(no_rows(), json!({"question": "fine"})),
    ];
    contract::check(&tool, "web", &fixtures).await.unwrap();
    assert_eq!(tool.exposure(), Exposure::Deferred);
    assert_eq!(tool.kind(), Kind::Agentic(Backend::Gemini));
    assert!(tool.description().starts_with("Runs a"));
}

#[tokio::test]
async fn web_read_keeps_the_tool_contract() {
    let stub = stub().await;
    let tool = toolkit_for(&stub).read();
    let url = |path: &str| json!({"url": format!("{}{path}", stub.base)});
    let fixtures = [
        Fixture::new(url("/page")),
        Fixture::new(json!({"url": "nonsense"})),
        Fixture::new(url("/missing")),
        Fixture::new(url("/forbidden")),
        Fixture::new(url("/busy")),
        Fixture::new(url("/slow")),
        Fixture::new(url("/boom")),
        Fixture::with_ctx(no_rows(), url("/page")),
    ];
    contract::check(&tool, "web", &fixtures).await.unwrap();
    assert_eq!(tool.exposure(), Exposure::Deferred);
    assert_eq!(tool.kind(), Kind::Primitive);
}

#[tokio::test]
async fn the_toolkit_registers_all_three_tools_once() {
    let stub = stub().await;
    let toolkit = toolkit_for(&stub);
    let mut registry = ToolRegistry::new();
    toolkit.register(&mut registry).unwrap();
    let names: Vec<_> = registry.iter().map(|t| t.name()).collect();
    assert_eq!(names, ["web_read", "web_research", "web_search"]);
    assert!(toolkit.register(&mut registry).is_err());
    assert_eq!(toolkit.tools().len(), 3);
}

#[tokio::test]
async fn web_read_keeps_the_tool_contract_on_a_slow_machine() {
    let stub = stub_with_latency(Duration::from_millis(600)).await;
    let tool = toolkit_for(&stub).read();
    let url = |path: &str| json!({"url": format!("{}{path}", stub.base)});
    let fixtures = [
        Fixture::new(url("/page")),
        Fixture::new(json!({"url": "nonsense"})),
        Fixture::new(url("/missing")),
        Fixture::new(url("/forbidden")),
        Fixture::new(url("/busy")),
        Fixture::new(url("/slow")),
        Fixture::new(url("/boom")),
        Fixture::with_ctx(no_rows(), url("/page")),
    ];
    contract::check(&tool, "web", &fixtures).await.unwrap();
}
