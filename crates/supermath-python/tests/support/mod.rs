//! A loopback stub of the Python service replaying recorded answers, and
//! helpers shared by the integration tests. Nothing here touches the real
//! network; the fixtures in `tests/fixtures/` were recorded from the real
//! service (see `docker/python/`).

#![allow(
    dead_code,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use supermath_files::{ProjectPath, Workspace, WriteMeta};
use supermath_python::{Python, PythonConfig};
use supermath_tool::{CallContext, Project};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// One request the stub saw.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub body: Value,
}

/// What the stub answers.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub body: Value,
    pub delay_ms: u64,
}

impl Reply {
    pub fn ok(body: Value) -> Self {
        Self {
            status: 200,
            body,
            delay_ms: 0,
        }
    }

    pub fn fixture(name: &str) -> Self {
        Self::ok(fixture(name))
    }

    pub fn error(status: u16, code: &str, message: &str) -> Self {
        Self {
            status,
            body: json!({"error": {"code": code, "message": message}}),
            delay_ms: 0,
        }
    }

    pub fn delayed(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }

    /// Sets the `file_sha256` the service reports it ran.
    pub fn with_file_sha(mut self, sha: &str) -> Self {
        if self.body.get("file_sha256").is_some() {
            self.body["file_sha256"] = json!(sha);
        }
        self
    }
}

/// The JSON of `tests/fixtures/<name>.json`.
pub fn fixture(name: &str) -> Value {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        &format!("{name}.json"),
    ]
    .iter()
    .collect();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
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
                    let Some(request) = read_request(&mut socket).await else {
                        return;
                    };
                    log.lock().unwrap().push(request.clone());
                    let reply = handler(&request);
                    if reply.delay_ms > 0 {
                        tokio::time::sleep(Duration::from_millis(reply.delay_ms)).await;
                    }
                    let body = serde_json::to_vec(&reply.body).unwrap();
                    let head = format!(
                        "HTTP/1.1 {} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        reply.status,
                        body.len()
                    );
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&body).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        Self { base, requests }
    }

    /// Every request seen so far.
    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }
}

async fn read_request(socket: &mut tokio::net::TcpStream) -> Option<Recorded> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => return None,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.lines();
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_owned();
    let path = first.next()?.to_owned();
    let length = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    while buf.len() < head_end + length {
        match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    let body = serde_json::from_slice(&buf[head_end..]).unwrap_or(Value::Null);
    Some(Recorded { method, path, body })
}

/// A scratch workspace with a Python toolkit over it.
pub struct Setup {
    pub dir: tempfile::TempDir,
    pub workspace: Arc<Workspace>,
    pub python: Python,
}

impl Setup {
    /// Builds a toolkit that talks to `url` (or nothing).
    pub fn new(url: Option<&str>) -> Self {
        Self::in_dir(tempfile::tempdir().unwrap(), url)
    }

    /// As [`Setup::new`], over an existing scratch directory.
    pub fn in_dir(dir: tempfile::TempDir, url: Option<&str>) -> Self {
        let workspace = Arc::new(Workspace::new(dir.path()));
        let config = PythonConfig {
            url: url.map(str::to_owned),
            max_timeout_s: 300,
            max_frame: 1_000_000,
            ..PythonConfig::default()
        };
        let python = Python::new(&config, workspace.clone()).unwrap();
        Self {
            dir,
            workspace,
            python,
        }
    }

    /// Writes `source` to `path` in project `demo`, returning its digest.
    pub fn write(&self, path: &str, source: &str) -> String {
        let path = ProjectPath::parse(path).unwrap();
        let meta = WriteMeta::new("file_write", "setup");
        self.workspace
            .write("demo", &path, source, None, &meta)
            .unwrap()
            .sha256
    }

    pub fn tool(&self, name: &str) -> Arc<dyn supermath_tool::Tool> {
        self.python
            .tools()
            .into_iter()
            .find(|t| t.name() == name)
            .unwrap()
    }
}

/// A call context for project `demo`.
pub fn ctx() -> CallContext {
    ctx_with("call-1")
}

pub fn ctx_with(call_id: &str) -> CallContext {
    CallContext::new(
        Project::new("demo", "ws:test/project:demo").unwrap(),
        call_id,
    )
}
