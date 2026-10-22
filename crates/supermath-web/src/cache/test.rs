//! Tests for the response cache.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::{ResponseCache, TTL_SECS};

#[tokio::test]
async fn returns_a_fresh_entry_with_its_age() {
    let dir = tempfile::tempdir().unwrap();
    let cache = ResponseCache::new(dir.path().join("web"));
    let key = ResponseCache::key("m", "p");
    cache.put(&key, &json!({"a": 1}), 1000).await;
    let hit = cache.get(&key, 1090).await.unwrap();
    assert_eq!(hit.response, json!({"a": 1}));
    assert_eq!(hit.age_secs, 90);
}

#[tokio::test]
async fn an_entry_a_day_old_is_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let cache = ResponseCache::new(dir.path());
    let key = ResponseCache::key("m", "p");
    cache.put(&key, &json!(1), 0).await;
    assert!(cache.get(&key, TTL_SECS - 1).await.is_some());
    assert!(cache.get(&key, TTL_SECS).await.is_none());
}

#[tokio::test]
async fn a_corrupt_or_missing_entry_is_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let cache = ResponseCache::new(dir.path());
    let key = ResponseCache::key("m", "p");
    assert!(cache.get(&key, 0).await.is_none());
    tokio::fs::write(dir.path().join(format!("{key}.json")), b"not json")
        .await
        .unwrap();
    assert!(cache.get(&key, 0).await.is_none());
}

#[tokio::test]
async fn an_unwritable_directory_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("occupied");
    tokio::fs::write(&file, b"x").await.unwrap();
    let cache = ResponseCache::new(file.join("sub"));
    cache.put("k", &json!(1), 0).await;
    assert!(cache.get("k", 0).await.is_none());
}

#[test]
fn the_key_depends_on_model_and_prompt() {
    let a = ResponseCache::key("m1", "p");
    assert_eq!(a, ResponseCache::key("m1", "p"));
    assert_ne!(a, ResponseCache::key("m2", "p"));
    assert_ne!(a, ResponseCache::key("m1", "q"));
    assert_ne!(ResponseCache::key("ab", "c"), ResponseCache::key("a", "bc"));
}
