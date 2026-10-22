//! Grounding redirect resolution against a loopback redirect chain.

use std::time::Duration;

use supermath_web::{Error, MAX_REDIRECTS, redirect_client, resolve_redirect};

use crate::support::{Response, Stub, redirect_route};

const T: Duration = Duration::from_secs(5);

async fn stub() -> Stub {
    Stub::start(|request| redirect_route(&request.path).unwrap_or_else(|| Response::status(404)))
        .await
}

fn hosts() -> Vec<String> {
    vec!["127.0.0.1".to_owned()]
}

async fn resolve(stub: &Stub, path: &str) -> Result<String, Error> {
    let client = redirect_client().unwrap();
    resolve_redirect(&client, &hosts(), &format!("{}{path}", stub.base), T)
        .await
        .map(|url| url.to_string())
}

#[tokio::test]
async fn follows_a_chain_to_the_real_url_without_requesting_it() {
    let stub = stub().await;
    assert_eq!(
        resolve(&stub, "/r/blog").await.unwrap(),
        "https://terrytao.wordpress.com/2019/10/01/sunflowers/"
    );
    // Two HEADs against the redirect host, nothing else.
    let requests = stub.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|r| r.method == "HEAD"));
}

#[tokio::test]
async fn a_uri_on_another_host_is_already_direct() {
    let stub = stub().await;
    let client = redirect_client().unwrap();
    let url = resolve_redirect(&client, &hosts(), "https://example.com/a", T)
        .await
        .unwrap();
    assert_eq!(url.as_str(), "https://example.com/a");
    assert!(stub.requests().is_empty());
}

#[tokio::test]
async fn five_redirects_are_allowed() {
    assert_eq!(MAX_REDIRECTS, 5);
    let stub = stub().await;
    assert_eq!(
        resolve(&stub, "/r/five1").await.unwrap(),
        "https://example.com/final"
    );
}

#[tokio::test]
async fn a_sixth_redirect_is_too_many() {
    let stub = stub().await;
    assert_eq!(
        resolve(&stub, "/r/six1").await.unwrap_err(),
        Error::TooManyRedirects
    );
}

#[tokio::test]
async fn a_loop_is_detected() {
    let stub = stub().await;
    assert_eq!(
        resolve(&stub, "/r/loop1").await.unwrap_err(),
        Error::RedirectLoop
    );
}

#[tokio::test]
async fn a_dead_end_is_reported_with_its_status() {
    let stub = stub().await;
    assert_eq!(
        resolve(&stub, "/r/dead").await.unwrap_err(),
        Error::DeadEnd(404)
    );
    assert_eq!(
        resolve(&stub, "/r/plain").await.unwrap_err(),
        Error::DeadEnd(200)
    );
    assert_eq!(
        resolve(&stub, "/r/nolocation").await.unwrap_err(),
        Error::DeadEnd(302)
    );
}

#[tokio::test]
async fn a_malformed_uri_or_location_is_invalid() {
    let stub = stub().await;
    assert!(matches!(
        resolve(&stub, "/r/badlocation").await,
        Err(Error::InvalidUrl(_))
    ));
    let client = redirect_client().unwrap();
    assert!(matches!(
        resolve_redirect(&client, &hosts(), "not a url", T).await,
        Err(Error::InvalidUrl(_))
    ));
}

#[tokio::test]
async fn a_slow_redirect_host_times_out() {
    let slow = Stub::start(|_| {
        Response::status(302)
            .with_header("location", "https://example.com/")
            .with_delay(Duration::from_millis(500))
    })
    .await;
    let client = redirect_client().unwrap();
    let uri = format!("{}/r/x", slow.base);
    assert_eq!(
        resolve_redirect(&client, &hosts(), &uri, Duration::from_millis(50))
            .await
            .unwrap_err(),
        Error::Timeout
    );
}

#[tokio::test]
async fn an_unreachable_redirect_host_is_unavailable() {
    let client = redirect_client().unwrap();
    let result = resolve_redirect(&client, &hosts(), "http://127.0.0.1:1/r/x", T).await;
    assert!(matches!(result, Err(Error::Unavailable(_))));
}
