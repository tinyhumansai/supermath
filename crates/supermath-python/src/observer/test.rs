//! Tests for the Python run log.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use supermath_files::{FileObserver, Language, ProjectPath};

use super::PythonObserver;

fn path(raw: &str) -> ProjectPath {
    ProjectPath::parse(raw).unwrap()
}

#[test]
fn reports_python_files() {
    assert_eq!(PythonObserver::new().language(), Language::Python);
}

#[test]
fn a_file_never_run_has_no_result() {
    assert!(
        PythonObserver::new()
            .last_result("demo", &path("python/work/a.py"))
            .is_none()
    );
}

#[test]
fn the_last_record_wins_per_project_and_file() {
    let observer = PythonObserver::new();
    let a = path("python/work/a.py");
    observer.record("demo", &a, "raised", "one");
    observer.record("demo", &a, "returned", "two");
    observer.record("other", &a, "unknown", "three");
    let last = observer.last_result("demo", &a).unwrap();
    assert_eq!(
        (last.result.as_str(), last.sha256.as_str()),
        ("returned", "two")
    );
    assert_eq!(observer.last_result("other", &a).unwrap().result, "unknown");
    assert!(
        observer
            .last_result("demo", &path("python/work/b.py"))
            .is_none()
    );
}

#[test]
fn is_usable_as_a_shared_trait_object() {
    let observer: Arc<dyn FileObserver> = Arc::new(PythonObserver::new());
    assert_eq!(observer.language(), Language::Python);
}

#[test]
fn survives_a_poisoned_lock() {
    let observer = Arc::new(PythonObserver::new());
    let poisoner = observer.clone();
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.results.lock().unwrap();
        panic!("poison the lock");
    })
    .join();
    observer.record("demo", &path("python/work/a.py"), "returned", "x");
    assert!(
        observer
            .last_result("demo", &path("python/work/a.py"))
            .is_some()
    );
}
