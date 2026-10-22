//! The entry's wire form, pinned: a host and a stored entry that disagree
//! about a field name fail at runtime.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;
use supermath_tool::EvidenceOrigin;

use super::{Alignment, BoundSpec, Direction, Entry, Kind, LeanClaim, Source, Status};
use crate::evidence::LeanVerdictKind;

fn entry() -> Entry {
    Entry {
        id: "clm_7f3a".into(),
        revision: 1,
        kind: Kind::Result,
        statement: "every polynomial of degree n".into(),
        status: Status::Cited,
        evidence_origin: EvidenceOrigin::Collected,
        source: Source::Arxiv {
            id: "2012.04125".into(),
            version: "v3".into(),
            label: Some("thm:main".into()),
            section: None,
        },
        lean: None,
        alignment: Alignment::NotReviewed,
        uses: vec![],
        supersedes: None,
        rationale: None,
        reason: None,
        refutes: None,
        frame: None,
        bound: None,
    }
}

#[test]
fn an_entry_serialises_to_the_documented_form() {
    assert_eq!(
        serde_json::to_value(entry()).unwrap(),
        json!({
            "id": "clm_7f3a",
            "revision": 1,
            "kind": "result",
            "statement": "every polynomial of degree n",
            "status": "cited",
            "evidence_origin": "collected",
            "source": {"arxiv": {"id": "2012.04125", "version": "v3", "label": "thm:main"}},
            "lean": null,
            "alignment": "not_reviewed",
            "uses": [],
            "supersedes": null
        })
    );
}

#[test]
fn every_source_has_its_own_wire_form() {
    let cases = [
        (
            Source::Lean {
                path: "lean/A.lean".into(),
                declaration: "A.main".into(),
                file_sha256: "ab".into(),
            },
            json!({"lean": {"path": "lean/A.lean", "declaration": "A.main", "file_sha256": "ab"}}),
        ),
        (
            Source::Tool {
                tool: "py_run".into(),
                call_id: "c1".into(),
                args_digest: Some("d".into()),
            },
            json!({"tool": {"tool": "py_run", "call_id": "c1", "args_digest": "d"}}),
        ),
        (
            Source::Web {
                url: "https://x".into(),
                title: "t".into(),
                class: "forum".into(),
                retrieved_at: "2026-01-01".into(),
            },
            json!({"web": {"url": "https://x", "title": "t", "class": "forum", "retrieved_at": "2026-01-01"}}),
        ),
        (Source::Supplied("text".into()), json!({"supplied": "text"})),
    ];
    for (source, wire) in cases {
        assert_eq!(serde_json::to_value(&source).unwrap(), wire);
        assert_eq!(serde_json::from_value::<Source>(wire).unwrap(), source);
    }
}

#[test]
fn a_full_entry_round_trips() {
    let mut full = entry();
    full.lean = Some(LeanClaim {
        path: "lean/A.lean".into(),
        declaration: "A.main".into(),
        file_sha256: "ab".into(),
        verdict: LeanVerdictKind::CheckedWithSorry,
        debt: vec!["A.main".into()],
        provenance: json!({"toolchain": "v4"}),
    });
    full.rationale = Some("why".into());
    full.reason = Some("because".into());
    full.refutes = Some("clm_1".into());
    full.frame = Some("n <= 100".into());
    full.supersedes = Some("clm_7f3a#1".into());
    full.bound = Some(BoundSpec {
        quantity: "q".into(),
        direction: Direction::Upper,
        value: 3.into(),
    });
    let wire = serde_json::to_string(&full).unwrap();
    assert!(wire.contains("\"verdict\":\"checked_with_sorry\""));
    assert_eq!(serde_json::from_str::<Entry>(&wire).unwrap(), full);
}

#[test]
fn statuses_are_ordered_by_the_evidence_they_need() {
    assert!(Status::Asserted < Status::Cited);
    assert!(Status::Cited < Status::Checked);
    assert!(Status::Checked < Status::Formalised);
}

#[test]
fn wire_names_match_the_labels() {
    let kinds = [
        (Kind::Fact, "fact"),
        (Kind::Result, "result"),
        (Kind::Intake, "intake"),
        (Kind::Bound, "bound"),
        (Kind::Lead, "lead"),
        (Kind::DeadEnd, "dead_end"),
        (Kind::Refutation, "refutation"),
    ];
    for (kind, name) in kinds {
        assert_eq!(kind.as_str(), name);
        assert_eq!(serde_json::to_value(kind).unwrap(), json!(name));
    }
    for status in [
        Status::Asserted,
        Status::Cited,
        Status::Checked,
        Status::Formalised,
    ] {
        assert_eq!(
            serde_json::to_value(status).unwrap(),
            json!(status.as_str())
        );
    }
}

#[test]
fn an_entry_is_labelled_with_its_kind_status_and_id() {
    assert_eq!(
        entry().labels(),
        ["kind:result", "status:cited", "claim:clm_7f3a"]
    );
}
