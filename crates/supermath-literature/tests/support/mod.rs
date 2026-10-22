//! A loopback HTTP stub serving recorded fixtures, and helpers shared by the
//! integration tests. Nothing here touches the real network.

#![allow(
    dead_code,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use supermath_literature::LiteratureConfig;
use supermath_tool::{CallContext, Project};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// One request the stub saw.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub target: String,
    pub headers: HashMap<String, String>,
}

impl Recorded {
    /// The path without its query.
    pub fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or("")
    }

    /// A decoded query parameter.
    pub fn query(&self, name: &str) -> Option<String> {
        let url = url::Url::parse(&format!("http://stub{}", self.target)).unwrap();
        url.query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    }
}

/// What the stub answers.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub delay_ms: u64,
}

impl Reply {
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: body.into(),
            delay_ms: 0,
        }
    }

    pub fn fixture(name: &str) -> Self {
        Self::ok(fixture(name))
    }

    pub fn status(status: u16) -> Self {
        Self {
            status,
            ..Self::ok(Vec::new())
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    pub fn delayed(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }
}

/// The bytes of `tests/fixtures/<name>`.
pub fn fixture(name: &str) -> Vec<u8> {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", name]
        .iter()
        .collect();
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {}: {e}", path.display()))
}

/// A running stub.
pub struct Stub {
    pub base: String,
    requests: Arc<Mutex<Vec<Recorded>>>,
}

impl Stub {
    pub async fn start(handler: impl Fn(&Recorded) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let handler = Arc::new(handler);
        let log = requests.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let handler = handler.clone();
                let log = log.clone();
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let mut chunk = [0u8; 2048];
                    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        match socket.read(&mut chunk).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        }
                    }
                    let head = String::from_utf8_lossy(&buf).into_owned();
                    let mut lines = head.lines();
                    let target = lines
                        .next()
                        .and_then(|l| l.split_whitespace().nth(1))
                        .unwrap_or("/")
                        .to_owned();
                    let headers = lines
                        .filter_map(|l| l.split_once(':'))
                        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
                        .collect();
                    let recorded = Recorded { target, headers };
                    log.lock().unwrap().push(recorded.clone());
                    let reply = handler(&recorded);
                    if reply.delay_ms > 0 {
                        tokio::time::sleep(Duration::from_millis(reply.delay_ms)).await;
                    }
                    let mut out = format!(
                        "HTTP/1.1 {} Stub\r\nContent-Length: {}\r\nConnection: close\r\n",
                        reply.status,
                        reply.body.len()
                    );
                    for (k, v) in &reply.headers {
                        let _ = write!(out, "{k}: {v}\r\n");
                    }
                    out.push_str("\r\n");
                    let _ = socket.write_all(out.as_bytes()).await;
                    let _ = socket.write_all(&reply.body).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        Self { base, requests }
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.requests().len()
    }
}

/// A config pointing every source at `base`, with no politeness delay.
pub fn config(base: &str) -> LiteratureConfig {
    LiteratureConfig {
        arxiv_api_url: format!("{base}/api/query"),
        arxiv_eprint_url: format!("{base}/e-print"),
        scholar_url: format!("{base}/graph/v1"),
        oeis_url: format!("{base}/search"),
        contact: "tests@example.org".into(),
        arxiv_interval_ms: 0,
        timeout_ms: 400,
        ..LiteratureConfig::default()
    }
}

pub fn ctx() -> CallContext {
    CallContext::new(Project::new("lit", "ws:test").unwrap(), "call-1")
}

/// A gzipped single-file e-print.
pub fn gzip(text: &str) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(text.as_bytes()).unwrap();
    encoder.finish().unwrap()
}

/// The stub every tool test shares: routes by path, then by a marker in the
/// query, so one stub serves a tool's success and every failure.
pub fn routes(req: &Recorded) -> Reply {
    let marker = |name: &str| req.query(name).unwrap_or_default();
    match req.path() {
        "/api/query" => {
            let what = format!("{} {}", marker("search_query"), marker("id_list"));
            if what.contains("ratelimited") || what.contains("2488.88888") {
                Reply::status(429).header("Retry-After", "7")
            } else if what.contains("broken") || what.contains("2477.77777") {
                Reply::status(500)
            } else if what.contains("slow") || what.contains("2466.66666") {
                Reply::fixture("search_sendov.xml").delayed(1500)
            } else if what.contains("nothing") || what.contains("2499.99999") {
                Reply::fixture("empty.xml")
            } else if what.contains("badq") {
                Reply::fixture("error.xml")
            } else if what.contains("garbled") {
                Reply::ok("</feed>")
            } else if what.contains("2402.00002") {
                Reply::fixture("paper_2402.00002.xml")
            } else if what.contains("2401.00001") {
                Reply::fixture("paper_2401.00001.xml")
            } else {
                Reply::fixture("search_sendov.xml")
            }
        }
        "/e-print/2401.00001v2" => Reply::fixture("eprint_2401.00001v2.tar.gz"),
        "/e-print/2402.00002v1" => Reply::fixture("eprint_pdf_only.pdf"),
        p if p.starts_with("/e-print/") => Reply::status(404),
        p if p.starts_with("/graph/v1/paper/arXiv:") => {
            if p.contains("2499.99999") {
                Reply::status(404)
            } else if p.contains("2488.88888") {
                Reply::status(429)
            } else if p.contains("2477.77777") {
                Reply::status(500)
            } else if p.contains("2466.66666") {
                Reply::fixture("scholar_references.json").delayed(1500)
            } else if p.ends_with("/citations") {
                Reply::fixture("scholar_citations.json")
            } else {
                Reply::fixture("scholar_references.json")
            }
        }
        "/search" => {
            let q = marker("q");
            if q.contains("ratelimited") {
                Reply::status(429).header("Retry-After", "2")
            } else if q.contains("broken") {
                Reply::status(500)
            } else if q.contains("slow") {
                Reply::fixture("oeis_fibonacci.json").delayed(1500)
            } else if q.contains("nothing") {
                Reply::fixture("oeis_none.json")
            } else if q.contains("legacy") {
                Reply::fixture("oeis_legacy.json")
            } else if q.contains("garbled") {
                Reply::ok("{not json")
            } else {
                Reply::fixture("oeis_fibonacci.json")
            }
        }
        _ => Reply::status(404),
    }
}
