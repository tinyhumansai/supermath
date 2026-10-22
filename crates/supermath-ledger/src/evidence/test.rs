//! The verdict rule: what earns `formalised`, and the scripted fake.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use super::{AXIOM_ALLOWLIST, LeanEvidence, LeanVerdict, LeanVerdictKind};
use crate::error::Error;
use crate::evidence::FakeLeanEvidence;

fn clean() -> LeanVerdict {
    FakeLeanEvidence::clean(
        "sha",
        "A.main",
        &["propext", "Classical.choice", "Quot.sound"],
    )
}

#[test]
fn a_clean_check_inside_the_allowlist_earns_formalised() {
    assert!(clean().earns_formalised("A.main"));
    assert_eq!(AXIOM_ALLOWLIST.len(), 3);
}

#[test]
fn every_way_of_falling_short_is_refused() {
    let cases: Vec<(&str, LeanVerdict)> = vec![
        (
            "sorry verdict",
            LeanVerdict {
                verdict: LeanVerdictKind::CheckedWithSorry,
                ..clean()
            },
        ),
        (
            "failed",
            LeanVerdict {
                verdict: LeanVerdictKind::Failed,
                ..clean()
            },
        ),
        (
            "unknown",
            LeanVerdict {
                verdict: LeanVerdictKind::Unknown,
                ..clean()
            },
        ),
        (
            "sorry count",
            LeanVerdict {
                sorries: 1,
                ..clean()
            },
        ),
        (
            "named debt",
            LeanVerdict {
                debt: vec!["A.main".into()],
                ..clean()
            },
        ),
        (
            "custom axiom",
            FakeLeanEvidence::clean("sha", "A.main", &["propext", "my_axiom"]),
        ),
        (
            "native_decide",
            FakeLeanEvidence::clean("sha", "A.main", &["Lean.ofReduceBool"]),
        ),
        (
            "declaration not reported",
            LeanVerdict {
                axioms: BTreeMap::new(),
                ..clean()
            },
        ),
    ];
    for (name, verdict) in cases {
        assert!(!verdict.earns_formalised("A.main"), "{name}");
    }
}

#[test]
fn a_verdict_for_another_declaration_proves_nothing_about_this_one() {
    assert!(!clean().earns_formalised("A.other"));
}

#[tokio::test]
async fn the_fake_answers_from_its_table_and_counts_calls() {
    let fake = FakeLeanEvidence::new();
    fake.set("a.lean", "A.main", Ok(clean()));
    fake.set("t.lean", "T", Err(Error::EvidenceTimeout));
    fake.set(
        "d.lean",
        "D",
        Err(Error::EvidenceUnavailable("down".into())),
    );
    fake.set(
        "n.lean",
        "N",
        Err(Error::EvidenceNotFound {
            path: "n.lean".into(),
            declaration: "N".into(),
        }),
    );
    assert_eq!(fake.verify("a.lean", "A.main").await.unwrap(), clean());
    assert!(matches!(
        fake.verify("t.lean", "T").await,
        Err(Error::EvidenceTimeout)
    ));
    assert!(matches!(
        fake.verify("d.lean", "D").await,
        Err(Error::EvidenceUnavailable(_))
    ));
    assert!(matches!(
        fake.verify("n.lean", "N").await,
        Err(Error::EvidenceNotFound { .. })
    ));
    assert!(matches!(
        fake.verify("zz.lean", "Z").await,
        Err(Error::EvidenceNotFound { .. })
    ));
    assert_eq!(fake.calls().len(), 5);
    assert_eq!(
        fake.calls()[0],
        ("a.lean".to_string(), "A.main".to_string())
    );
}

#[tokio::test]
async fn the_fake_wraps_a_foreign_error_as_unavailable() {
    let fake = FakeLeanEvidence::new();
    fake.set("a.lean", "A", Err(Error::OriginFromCaller));
    assert!(matches!(
        fake.verify("a.lean", "A").await,
        Err(Error::EvidenceUnavailable(_))
    ));
}
