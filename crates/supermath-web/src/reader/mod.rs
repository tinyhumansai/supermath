//! Fetching a page for `web_read`, safely.
//!
//! A fetch refuses private, loopback, and link-local addresses after DNS
//! resolution and pins the connection to the addresses it checked, so a
//! page (or a redirect, or a rebinding DNS answer) cannot point the tool at
//! the operator's network. It honours `robots.txt`, follows at most five
//! redirects, and reads at most 5 MB. arXiv is refused on every hop, not
//! only the first: its papers are `lit_get_paper`'s to read.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, LOCATION, RETRY_AFTER};
use url::{Host, Origin, Url};

use crate::config::WebConfig;
use crate::error::{Error, Result};
use crate::robots::Robots;
use crate::source::arxiv_id;

/// Largest page read, in bytes.
pub(crate) const MAX_BYTES: u64 = 5 * 1024 * 1024;
/// Largest `robots.txt` read, in bytes.
const MAX_ROBOTS_BYTES: u64 = 512 * 1024;
/// Most redirects followed.
const MAX_REDIRECTS: usize = 5;

/// A fetched page.
#[derive(Debug)]
pub(crate) struct Page {
    /// The URL the body came from, after redirects.
    pub(crate) url: Url,
    pub(crate) body: String,
    pub(crate) html: bool,
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        || a == 0
        || (a == 100 && (64..128).contains(&b))
        || a >= 240)
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_v4(v4);
    }
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_unique_local()
        || ip.is_unicast_link_local())
}

/// Whether `ip` is a public internet address.
pub(crate) fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

/// Resolves the host of `url` and checks every address is public (unless
/// `private_ok`).
async fn resolve(private_ok: bool, url: &Url) -> Result<Vec<SocketAddr>> {
    let port = url
        .port_or_known_default()
        .ok_or_else(|| Error::InvalidUrl(url.to_string()))?;
    let addrs: Vec<SocketAddr> = match url.host() {
        Some(Host::Ipv4(ip)) => vec![SocketAddr::new(ip.into(), port)],
        Some(Host::Ipv6(ip)) => vec![SocketAddr::new(ip.into(), port)],
        Some(Host::Domain(domain)) => tokio::net::lookup_host((domain, port))
            .await
            .map_err(|_| Error::Unavailable(format!("cannot resolve {domain}")))?
            .collect(),
        None => return Err(Error::InvalidUrl(url.to_string())),
    };
    if addrs.is_empty() {
        return Err(Error::Unavailable("the host has no addresses".to_owned()));
    }
    if !private_ok && addrs.iter().any(|a| !is_public(a.ip())) {
        return Err(Error::PrivateAddress(
            url.host_str().unwrap_or_default().to_owned(),
        ));
    }
    Ok(addrs)
}

/// [`Error::ArxivPaper`] when `url` is on arxiv.org or one of its
/// subdomains, naming the paper when the URL does.
pub(crate) fn refuse_arxiv(url: &Url) -> Result<()> {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if host == "arxiv.org" || host.ends_with(".arxiv.org") {
        return Err(Error::ArxivPaper {
            arxiv_id: arxiv_id(url),
        });
    }
    Ok(())
}

/// Checks the scheme, upgrading `http` to `https` unless `private_ok`.
pub(crate) fn normalize(private_ok: bool, mut url: Url) -> Result<Url> {
    match url.scheme() {
        "https" => {}
        "http" if private_ok => {}
        "http" => {
            url.set_scheme("https")
                .map_err(|()| Error::InvalidUrl(url.to_string()))?;
        }
        _ => {
            return Err(Error::InvalidUrl(
                "only http and https urls are read".to_owned(),
            ));
        }
    }
    if url.host_str().is_none() {
        return Err(Error::InvalidUrl("the url has no host".to_owned()));
    }
    Ok(url)
}

async fn read_limited(mut response: reqwest::Response, limit: u64) -> Result<Vec<u8>> {
    if response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|len| len > limit)
    {
        return Err(Error::TooLarge { limit });
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| Error::Unavailable(e.without_url().to_string()))?
    {
        if body.len() as u64 + chunk.len() as u64 > limit {
            return Err(Error::TooLarge { limit });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn request_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout
    } else {
        Error::Unavailable(error.without_url().to_string())
    }
}

