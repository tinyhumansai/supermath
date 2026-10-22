//! Unit tests for verdict reduction: one test per row of the verdict table.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::*;
use crate::client::{Message, Sorry};

fn target(name: &str) -> Target {
    Target {
        name: name.into(),
        line: 3,
    }
}

fn message(severity: &str, text: &str) -> Message {
    Message {
        severity: severity.into(),
        line: 5,
        column: 2,
        text: text.into(),
    }
}

fn reply_with_axioms(axioms: &[(&str, Option<&[&str]>)]) -> CheckReply {
    CheckReply {
        axioms: axioms
            .iter()
            .map(|(name, list)| {
                (
                    (*name).to_owned(),
                    list.map(|l| l.iter().map(|a| (*a).to_owned()).collect()),
                )
            })
            .collect(),
        ..CheckReply::default()
    }
}

#[test]
fn a_clean_proof_is_checked() {
    let reply = reply_with_axioms(&[("t", Some(&["propext", "Classical.choice", "Quot.sound"]))]);
    let reduction = reduce(&reply, &[target("t")]);
    assert_eq!(reduction.verdict, Verdict::Checked);
    assert!(reduction.debt.is_empty());
    assert!(reduction.errors.is_empty());
    assert_eq!(reduction.axioms["t"].len(), 3);
}

#[test]
fn a_sorry_is_checked_with_sorry() {
    let mut reply = reply_with_axioms(&[("t", Some(&["sorryAx"]))]);
    reply.sorries.push(Sorry {
        line: 4,
        column: 2,
        goal: "⊢ True".into(),
    });
    reply
        .messages
        .push(message("warning", "declaration uses 'sorry'"));
    let reduction = reduce(&reply, &[target("t")]);
    assert_eq!(reduction.verdict, Verdict::CheckedWithSorry);
}

#[test]
fn an_admit_is_checked_with_sorry_by_its_warning_alone() {
    let mut reply = reply_with_axioms(&[("t", Some(&[]))]);
    reply
        .messages
        .push(message("warning", "declaration uses 'sorry'"));
    assert_eq!(
        reduce(&reply, &[target("t")]).verdict,
        Verdict::CheckedWithSorry
    );
}

#[test]
fn sorry_ax_in_a_dependency_is_checked_with_sorry() {
    let reply = reply_with_axioms(&[("t", Some(&["propext", "sorryAx"]))]);
    assert_eq!(
        reduce(&reply, &[target("t")]).verdict,
        Verdict::CheckedWithSorry
    );
}

#[test]
fn an_error_is_failed() {
    let mut reply = reply_with_axioms(&[]);
    reply
        .messages
        .push(message("error", "unknown identifier 'x'"));
    reply.messages.push(message("warning", "unused variable"));
    let reduction = reduce(&reply, &[]);
    assert_eq!(reduction.verdict, Verdict::Failed);
    assert_eq!(reduction.error_count, 1);
    assert_eq!(reduction.errors[0].line, 5);
    assert_eq!(reduction.errors[0].code, None);
}

#[test]
fn a_custom_axiom_is_failed_with_untrusted_axiom() {
    let reply = reply_with_axioms(&[("t", Some(&["propext", "Foo.myAxiom"]))]);
    let reduction = reduce(&reply, &[target("t")]);
    assert_eq!(reduction.verdict, Verdict::Failed);
    assert_eq!(reduction.errors[0].code.as_deref(), Some(UNTRUSTED_AXIOM));
    assert_eq!(reduction.errors[0].line, 3);
    assert!(reduction.errors[0].message.contains("Foo.myAxiom"));
}

#[test]
fn native_decide_is_checked_with_named_debt() {
    let reply = reply_with_axioms(&[
        ("t", Some(&["propext", "Lean.ofReduceBool"])),
        ("u", Some(&["Lean.trustCompiler"])),
        ("v", Some(&[])),
    ]);
    let reduction = reduce(&reply, &[target("t"), target("u"), target("v")]);
    assert_eq!(reduction.verdict, Verdict::Checked);
    assert_eq!(reduction.debt, ["t", "u"]);
}

#[test]
fn each_cut_off_is_unknown_with_its_reason() {
    for (status, reason) in [
        (Status::Timeout, UnknownReason::Timeout),
        (Status::Memory, UnknownReason::Memory),
        (Status::OutputLimit, UnknownReason::OutputLimit),
        (Status::Other, UnknownReason::Other),
    ] {
        let reply = CheckReply {
            status,
            ..CheckReply::default()
        };
        let reduction = reduce(&reply, &[target("t")]);
        assert_eq!(reduction.verdict, Verdict::Unknown);
        assert_eq!(reduction.unknown, Some(reason));
        assert!(!reason.describe().is_empty());
    }
}

#[test]
fn a_missing_target_is_listed_not_reduced() {
    let reply = reply_with_axioms(&[("t", None)]);
    let reduction = reduce(&reply, &[target("t"), target("absent")]);
    assert_eq!(reduction.missing, ["t", "absent"]);
    assert_eq!(reduction.verdict, Verdict::Checked);
}

#[test]
fn errors_are_capped_at_twenty_and_messages_at_four_hundred_characters() {
    let mut reply = CheckReply::default();
    for _ in 0..25 {
        reply.messages.push(message("error", &"x".repeat(500)));
    }
    let reduction = reduce(&reply, &[]);
    assert_eq!(reduction.errors.len(), MAX_ERRORS);
    assert_eq!(reduction.error_count, 25);
    assert_eq!(reduction.errors[0].message.chars().count(), 401);
    assert!(reduction.errors[0].message.ends_with('…'));
}

#[test]
fn verdicts_serialise_to_their_wire_names() {
    for verdict in [
        Verdict::Checked,
        Verdict::CheckedWithSorry,
        Verdict::Failed,
        Verdict::Unknown,
    ] {
        assert_eq!(
            serde_json::to_value(verdict).unwrap(),
            serde_json::json!(verdict.as_str())
        );
    }
}
