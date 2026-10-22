//! Resolving Google's grounding redirects to the real source URL.
//!
//! Grounding URIs arrive as `vertexaisearch.cloud.google.com` redirects. The
//! destination is the `Location` of the redirect, so the resolver asks the
//! redirect host with a `HEAD` request (no body) and reads where it points,
//! at most five hops. It never requests the destination itself: a source URL
//! is reported, not fetched, and the grounded pages stay unvisited.

use std::collections::HashSet;
use std::time::Duration;

use reqwest::header::LOCATION;
use url::Url;

use crate::error::{Error, Result};

/// Most redirects followed before giving up.
pub const MAX_REDIRECTS: usize = 5;

/// The HTTP client a resolver needs: one that does not follow redirects.
///
/// # Errors
///
/// [`Error::Client`] if the client cannot be built.
pub fn redirect_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| Error::Client(e.without_url().to_string()))
}

fn is_redirect_host(url: &Url, redirect_hosts: &[String]) -> bool {
    url.host_str()
        .is_some_and(|host| redirect_hosts.iter().any(|r| r.eq_ignore_ascii_case(host)))
}

/// Resolves a grounding URI to the URL it redirects to.
///
/// `client` must not follow redirects ([`redirect_client`]). A URI whose host
/// is not in `redirect_hosts` is already direct and comes back unchanged. For
/// a redirect host, each hop is a `HEAD` request whose `Location` is the
/// next URL; the chain ends at the first URL on a different host, which is
/// returned without being requested. The whole resolution must finish within
/// `timeout`.
///
/// # Errors
///
/// - [`Error::InvalidUrl`] if `uri` or a `Location` is not an absolute URL.
/// - [`Error::TooManyRedirects`] after [`MAX_REDIRECTS`] hops.
/// - [`Error::RedirectLoop`] if the chain revisits a URL.
/// - [`Error::DeadEnd`] if a redirect host answers without a `Location`
///   (a 404, a 5xx, or a plain 200).
/// - [`Error::Unavailable`] if the request fails, [`Error::Timeout`] if it
///   exceeds `timeout`.
pub async fn resolve_redirect(
    client: &reqwest::Client,
    redirect_hosts: &[String],
    uri: &str,
    timeout: Duration,
) -> Result<Url> {
    let start = Url::parse(uri).map_err(|_| Error::InvalidUrl(uri.to_owned()))?;
    tokio::time::timeout(timeout, follow(client, redirect_hosts, start))
        .await
        .map_err(|_| Error::Timeout)?
}

async fn follow(client: &reqwest::Client, redirect_hosts: &[String], start: Url) -> Result<Url> {
    let mut seen = HashSet::new();
    let mut current = start;
    for _ in 0..MAX_REDIRECTS {
        if !is_redirect_host(&current, redirect_hosts) {
            return Ok(current);
        }
        if !seen.insert(current.clone()) {
            return Err(Error::RedirectLoop);
        }
        let response = client
            .head(current.clone())
            .send()
            .await
            .map_err(|e| Error::Unavailable(e.without_url().to_string()))?;
        let status = response.status();
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .filter(|_| status.is_redirection());
        let Some(location) = location else {
            return Err(Error::DeadEnd(status.as_u16()));
        };
        current = current
            .join(location)
            .map_err(|_| Error::InvalidUrl(location.to_owned()))?;
    }
    if is_redirect_host(&current, redirect_hosts) {
        Err(Error::TooManyRedirects)
    } else {
        Ok(current)
    }
}