async fn robots_allows(client: &reqwest::Client, config: &WebConfig, url: &Url) -> Result<bool> {
    let Ok(robots_url) = url.join("/robots.txt") else {
        return Ok(true);
    };
    let Ok(response) = client.get(robots_url).send().await else {
        return Ok(true);
    };
    let status = response.status();
    if status.is_server_error() {
        return Err(Error::Forbidden("robots.txt is unavailable".to_owned()));
    }
    if !status.is_success() {
        return Ok(true);
    }
    let Ok(body) = read_limited(response, MAX_ROBOTS_BYTES).await else {
        return Ok(true);
    };
    let robots = Robots::parse(&String::from_utf8_lossy(&body), &config.user_agent());
    let target = match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_owned(),
    };
    Ok(robots.allows(&target))
}

/// What one request returned.
enum Hop {
    Redirect(Url),
    Page(Page),
}

async fn hop(config: &WebConfig, private_ok: bool, url: Url) -> Result<Hop> {
    let addrs = resolve(private_ok, &url).await?;
    let mut builder = reqwest::Client::builder()
        .user_agent(config.user_agent())
        .redirect(reqwest::redirect::Policy::none());
    if let Some(host) = url.host_str() {
        builder = builder.resolve_to_addrs(host, &addrs);
    }
    let client = builder
        .build()
        .map_err(|e| Error::Client(e.without_url().to_string()))?;

    if !robots_allows(&client, config, &url).await? {
        return Err(Error::Forbidden(
            "robots.txt disallows this path".to_owned(),
        ));
    }
    let response = client
        .get(url.clone())
        .header("accept", "text/html, text/plain;q=0.9")
        .send()
        .await
        .map_err(request_error)?;
    let status = response.status();
    if status.is_redirection() {
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(Error::DeadEnd(status.as_u16()))?;
        let next = url
            .join(location)
            .map_err(|_| Error::InvalidUrl(location.to_owned()))?;
        return Ok(Hop::Redirect(next));
    }
    match status.as_u16() {
        404 | 410 => return Err(Error::NotFound),
        401 | 403 => return Err(Error::Forbidden(format!("the site answered {status}"))),
        429 => {
            let retry_after_ms = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map_or(30_000, |secs| secs.saturating_mul(1000));
            return Err(Error::RateLimited { retry_after_ms });
        }
        _ if !status.is_success() => {
            return Err(Error::Unavailable(format!("the site answered {status}")));
        }
        _ => {}
    }

    let mime = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
        })
        .unwrap_or_default();
    let html = match mime.as_str() {
        "" | "text/html" | "application/xhtml+xml" => true,
        "text/plain" => false,
        "application/pdf" => {
            return Err(Error::UnsupportedContent(
                "PDFs are not read in this phase; find an HTML or arXiv version".to_owned(),
            ));
        }
        other => return Err(Error::UnsupportedContent(format!("content type {other}"))),
    };
    let body = read_limited(response, MAX_BYTES).await?;
    Ok(Hop::Page(Page {
        url,
        body: String::from_utf8_lossy(&body).into_owned(),
        html,
    }))
}

/// Fetches `url`, following at most five redirects and checking every hop.
///
/// # Errors
///
/// - [`Error::InvalidUrl`], [`Error::PrivateAddress`], [`Error::ArxivPaper`],
///   [`Error::UnsupportedContent`], [`Error::TooLarge`] for input the tool
///   will not read; the address and arXiv checks run on every hop.
/// - [`Error::NotFound`], [`Error::Forbidden`] (also for `robots.txt`),
///   [`Error::RateLimited`] for the site's answer.
/// - [`Error::Unavailable`] for any other failure, [`Error::TooManyRedirects`]
///   after five redirects.
pub(crate) async fn fetch(config: &WebConfig, url: Url) -> Result<Page> {
    fetch_from(config, url, None).await
}

/// [`fetch`], with `vetted` an origin the caller has already checked: a hop
/// on exactly that origin keeps its scheme and skips the address check, as
/// if private hosts were allowed. Every other hop is checked as usual, so a
/// vetted origin cannot redirect anywhere the config would refuse. [`fetch`]
/// vets nothing; the tests vet their loopback server to prove the checks
/// run on the hops after it.
pub(crate) async fn fetch_from(
    config: &WebConfig,
    url: Url,
    vetted: Option<&Origin>,
) -> Result<Page> {
    let private_ok = |url: &Url| config.allow_private_hosts || vetted == Some(&url.origin());
    let mut current = normalize(private_ok(&url), url)?;
    for _ in 0..=MAX_REDIRECTS {
        refuse_arxiv(&current)?;
        match hop(config, private_ok(&current), current).await? {
            Hop::Page(page) => return Ok(page),
            Hop::Redirect(next) => current = normalize(private_ok(&next), next)?,
        }
    }
    Err(Error::TooManyRedirects)
}

#[cfg(test)]
mod test;
