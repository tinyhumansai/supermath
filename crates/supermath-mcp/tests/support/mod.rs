//! Shared fixtures: a loopback HTTP stub, a scripted Lean service, and a
//! config that points every backend at stubs. Nothing here touches the real
//! network.

#![allow(
    dead_code,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use supermath_ledger::memory::stub::FakeCortex;
use supermath_mcp::{Config, Toolkit};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// One request a stub saw.
#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub body: Value,
}

/// A stub's answer.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl Reply {
    pub fn json(value: &Value) -> Self {
        Self {
            status: 200,
            content_type: "application/json",
            body: value.to_string(),
        }
    }

    pub fn status(status: u16) -> Self {
        Self {
            status,
            content_type: "application/json",
            body: json!({"message": "stub"}).to_string(),
        }
    }

    pub fn atom(body: &str) -> Self {
        Self {
            status: 200,
            content_type: "application/atom+xml",
            body: body.to_owned(),
        }
    }
}

/// A running loopback stub.
pub struct Stub {
    pub base: String,
    seen: Arc<Mutex<Vec<Request>>>,
}

impl Stub {
    pub async fn start(handler: impl Fn(&Request) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let handler = Arc::new(handler);
        let log = seen.clone();
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
                    let target = first.next().unwrap_or("/");
                    let path = target.split('?').next().unwrap_or("/").to_owned();
                    let body =
                        serde_json::from_slice(&buf[split..split + length]).unwrap_or(Value::Null);
                    let request = Request { method, path, body };
                    log.lock().unwrap().push(request.clone());
                    let reply = handler(&request);
                    let out = format!(
                        "HTTP/1.1 {} Stub\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        reply.status,
                        reply.content_type,
                        reply.body.len(),
                        reply.body
                    );
                    let _ = socket.write_all(out.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        Self { base, seen }
    }

    /// How many requests hit `path`.
    pub fn count(&self, path: &str) -> usize {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.path == path)
            .count()
    }

    /// The bodies of every request to `path`, oldest first.
    pub fn bodies(&self, path: &str) -> Vec<Value> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.path == path)
            .map(|r| r.body.clone())
            .collect()
    }
}

/// A scripted Lean service: `/check` answers whatever `reply` currently
/// holds, `/health` the toolchain.
pub struct LeanService {
    pub stub: Stub,
    reply: Arc<Mutex<Value>>,
}

impl LeanService {
    pub async fn start() -> Self {
        let reply = Arc::new(Mutex::new(clean_reply()));
        let shared = reply.clone();
        let stub = Stub::start(move |r| match r.path.as_str() {
            "/health" => Reply::json(
                &json!({"toolchain": "leanprover/lean4:v4.20.0", "mathlib": "4f2c9e1ab07d"}),
            ),
            "/check" => Reply::json(&shared.lock().unwrap().clone()),
            _ => Reply::status(404),
        })
        .await;
        Self { stub, reply }
    }

    /// Changes what the service answers to `/check`.
    pub fn answer(&self, reply: Value) {
        *self.reply.lock().unwrap() = reply;
    }
}

/// A clean check of `two_add_two`.
pub fn clean_reply() -> Value {
    json!({"status": "ok", "toolchain": "leanprover/lean4:v4.20.0", "mathlib": "4f2c9e1ab07d",
        "elapsed_ms": 1840, "messages": [], "sorries": [],
        "axioms": {"two_add_two": ["propext"]}})
}

/// Lean source that declares `two_add_two`.
pub const CLEAN_LEAN: &str =
    "import Mathlib\n\ntheorem two_add_two : 2 + 2 = 4 := by\n  norm_num\n";

/// The path used for the sample proof.
pub const LEAN_PATH: &str = "lean/Supermath/Clean.lean";

/// A scratch projects directory and a config pointing at it.
pub struct Env {
    pub dir: TempDir,
    pub vars: BTreeMap<String, String>,
}

impl Env {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut vars = BTreeMap::new();
        vars.insert(
            "SUPERMATH_PROJECTS".to_owned(),
            dir.path().to_string_lossy().into_owned(),
        );
        Self { dir, vars }
    }

    pub fn set(mut self, name: &str, value: &str) -> Self {
        self.vars.insert(name.to_owned(), value.to_owned());
        self
    }

    pub fn config(&self) -> Config {
        let vars = self.vars.clone();
        Config::from_lookup(move |name| vars.get(name).cloned()).unwrap()
    }

    pub fn toolkit(&self) -> Toolkit {
        Toolkit::compose(&self.config(), "demo").unwrap()
    }

    /// Writes a project file and returns its path under the projects root.
    pub fn put(&self, relative: &str, content: &str) {
        let path = self.dir.path().join("demo").join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }
}

/// A fake `CortexDB` server.
pub async fn cortex() -> FakeCortex {
    FakeCortex::start().await
}

/// Keeps the coverage instrumentation variables when a test clears a child
/// process's environment, so the spawned binary still reports its coverage.
pub fn keep_coverage_env(command: &mut std::process::Command) {
    for (name, value) in std::env::vars() {
        if name.starts_with("LLVM_") || name.starts_with("CARGO_LLVM") {
            command.env(name, value);
        }
    }
}
