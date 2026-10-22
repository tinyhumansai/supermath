//! Tests for projects, evidence origin, cancellation, and the context.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use serde_json::json;

use super::{Cancellation, EvidenceOrigin};
use crate::{Budget, CallContext, Error, Project};

fn project() -> Project {
    Project::new("twin-primes", "ws:supermath/project:twin-primes").unwrap()
}

#[test]
fn project_exposes_slug_and_scope_and_has_a_pinned_form() {
    let project = project();
    assert_eq!(project.slug(), "twin-primes");
    assert_eq!(project.scope(), "ws:supermath/project:twin-primes");
    assert_eq!(
        serde_json::to_value(&project).unwrap(),
        json!({"slug": "twin-primes", "scope": "ws:supermath/project:twin-primes"})
    );
}

#[test]
fn rejects_bad_slugs_and_empty_scopes() {
    for slug in ["", "Twin", "has space", "a/b", "a_b"] {
        assert_eq!(
            Project::new(slug, "s").unwrap_err(),
            Error::InvalidProjectSlug(slug.into())
        );
    }
    assert_eq!(
        Project::new("ok", "").unwrap_err(),
        Error::EmptyProjectScope
    );
}

#[test]
fn evidence_origin_is_snake_case() {
    assert_eq!(
        serde_json::to_value(EvidenceOrigin::Collected).unwrap(),
        json!("collected")
    );
    assert_eq!(
        serde_json::to_value(EvidenceOrigin::Supplied).unwrap(),
        json!("supplied")
    );
}

#[test]
fn context_defaults_to_collected_evidence_and_the_default_budget() {
    let ctx = CallContext::new(project(), "c1");
    assert_eq!(ctx.call_id, "c1");
    assert_eq!(ctx.evidence_origin, EvidenceOrigin::Collected);
    assert_eq!(ctx.budget, Budget::default());
    let ctx = ctx
        .with_evidence_origin(EvidenceOrigin::Supplied)
        .with_budget(Budget::agentic(2, 10));
    assert_eq!(ctx.evidence_origin, EvidenceOrigin::Supplied);
    assert_eq!(ctx.budget.iterations, Some(2));
}

#[test]
fn cancelling_one_clone_cancels_all() {
    let a = Cancellation::default();
    let b = a.clone();
    assert!(!b.is_cancelled());
    a.cancel();
    assert!(b.is_cancelled());
}

#[tokio::test]
async fn cancelled_completes_after_cancel_and_when_already_cancelled() {
    let signal = Cancellation::new();
    let waiter = signal.clone();
    let handle = tokio::spawn(async move { waiter.cancelled().await });
    tokio::task::yield_now().await;
    signal.cancel();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .unwrap()
        .unwrap();
    // Already fired: returns at once.
    tokio::time::timeout(Duration::from_secs(5), signal.cancelled())
        .await
        .unwrap();
}
