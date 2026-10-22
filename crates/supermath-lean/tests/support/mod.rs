//! A loopback HTTP stub that replays recorded Lean-service answers, and
//! helpers shared by the integration tests. Nothing here touches the real
//! network.

#![allow(
    clippy::assert_is_empty,
    dead_code,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use supermath_files::{ProjectPath, Workspace, WriteMeta};
use supermath_lean::{Lean, LeanConfig};
use supermath_tool::{CallContext, Envelope, Project, Tool};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// One request the stub saw.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub query: String,
    pub body: Value,
}

/// What the stub answers.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
    pub delay_ms: u64,
    pub headers: Vec<(String, String)>,
}

impl Reply {
    pub fn json(value: &Value) -> Self {
        Self {
            status: 200,
            body: serde_json::to_vec(value).unwrap(),
            delay_ms: 0,
            headers: Vec::new(),
        }
    }

    pub fn fixture(name: &str) -> Self {
        Self {
            status: 200,
            body: fixture_bytes(name),
            delay_ms: 0,
            headers: Vec::new(),
        }
    }

    pub fn status(status: u16, message: &str) -> Self {
        Self {
            status,
            ..Self::json(&serde_json::json!({"message": message}))
        }
    }

    pub fn raw(body: &str) -> Self {
        Self {
            status: 200,
            body: body.as_bytes().to_vec(),
            delay_ms: 0,
            headers: Vec::new(),
        }
    }

    pub fn delayed(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

/// The bytes of `tests/fixtures/<name>`.
pub fn fixture_bytes(name: &str) -> Vec<u8> {
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
                    let mut chunk = [0u8; 4096];
                    let split = loop {
                        if let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            break at + 4;
                        }
                        match socket.read(&mut chunk).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        }
                    };
                    let head = String::from_utf8_lossy(&buf[..split]).into_owned();
                    let length = head
                        .lines()
                        .filter_map(|l| l.split_once(':'))
                        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    while buf.len() < split + length {
                        match socket.read(&mut chunk).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        }
                    }
                    let mut first = head.lines().next().unwrap_or("").split_whitespace();
                    let method = first.next().unwrap_or("").to_owned();
                    let target = first.next().unwrap_or("/").to_owned();
                    let (path, query) = target
                        .split_once('?')
                        .map_or((target.clone(), String::new()), |(p, q)| {
                            (p.to_owned(), q.to_owned())
                        });
                    let body =
                        serde_json::from_slice(&buf[split..split + length]).unwrap_or(Value::Null);
                    let recorded = Recorded {
                        method,
                        path,
                        query,
                        body,
                    };
                    log.lock().unwrap().push(recorded.clone());
                    let reply = handler(&recorded);
                    if reply.delay_ms > 0 {
                        tokio::time::sleep(Duration::from_millis(reply.delay_ms)).await;
                    }
                    let mut out = format!(
                        "HTTP/1.1 {} Stub\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
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

    /// Every request seen, oldest first.
    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }

    /// How many requests hit `path`.
    pub fn count(&self, path: &str) -> usize {
        self.requests().iter().filter(|r| r.path == path).count()
    }

    /// The body of the last request to `path`.
    pub fn last_body(&self, path: &str) -> Value {
        self.requests()
            .iter()
            .rev()
            .find(|r| r.path == path)
            .unwrap_or_else(|| panic!("no request to {path}"))
            .body
            .clone()
    }
}

/// The address of a port nothing listens on.
pub async fn dead_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    url
}

/// The service's recorded answer for a `/check` of `path`: the fixture
/// `check_<file stem lowercased>.json`.
pub fn check_fixture(path: &str) -> Reply {
    let stem = path
        .rsplit('/')
        .next()
        .unwrap()
        .trim_end_matches(".lean")
        .to_ascii_lowercase();
    Reply::fixture(&format!("check_{stem}.json"))
}

/// A scratch project with a workspace and the Lean bundle over it.
pub struct Env {
    pub dir: TempDir,
    pub workspace: Arc<Workspace>,
    pub lean: Lean,
    pub config: LeanConfig,
}

pub fn config(url: Option<&str>) -> LeanConfig {
    LeanConfig {
        url: url.map(str::to_owned),
        http_slack_ms: 0,
        ..LeanConfig::default()
    }
}

impl Env {
    pub fn new(config: LeanConfig) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let workspace = Arc::new(Workspace::new(dir.path()));
        let lean = Lean::new(config.clone(), workspace.clone()).unwrap();
        Self {
            dir,
            workspace,
            lean,
            config,
        }
    }

    /// A bundle over the stub at `base`.
    pub fn at(base: &str) -> Self {
        Self::new(config(Some(base)))
    }

    /// Writes (or overwrites) a project file; returns its digest.
    pub fn put(&self, path: &str, text: &str) -> String {
        self.put_in("demo", path, text)
    }

    /// As [`Env::put`], in another project.
    pub fn put_in(&self, slug: &str, path: &str, text: &str) -> String {
        let path = ProjectPath::parse(path).unwrap();
        let old = self
            .workspace
            .exists(slug, &path)
            .unwrap()
            .then(|| self.workspace.sha256(slug, &path).unwrap());
        let meta = WriteMeta::new("file_write", "setup").allowing_generated();
        self.workspace
            .write(slug, &path, text, old.as_deref(), &meta)
            .unwrap()
            .sha256
    }

    pub fn sha(&self, path: &str) -> String {
        let path = ProjectPath::parse(path).unwrap();
        self.workspace.sha256("demo", &path).unwrap()
    }

    pub fn tool(&self, name: &str) -> Arc<dyn Tool> {
        self.lean
            .tools()
            .into_iter()
            .find(|t| t.name() == name)
            .unwrap()
    }

    pub async fn call(&self, name: &str, args: Value) -> Envelope {
        self.tool(name).call(&ctx(), args).await
    }
}

