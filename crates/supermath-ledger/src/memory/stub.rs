//! Loopback HTTP stubs of `CortexDB`, for tests that must never touch the
//! network.
//!
//! [`stub`] serves exactly one canned response and reports what it received.
//! [`FakeCortex`] is a small stateful server: it stores the experiences it is
//! sent, per scope, and recalls them newest first, so the ledger can be
//! exercised end to end. Compiled for this crate's tests and, behind the
//! `test-support` feature, for downstream tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// One request the stub received.
#[derive(Debug)]
pub struct Seen {
    /// The request line and headers.
    pub head: String,
    /// The JSON body, or `Null`.
    pub body: Value,
}

/// Reads one HTTP request from `socket`.
async fn read_request(socket: &mut TcpStream) -> Seen {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let (head, length) = loop {
        let read = socket.read(&mut chunk).await.unwrap();
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buffer[..end]).to_string();
            let length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            buffer.drain(..end + 4);
            break (head, length);
        }
    };
    while buffer.len() < length {
        let read = socket.read(&mut chunk).await.unwrap();
        buffer.extend_from_slice(&chunk[..read]);
    }
    Seen {
        head,
        body: serde_json::from_slice(&buffer).unwrap_or(Value::Null),
    }
}

/// Writes a JSON response and closes the connection.
async fn respond(socket: &mut TcpStream, status: u16, body: &str) {
    let response = format!(
        "HTTP/1.1 {status} STUB\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(response.as_bytes()).await.unwrap();
}

/// Serves exactly one HTTP request with `status` and `body`, then hands back
/// what it received.
///
/// # Panics
///
/// If the loopback socket cannot be bound or read; this is test support.
pub async fn stub(status: u16, body: &str) -> (String, tokio::task::JoinHandle<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let body = body.to_string();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let seen = read_request(&mut socket).await;
        respond(&mut socket, status, &body).await;
        seen
    });
    (url, handle)
}

/// One experience a [`FakeCortex`] stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredEvent {
    /// The id the server assigned.
    pub event_id: String,
    /// The scope it was written to.
    pub scope: String,
    /// The note text.
    pub text: String,
    /// The labels it carried.
    pub labels: Vec<String>,
}

#[derive(Debug, Default)]
struct State {
    events: Vec<StoredEvent>,
    keys: Vec<(String, String)>,
    refuse_with: Option<u16>,
    refuse_scope: Option<String>,
    skip_index: bool,
    recalls: usize,
}

/// A stateful loopback `CortexDB`: `POST /v1/experience` stores, `POST
/// /v1/recall` returns the scope's events newest first. A write repeated
/// with the same idempotency key stores nothing new.
#[derive(Debug, Clone)]
pub struct FakeCortex {
    url: String,
    state: Arc<Mutex<State>>,
}

impl FakeCortex {
    /// Starts the server on a free loopback port. It runs until the test's
    /// runtime shuts down.
    ///
    /// # Panics
    ///
    /// If the loopback socket cannot be bound; this is test support.
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(State::default()));
        let shared = Arc::clone(&state);
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let state = Arc::clone(&shared);
                tokio::spawn(async move {
                    let seen = read_request(&mut socket).await;
                    let (status, body) = handle(&state, &seen);
                    respond(&mut socket, status, &body).await;
                });
            }
        });
        Self { url, state }
    }

    /// The server's base URL.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Every stored event, oldest first.
    #[must_use]
    pub fn events(&self) -> Vec<StoredEvent> {
        self.lock().events.clone()
    }

    /// How many recalls the server has answered.
    #[must_use]
    pub fn recalls(&self) -> usize {
        self.lock().recalls
    }

    /// Answers every following request with `status` until cleared with
    /// `None`.
    pub fn refuse_with(&self, status: Option<u16>) {
        self.lock().refuse_with = status;
    }

    /// Answers every request whose scope contains `fragment` with `503`,
    /// until cleared with `None`. Other scopes are served normally, so one
    /// project can be "down" while the rest work.
    pub fn refuse_scope(&self, fragment: Option<&str>) {
        self.lock().refuse_scope = fragment.map(str::to_string);
    }

    /// Makes following writes report only the `stored` stage.
    pub fn skip_index(&self, skip: bool) {
        self.lock().skip_index = skip;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn handle(state: &Mutex<State>, seen: &Seen) -> (u16, String) {
    let mut state = state.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(status) = state.refuse_with {
        return (
            status,
            json!({"error_code": "STUB_REFUSED", "message": "refused"}).to_string(),
        );
    }
    let scope = seen.body["scope"].as_str().unwrap_or_default().to_string();
    if state
        .refuse_scope
        .as_deref()
        .is_some_and(|f| scope.contains(f))
    {
        return (
            503,
            json!({"error_code": "STUB_REFUSED", "message": "scope is down"}).to_string(),
        );
    }
    if seen.head.starts_with("POST /v1/experience") {
        let key = seen.body["idempotency_key"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let stages = if state.skip_index {
            json!(["stored"])
        } else {
            json!(["stored", "indexed"])
        };
        if let Some((_, id)) = state.keys.iter().find(|(k, _)| *k == key) {
            return (
                200,
                json!({"event_id": id, "stages_completed": stages}).to_string(),
            );
        }
        let event_id = format!("ev-{}", state.events.len() + 1);
        let labels = seen.body["context"]["labels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| l.as_str().map(str::to_string))
            .collect();
        state.events.push(StoredEvent {
            event_id: event_id.clone(),
            scope,
            text: seen.body["content"]["text"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            labels,
        });
        state.keys.push((key, event_id.clone()));
        (
            200,
            json!({"event_id": event_id, "stages_completed": stages}).to_string(),
        )
    } else if seen.head.starts_with("POST /v1/recall") {
        state.recalls += 1;
        let limit = seen.body["budgets"]["per_layer_limits"]["event"]
            .as_u64()
            .map_or(usize::MAX, |n| usize::try_from(n).unwrap_or(usize::MAX));
        let events: Vec<Value> = state
            .events
            .iter()
            .rev()
            .filter(|e| e.scope == scope)
            .take(limit)
            .map(|e| json!({"content": {"text": e.text}}))
            .collect();
        if events.is_empty() {
            (404, json!({"error_code": "NOT_FOUND"}).to_string())
        } else {
            (200, json!({"layers": {"events": events}}).to_string())
        }
    } else {
        (404, json!({"error_code": "NOT_FOUND"}).to_string())
    }
}
