//! Tests for the response cache, on a fake clock.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use super::ResponseCache;
use crate::ManualClock;

fn cache(dir: &tempfile::TempDir) -> (ResponseCache, Arc<ManualClock>) {
    let clock = Arc::new(ManualClock::new(10_000));
    (
        ResponseCache::new(Some(dir.path().join("lit")), clock.clone()),
        clock,
    )
}

#[tokio::test]
async fn a_stored_body_comes_back_with_its_age() {
    let dir = tempfile::tempdir().unwrap();
    let (cache, clock) = cache(&dir);
    cache.put("http://x/a", b"body").await;
    clock.advance(2_500);
    let hit = cache.get("http://x/a", None).await.unwrap();
    assert_eq!(hit.body, b"body");
    assert_eq!(hit.age_ms, 2_500);
}

#[tokio::test]
async fn a_body_older_than_the_limit_is_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let (cache, clock) = cache(&dir);
    cache.put("http://x/a", b"body").await;
    clock.advance(1_001);
    assert!(cache.get("http://x/a", Some(1_000)).await.is_none());
    assert!(cache.get("http://x/a", Some(1_001)).await.is_some());
}

#[tokio::test]
async fn an_unknown_url_is_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let (cache, _) = cache(&dir);
    assert!(cache.get("http://x/missing", None).await.is_none());
}

#[tokio::test]
async fn a_disabled_cache_stores_and_returns_nothing() {
    let cache = ResponseCache::new(None, Arc::new(ManualClock::new(0)));
    cache.put("http://x/a", b"body").await;
    assert!(cache.get("http://x/a", None).await.is_none());
}

#[tokio::test]
async fn a_truncated_file_is_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let (cache, _) = cache(&dir);
    cache.put("http://x/a", b"body").await;
    let entry = std::fs::read_dir(dir.path().join("lit"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::write(entry, [1, 2, 3]).unwrap();
    assert!(cache.get("http://x/a", None).await.is_none());
}

#[tokio::test]
async fn an_unwritable_directory_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, "a file, not a directory").unwrap();
    let cache = ResponseCache::new(Some(blocker.join("lit")), Arc::new(ManualClock::new(0)));
    cache.put("http://x/a", b"body").await;
    assert!(cache.get("http://x/a", None).await.is_none());
}

#[tokio::test]
async fn a_failed_rename_leaves_no_partial_file() {
    let dir = tempfile::tempdir().unwrap();
    let (cache, _) = cache(&dir);
    cache.put("http://x/a", b"body").await;
    let entry = std::fs::read_dir(dir.path().join("lit"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::remove_file(&entry).unwrap();
    // A directory squatting on the final name makes the rename fail.
    std::fs::create_dir(&entry).unwrap();
    cache.put("http://x/a", b"body").await;
    let leftovers: Vec<_> = std::fs::read_dir(dir.path().join("lit"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "partial"))
        .collect();
    assert_eq!(leftovers.len(), 0);
}
