//! Tests for the child descriptions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::ChildKind;
use crate::fake::project;
use crate::limits::Limits;

#[test]
fn each_child_has_its_own_tool_suffix_and_budget() {
    let rows: Vec<_> = ChildKind::ALL
        .iter()
        .map(|kind| {
            let spec = kind.spec();
            (spec.tool, spec.suffix, spec.defaults)
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("agent_digest_paper", "reader", Limits::DEFAULT),
            ("agent_formalize", "formalizer", Limits::DEFAULT),
            ("agent_review_alignment", "reviewer", Limits::REVIEW),
            ("agent_refute", "refuter", Limits::DEFAULT),
        ]
    );
}

#[test]
fn agent_ids_are_per_project_and_per_child() {
    let id = ChildKind::Formalizer.spec().agent_id(&project());
    assert_eq!(id, "supermath-demo-formalizer");
}

#[test]
fn no_child_may_call_an_agent_tool_or_memory() {
    for kind in ChildKind::ALL {
        let spec = kind.spec();
        for name in spec.required() {
            assert!(!name.starts_with("agent_"), "{} lists {name}", spec.tool);
            assert_ne!(name, "memory");
        }
    }
}

#[test]
fn the_reviewer_never_sees_the_proof() {
    let spec = ChildKind::Reviewer.spec();
    assert_eq!(
        spec.primitives,
        ["lean_decl", "lean_search", "lit_get_paper", "recall_memory"]
    );
    for forbidden in [
        "file_read",
        "file_list",
        "lean_goals",
        "file_write",
        "lean_check",
    ] {
        assert!(!spec.primitives.contains(&forbidden), "{forbidden}");
    }
}

#[test]
fn only_the_formalizer_and_refuter_may_write_files() {
    for kind in ChildKind::ALL {
        let spec = kind.spec();
        let writes = spec.primitives.contains(&"file_write");
        assert_eq!(
            writes,
            matches!(kind, ChildKind::Formalizer | ChildKind::Refuter),
            "{}",
            spec.tool
        );
    }
}

#[test]
fn only_the_tool_records_for_the_formalizer_and_the_refuter() {
    for kind in [
        ChildKind::Formalizer,
        ChildKind::Refuter,
        ChildKind::Reviewer,
    ] {
        let spec = kind.spec();
        assert!(!spec.primitives.contains(&"ledger_record"), "{}", spec.tool);
        assert!(spec.verifies.contains(&"ledger_record"), "{}", spec.tool);
    }
    assert!(
        ChildKind::Reader
            .spec()
            .primitives
            .contains(&"ledger_record")
    );
}

#[test]
fn every_prompt_names_the_reply_the_tool_parses() {
    let marker = |kind: ChildKind| kind.spec().prompt;
    assert!(marker(ChildKind::Formalizer).contains("NOTE:"));
    assert!(marker(ChildKind::Reviewer).contains("VERDICT: accepted"));
    assert!(marker(ChildKind::Reviewer).contains("MISMATCH:"));
    assert!(marker(ChildKind::Refuter).contains("REPAIRED:"));
    assert!(marker(ChildKind::Reader).contains("120 words"));
}

// ---------------------------------------------------------------- control

mod control {
    use super::super::{ControlSignal, TurnControl};

    #[tokio::test]
    async fn steers_come_out_in_the_order_they_were_queued() {
        let control = TurnControl::new();
        control.steer("first");
        control.steer("second");
        assert_eq!(control.next().await, ControlSignal::Steer("first".into()));
        assert_eq!(control.next().await, ControlSignal::Steer("second".into()));
        assert!(control.take_steers().is_empty());
    }

    #[tokio::test]
    async fn a_cancel_comes_before_queued_steers_and_stays() {
        let control = TurnControl::new();
        control.steer("never applied");
        control.cancel();
        assert!(control.is_cancelled());
        assert_eq!(control.next().await, ControlSignal::Cancel);
        assert_eq!(control.next().await, ControlSignal::Cancel);
        assert_eq!(control.take_steers(), ["never applied"]);
    }

    #[tokio::test]
    async fn a_waiting_runner_wakes_on_a_steer_from_another_task() {
        let control = TurnControl::new();
        let host = control.clone();
        let waiter = tokio::spawn(async move { control.next().await });
        tokio::task::yield_now().await;
        host.steer("look at odd n");
        assert_eq!(
            waiter.await.unwrap(),
            ControlSignal::Steer("look at odd n".into())
        );
    }

    #[tokio::test]
    async fn a_waiting_runner_wakes_on_a_cancel() {
        let control = TurnControl::new();
        let host = control.clone();
        let waiter = tokio::spawn(async move { control.next().await });
        tokio::task::yield_now().await;
        host.cancel();
        assert_eq!(waiter.await.unwrap(), ControlSignal::Cancel);
    }
}
