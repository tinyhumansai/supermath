//! Tests for the address and scheme checks.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

use super::{fetch_from, is_public, normalize};
use crate::config::WebConfig;
use crate::error::Error;

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

#[test]
fn private_loopback_and_link_local_addresses_are_not_public() {
    for text in [
        "10.1.2.3",
        "172.16.0.1",
        "192.168.1.1",
        "127.0.0.1",
        "169.254.169.254",
        "0.0.0.0",
        "100.64.0.1",
        "224.0.0.1",
        "255.255.255.255",
        "192.0.2.1",
        "0.1.2.3",
        "250.0.0.1",
        "::1",
        "::",
        "fc00::1",
        "fe80::1",
        "ff02::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
    ] {
        assert!(!is_public(ip(text)), "{text}");
    }
}

#[test]
fn ordinary_internet_addresses_are_public() {
    for text in [
        "8.8.8.8",
        "1.1.1.1",
        "140.82.112.3",
        "100.63.0.1",
        "2606:4700:4700::1111",
        "::ffff:8.8.8.8",
    ] {
        assert!(is_public(ip(text)), "{text}");
    }
}

#[test]
fn http_is_upgraded_and_other_schemes_are_refused() {
    let upgraded = normalize(false, Url::parse("http://example.com/a?b=1").unwrap()).unwrap();
    assert_eq!(upgraded.as_str(), "https://example.com/a?b=1");
    assert!(normalize(false, Url::parse("https://example.com/").unwrap()).is_ok());
    for bad in [
        "ftp://example.com/",
        "file:///etc/passwd",
        "data:text/plain,hi",
    ] {
        assert!(matches!(
            normalize(false, Url::parse(bad).unwrap()),
            Err(Error::InvalidUrl(_))
        ));
    }
}

#[test]
fn private_mode_keeps_plain_http() {
    let kept = normalize(true, Url::parse("http://127.0.0.1:8080/x").unwrap()).unwrap();
    assert_eq!(kept.scheme(), "http");
}

/// A loopback server answering every request (`robots.txt` included) with
/// `reply`, counting the connections it accepts.
async fn loopback(reply: String) -> (Url, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = Url::parse(&format!("http://{}/start", listener.local_addr().unwrap())).unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            counter.fetch_add(1, Ordering::SeqCst);
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;
            let _ = socket.write_all(reply.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });
    (url, hits)
}

fn redirect_to(location: &str) -> String {
    format!(
        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

#[tokio::test]
async fn a_redirect_into_a_private_address_is_refused_on_that_hop() {
    let (secret, secret_hits) =
        loopback("HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nsecret".to_owned()).await;
    let port = secret.port().unwrap();
    let config = WebConfig::default();
    assert!(!config.allow_private_hosts);
    for target in [
        format!("http://127.0.0.1:{port}/admin"),
        format!("http://localhost:{port}/admin"),
        "http://10.0.0.5/".to_owned(),
        "https://169.254.169.254/latest/meta-data".to_owned(),
        "http://[::1]:9/".to_owned(),
    ] {
        let (start, start_hits) = loopback(redirect_to(&target)).await;
        // The first hop is vetted, so the loopback redirector is reached
        // with private hosts refused; only the hops after it are checked.
        let vetted = start.origin();
        let result = fetch_from(&config, start, Some(&vetted)).await;
        assert!(
            matches!(result, Err(Error::PrivateAddress(_))),
            "{target}: {result:?}"
        );
        assert!(
            start_hits.load(Ordering::SeqCst) > 0,
            "the first hop was fetched"
        );
    }
    assert_eq!(
        secret_hits.load(Ordering::SeqCst),
        0,
        "no redirect reached the private address"
    );
}

#[tokio::test]
async fn without_a_vetted_origin_the_first_hop_is_checked_too() {
    let (start, hits) = loopback(redirect_to("https://example.com/")).await;
    let result = fetch_from(&WebConfig::default(), start, None).await;
    assert!(
        matches!(result, Err(Error::PrivateAddress(_))),
        "{result:?}"
    );
    assert_eq!(hits.load(Ordering::SeqCst), 0);
}
