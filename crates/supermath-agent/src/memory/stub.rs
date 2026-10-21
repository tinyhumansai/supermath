//! A one-request loopback HTTP stub of `CortexDB`, for unit tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// One request the stub received.
#[derive(Debug)]
pub(crate) struct Seen {
    /// The request line and headers.
    pub(crate) head: String,
    /// The JSON body, or `Null`.
    pub(crate) body: Value,
}

/// Serves exactly one HTTP request with `status` and `body`, then hands back
/// what it received.
pub(crate) async fn stub(status: u16, body: &str) -> (String, tokio::task::JoinHandle<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let body = body.to_string();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
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
        let response = format!(
            "HTTP/1.1 {status} STUB\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        Seen {
            head,
            body: serde_json::from_slice(&buffer).unwrap_or(Value::Null),
        }
    });
    (url, handle)
}
