//! A loopback HTTP stub for the binary's tests: answers every request with a
//! handler and records what it received.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    dead_code,
    unreachable_pub
)]

use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// One request the stub received.
#[derive(Debug, Clone)]
pub struct Seen {
    /// `POST`, `GET`, ...
    pub method: String,
    /// The path and query, such as `/v1/recall`.
    pub path: String,
    /// The JSON body, or `Null` when there was none.
    pub body: Value,
}

/// What the handler answers: a status and a JSON body.
pub type Handler = Arc<dyn Fn(&Seen) -> (u16, Value) + Send + Sync>;

/// A running stub.
pub struct Stub {
    /// `http://127.0.0.1:<port>`.
    pub url: String,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Stub {
    /// Starts serving on a free loopback port.
    pub async fn start(handler: Handler) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = seen.clone();
        tokio::spawn(async move {
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let handler = handler.clone();
                let record = record.clone();
                tokio::spawn(async move { serve(socket, handler, record).await });
            }
        });
        Self { url, seen }
    }

    /// Every request received so far.
    pub fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

async fn serve(mut socket: tokio::net::TcpStream, handler: Handler, record: Arc<Mutex<Vec<Seen>>>) {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    let (head, length) = loop {
        let Ok(read) = socket.read(&mut chunk).await else {
            return;
        };
        if read == 0 {
            return;
        }
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
        let Ok(read) = socket.read(&mut chunk).await else {
            return;
        };
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    let mut request_line = head.lines().next().unwrap_or_default().split(' ');
    let seen = Seen {
        method: request_line.next().unwrap_or_default().to_string(),
        path: request_line.next().unwrap_or_default().to_string(),
        body: serde_json::from_slice(&buffer).unwrap_or(Value::Null),
    };
    let (status, body) = handler(&seen);
    record.lock().unwrap().push(seen);
    let body = body.to_string();
    let response = format!(
        "HTTP/1.1 {status} STUB\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = socket.write_all(response.as_bytes()).await;
}
