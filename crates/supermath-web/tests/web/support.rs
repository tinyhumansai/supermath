//! A hand-rolled loopback HTTP stub and shared test helpers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use supermath_tool::{CallContext, Project};
use supermath_web::{Clock, WebConfig, WebToolkit};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// The key tests present; it must never appear in any answer.
pub(crate) const KEY: &str = "test-key-DO-NOT-LEAK";

/// One request the stub received.
#[derive(Debug, Clone)]
pub(crate) struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: String,
}

/// What the stub answers.
#[derive(Debug, Clone)]
pub(crate) struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    delay: Duration,
}

impl Response {
    pub(crate) fn status(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            delay: Duration::ZERO,
        }
    }

    pub(crate) fn json(body: &str) -> Self {
        Self::status(200)
            .with_header("content-type", "application/json")
            .with_body(body.as_bytes())
    }

    pub(crate) fn html(body: &str) -> Self {
        Self::status(200)
            .with_header("content-type", "text/html; charset=utf-8")
            .with_body(body.as_bytes())
    }

    pub(crate) fn text(body: &str) -> Self {
        Self::status(200)
            .with_header("content-type", "text/plain")
            .with_body(body.as_bytes())
    }

    pub(crate) fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    pub(crate) fn with_body(mut self, body: &[u8]) -> Self {
        self.body = body.to_vec();
        self
    }

    /// Whether the answer is sent without any deliberate delay.
    pub(crate) fn is_instant(&self) -> bool {
        self.delay.is_zero()
    }

    pub(crate) fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

/// A running stub server.
pub(crate) struct Stub {
    /// `http://127.0.0.1:{port}`.
    pub(crate) base: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Stub {
    pub(crate) async fn start(
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let handler = Arc::new(handler);
        let log = Arc::clone(&requests);
        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                let handler = Arc::clone(&handler);
                let log = Arc::clone(&log);
                tokio::spawn(async move {
                    let Some(request) = read_request(&mut stream).await else {
                        return;
                    };
                    let response = handler(&request);
                    let head = request.method == "HEAD";
                    log.lock().unwrap().push(request);
                    tokio::time::sleep(response.delay).await;
                    let mut lines = vec![format!("HTTP/1.1 {} X", response.status)];
                    lines.extend(response.headers.iter().map(|(n, v)| format!("{n}: {v}")));
                    lines.push(format!("content-length: {}", response.body.len()));
                    lines.push("connection: close".to_owned());
                    let out = format!("{}\r\n\r\n", lines.join("\r\n"));
                    let _ = stream.write_all(out.as_bytes()).await;
                    if !head {
                        let _ = stream.write_all(&response.body).await;
                    }
                    let _ = stream.shutdown().await;
                });
            }
        });
        Self { base, requests }
    }

    /// Every request received so far.
    pub(crate) fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    /// How many requests matched `path_prefix`.
    pub(crate) fn count(&self, path_prefix: &str) -> usize {
        self.requests()
            .iter()
            .filter(|r| r.path.starts_with(path_prefix))
            .count()
    }
}

async fn read_request(stream: &mut tokio::net::TcpStream) -> Option<Request> {
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 4096];
    let head_end = loop {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break at;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, path) = (first.next()?.to_owned(), first.next()?.to_owned());
    let headers: HashMap<String, String> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let length: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    Some(Request {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

/// Reads `tests/fixtures/{name}`, replacing `{{BASE}}` with the stub's base.
pub(crate) fn fixture(name: &str, base: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path)
        .unwrap()
        .replace("{{BASE}}", base)
}

/// The grounding redirect routes of `redirect_chain.json`, answered with the
/// status and `Location` the fixture names.
pub(crate) fn redirect_route(path: &str) -> Option<Response> {
    let fixture: Value = serde_json::from_str(&fixture("redirect_chain.json", "")).unwrap();
    let route = fixture["routes"].get(path)?;
    let status = u16::try_from(route[0].as_u64().unwrap()).unwrap();
    let response = Response::status(status);
    Some(match route[1].as_str() {
        Some(location) => response.with_header("location", location),
        None => response,
    })
}

/// A Gemini stub: `generate` answers `generateContent`, redirect routes
/// answer the rest, anything else is a 404.
pub(crate) async fn gemini_stub(
    generate: impl Fn(&Request) -> Response + Send + Sync + 'static,
) -> Stub {
    Stub::start(move |request| {
        if request.path.contains(":generateContent") {
            generate(request)
        } else {
            redirect_route(&request.path).unwrap_or_else(|| Response::status(404))
        }
    })
    .await
}

/// A stub that answers every Gemini request with one fixture.
pub(crate) async fn fixed_gemini(name: &'static str) -> Stub {
    // The fixture's `{{BASE}}` is the stub's own address, known only after
    // the listener binds, so the first request fills it in from `Host`.
    gemini_stub(move |request| {
        let host = request.headers.get("host").cloned().unwrap_or_default();
        Response::json(&fixture(name, &format!("http://{host}")))
    })
    .await
}

/// A config pointing at `stub`, with a key, loopback redirects, and private
/// hosts allowed.
pub(crate) fn config(stub: &Stub) -> WebConfig {
    let mut config = WebConfig::default()
        .with_api_key(KEY)
        .with_base_url(stub.base.clone());
    config.redirect_hosts = vec!["127.0.0.1".to_owned()];
    config.allow_private_hosts = true;
    config
}

pub(crate) fn toolkit(config: WebConfig) -> WebToolkit {
    WebToolkit::new(config).unwrap()
}

pub(crate) fn ctx() -> CallContext {
    CallContext::new(
        Project::new("test-project", "ws:test/project:test-project").unwrap(),
        "call-1",
    )
}

/// A clock that always reads the same instant.
#[derive(Debug)]
pub(crate) struct FixedClock(pub(crate) u64);

impl Clock for FixedClock {
    fn now_secs(&self) -> u64 {
        self.0
    }
}

/// Asserts the envelope, serialised, never contains the API key.
pub(crate) fn assert_no_key(envelope: &supermath_tool::Envelope) {
    let text = serde_json::to_string(envelope).unwrap();
    assert!(!text.contains(KEY), "the API key leaked into an envelope");
}