pub fn ctx() -> CallContext {
    CallContext::new(
        Project::new("demo", "ws:test/project:demo").unwrap(),
        "call-1",
    )
}

/// The file stem of the `path` a request names, lowercased.
fn stem(body: &Value) -> String {
    body["path"]
        .as_str()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim_end_matches(".lean")
        .to_ascii_lowercase()
}

/// A stub of the whole Lean service and of the hosted indexes, replaying the
/// recorded answers in `tests/fixtures`. Requests are routed by the file stem
/// they name (`down` is a 500, `gone` a 404, `slow` a timeout) or, for
/// searches, by the query.
pub async fn service() -> Stub {
    Stub::start(|r| {
        let stem = stem(&r.body);
        let query = r.body["query"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| {
                r.query
                    .strip_prefix("q=")
                    .map(|q| q.replace("%20", " ").replace('+', " "))
            })
            .unwrap_or_default();
        if stem == "down"
            || r.body["project"] == "downproj"
            || query.contains("down")
            || r.body["name"] == "Down.thing"
        {
            return Reply::status(500, "the repl pool is restarting");
        }
        if stem == "gone" {
            return Reply::status(404, "no such file on the projects volume");
        }
        if stem == "slow" {
            return Reply::json(&serde_json::json!({"status": "timeout"}));
        }
        match r.path.as_str() {
            "/health" => Reply::fixture("health.json"),
            "/check" => check_fixture(r.body["path"].as_str().unwrap()),
            "/goals" if stem == "badpos" => {
                Reply::json(&serde_json::json!({"status": "bad_position"}))
            }
            "/goals" => Reply::fixture("goals_ok.json"),
            "/tactics" => match stem.as_str() {
                "nosorry" => Reply::json(&serde_json::json!({"status": "no_sorry"})),
                "exact" => Reply::fixture("tactics_exact.json"),
                "odd" => Reply::fixture("tactics_odd.json"),
                _ => Reply::fixture("tactics_ok.json"),
            },
            "/decl" => match r.body["name"].as_str().unwrap_or("") {
                "Sendov.main" => Reply::fixture("decl_project.json"),
                "Missing.thing" => Reply::json(&serde_json::json!({"status": "not_found"})),
                "Slow.thing" => Reply::json(&serde_json::json!({"status": "timeout"})),
                _ => Reply::fixture("decl_nat_prime.json"),
            },
            "/search" => Reply::fixture("search_name.json"),
            "/counterexample" => match r.body["predicate"].as_str().unwrap_or("") {
                "Euler.notPrime" | "badWitness" => Reply::fixture("counter_found.json"),
                "negCheck" => Reply::fixture("counter_int.json"),
                "noneSuch" => Reply::fixture("counter_none.json"),
                "tooSlow" => Reply::fixture("counter_timeout.json"),
                "tooSlowEarly" => Reply::fixture("counter_timeout_early.json"),
                "notDecidable" => Reply::fixture("counter_bad.json"),
                _ => Reply::json(&serde_json::json!({"status": "weird"})),
            },
            "/loogle" if query.contains("429") => {
                Reply::status(429, "slow down").header("Retry-After", "7")
            }
            "/loogle" if query.contains("broken") => Reply::fixture("loogle_error.json"),
            "/loogle" if query.contains("garbage") => Reply::raw("<html>not json</html>"),
            "/loogle" if query.contains("slowq") => Reply::fixture("loogle.json").delayed(600),
            "/loogle" => Reply::fixture("loogle.json"),
            "/leansearch" => Reply::fixture("leansearch.json"),
            _ => Reply::status(404, "no such route"),
        }
    })
    .await
}

impl Env {
    /// A bundle over [`service`], with the hosted indexes on the same stub and
    /// short query timeouts.
    pub fn over(stub: &Stub) -> Self {
        let mut cfg = config(Some(&stub.base));
        cfg.loogle_url = format!("{}/loogle", stub.base);
        cfg.leansearch_url = format!("{}/leansearch", stub.base);
        cfg.query_timeout_ms = 200;
        Self::new(cfg)
    }
}

/// A context for the project `downproj`, whose every `/check` the stub fails.
pub fn down_ctx() -> CallContext {
    CallContext::new(
        Project::new("downproj", "ws:test/project:downproj").unwrap(),
        "call-down",
    )
}
