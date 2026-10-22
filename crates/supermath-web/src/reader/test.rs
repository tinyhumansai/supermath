//! Tests for the address and scheme checks.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::IpAddr;

use url::Url;

use super::{is_public, normalize};
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
    let config = WebConfig::default();
    let upgraded = normalize(&config, Url::parse("http://example.com/a?b=1").unwrap()).unwrap();
    assert_eq!(upgraded.as_str(), "https://example.com/a?b=1");
    assert!(normalize(&config, Url::parse("https://example.com/").unwrap()).is_ok());
    for bad in [
        "ftp://example.com/",
        "file:///etc/passwd",
        "data:text/plain,hi",
    ] {
        assert!(matches!(
            normalize(&config, Url::parse(bad).unwrap()),
            Err(Error::InvalidUrl(_))
        ));
    }
}

#[test]
fn private_mode_keeps_plain_http() {
    let mut config = WebConfig::default();
    config.allow_private_hosts = true;
    let kept = normalize(&config, Url::parse("http://127.0.0.1:8080/x").unwrap()).unwrap();
    assert_eq!(kept.scheme(), "http");
}
