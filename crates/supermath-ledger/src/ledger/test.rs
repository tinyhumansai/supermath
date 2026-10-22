//! The ledger against the `CortexDB` stub and a fake Lean: every allowed and
//! refused status transition, effective status, edges, and bounds.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use serde_json::{Number, json};
use supermath_tool::{
    CallContext, CallLog, CallRecord, Envelope, ErrorCode, EvidenceOrigin, InMemoryCallLog, Project,
};

use super::{BoundRequest, Claim, Ledger, compare, improves, latest_revisions};
use crate::error::Error;
use crate::evidence::{FakeLeanEvidence, LeanVerdict, LeanVerdictKind};
use crate::memory::stub::FakeCortex;
use crate::memory::{Memory, Project as MemoryProject};
use crate::types::{Alignment, Direction, Entry, Kind, Source, Status};

const SLUG: &str = "sendov";
const AXIOMS: [&str; 3] = ["propext", "Classical.choice", "Quot.sound"];

struct Harness {
    cortex: FakeCortex,
    lean: Arc<FakeLeanEvidence>,
    calls: Arc<InMemoryCallLog>,
    ledger: Ledger,
}

async fn harness() -> Harness {
    let cortex = FakeCortex::start().await;
    let lean = Arc::new(FakeLeanEvidence::new());
    let calls = Arc::new(InMemoryCallLog::new());
    let ledger = Ledger::new(
        Memory::new(cortex.url(), "k", "ws:t"),
        lean.clone(),
        calls.clone(),
    );
    Harness {
        cortex,
        lean,
        calls,
        ledger,
    }
}

fn ctx() -> CallContext {
    CallContext::new(Project::new(SLUG, "ws:t").unwrap(), "call-1")
}

fn supplied(text: &str) -> Claim {
    Claim::new(Kind::Result, text, Source::Supplied("a remark".into()))
}

fn lean_source(sha: &str) -> Source {
    Source::Lean {
        path: "lean/Main.lean".into(),
        declaration: "Main.thm".into(),
        file_sha256: sha.into(),
    }
}

fn lean_claim(text: &str, sha: &str) -> Claim {
    Claim::new(Kind::Result, text, lean_source(sha))
}

fn tool_claim(tool: &str, call_id: &str, frame: Option<&str>) -> Claim {
    let mut claim = Claim::new(
        Kind::Result,
        format!("holds by {tool}"),
        Source::Tool {
            tool: tool.into(),
            call_id: call_id.into(),
            args_digest: None,
        },
    );
    claim.frame = frame.map(str::to_string);
    claim
}

fn log_call(h: &Harness, call_id: &str, tool: &str, project: &str, envelope: &Envelope) {
    h.calls
        .record(CallRecord::new(call_id, tool, project, envelope, 3));
}

fn ok() -> Envelope {
    Envelope::ok(json!({}))
}

fn clean(sha: &str) -> LeanVerdict {
    FakeLeanEvidence::clean(sha, "Main.thm", &AXIOMS)
}

fn teach(h: &Harness, verdict: LeanVerdict) {
    h.lean.set("lean/Main.lean", "Main.thm", Ok(verdict));
}

#[tokio::test]
async fn a_supplied_claim_is_asserted_and_recording_it_twice_is_a_no_op() {
    let h = harness().await;
    let first = h
        .ledger
        .record(&ctx(), supplied("n^2 is even"))
        .await
        .unwrap();
    assert!(first.created);
    assert_eq!(first.entry.status, Status::Asserted);
    assert_eq!(first.entry.evidence_origin, EvidenceOrigin::Supplied);
    assert_eq!(first.entry.revision, 1);
    assert!(first.entry.id.starts_with("clm_"));
    assert_eq!(first.entry.id.len(), 4 + 16);

    let again = h
        .ledger
        .record(&ctx(), supplied("  n^2 is even  "))
        .await
        .unwrap();
    assert!(!again.created);
    assert_eq!(again.entry.id, first.entry.id);
    assert_eq!(h.cortex.events().len(), 1);
    assert_eq!(
        h.cortex.events()[0].labels,
        [
            "kind:result",
            "status:asserted",
            format!("claim:{}", first.entry.id).as_str()
        ]
    );
}

#[tokio::test]
async fn an_arxiv_source_is_cited_and_a_web_source_is_capped_at_asserted() {
    let h = harness().await;
    let arxiv = Claim::new(
        Kind::Fact,
        "Sendov holds for degree 5",
        Source::Arxiv {
            id: "2012.04125".into(),
            version: "v3".into(),
            label: Some("thm:main".into()),
            section: None,
        },
    );
    assert_eq!(
        h.ledger.record(&ctx(), arxiv).await.unwrap().entry.status,
        Status::Cited
    );

    let web = Claim::new(
        Kind::Fact,
        "a MathOverflow answer",
        Source::Web {
            url: "https://mathoverflow.net/q/1".into(),
            title: "t".into(),
            class: "forum".into(),
            retrieved_at: "2026-01-01".into(),
        },
    );
    let recorded = h.ledger.record(&ctx(), web).await.unwrap();
    assert_eq!(recorded.entry.status, Status::Asserted);
    assert_eq!(recorded.entry.evidence_origin, EvidenceOrigin::Collected);
    assert_eq!(recorded.notes.len(), 1);
}

#[tokio::test]
async fn an_empty_statement_is_refused() {
    let h = harness().await;
    let error = h.ledger.record(&ctx(), supplied("   ")).await.unwrap_err();
    assert!(matches!(error, Error::InvalidArguments(_)));
    assert_eq!(h.cortex.events().len(), 0);
}

#[tokio::test]
async fn uses_must_name_known_claims_once_each() {
    let h = harness().await;
    let base = h
        .ledger
        .record(&ctx(), supplied("base"))
        .await
        .unwrap()
        .entry;

    let mut unknown = supplied("builds on nothing");
    unknown.uses = vec!["clm_missing".into()];
    assert!(matches!(
        h.ledger.record(&ctx(), unknown).await.unwrap_err(),
        Error::ClaimNotFound(id) if id == "clm_missing"
    ));

    let mut twice = supplied("repeats");
    twice.uses = vec![base.id.clone(), base.id.clone()];
    assert!(matches!(
        h.ledger.record(&ctx(), twice).await.unwrap_err(),
        Error::InvalidArguments(_)
    ));

    let mut fine = supplied("builds on base");
    fine.uses = vec![base.id.clone()];
    assert_eq!(
        h.ledger.record(&ctx(), fine).await.unwrap().entry.uses,
        [base.id]
    );
}

#[tokio::test]
async fn a_tool_result_reaches_checked_only_on_a_stated_frame() {
    let h = harness().await;
    log_call(&h, "c-py", "py_run", SLUG, &ok());
    log_call(&h, "c-calc", "calc_factor", SLUG, &ok());

    let py = h
        .ledger
        .record(&ctx(), tool_claim("py_run", "c-py", Some("n <= 10^6")))
        .await
        .unwrap();
    assert_eq!(py.entry.status, Status::Checked);
    assert_eq!(py.entry.evidence_origin, EvidenceOrigin::Collected);
    assert_eq!(py.entry.frame.as_deref(), Some("n <= 10^6"));

    let calc = h
        .ledger
        .record(&ctx(), tool_claim("calc_factor", "c-calc", Some("n = 91")))
        .await
        .unwrap();
    assert_eq!(calc.entry.status, Status::Checked);

    for frame in [None, Some("  ")] {
        let error = h
            .ledger
            .record(&ctx(), tool_claim("py_run", "c-py", frame))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::MissingFrame));
    }
}

#[tokio::test]
async fn a_tool_source_must_be_a_successful_call_this_process_made_for_this_project() {
    let h = harness().await;
    log_call(&h, "c-other-project", "py_run", "another", &ok());
    log_call(&h, "c-other-tool", "calc_factor", SLUG, &ok());
    log_call(
        &h,
        "c-unknown",
        "py_run",
        SLUG,
        &Envelope::unknown("timed out"),
    );
    log_call(
        &h,
        "c-error",
        "py_run",
        SLUG,
        &Envelope::error(ErrorCode::Timeout, "slow"),
    );

    let unrecorded = h
        .ledger
        .record(&ctx(), tool_claim("py_run", "never-ran", Some("f")))
        .await
        .unwrap_err();
    assert!(matches!(unrecorded, Error::UnrecordedCall(id) if id == "never-ran"));

    for call_id in ["c-other-project", "c-other-tool", "c-unknown", "c-error"] {
        let error = h
            .ledger
            .record(&ctx(), tool_claim("py_run", call_id, Some("f")))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::CallNotUsable { .. }), "{call_id}");
    }
    assert_eq!(h.cortex.events().len(), 0);
}

#[tokio::test]
async fn tools_that_cannot_check_anything_leave_the_claim_asserted() {
    let h = harness().await;
    for tool in ["lit_search_arxiv", "agent_formalize", "web_search"] {
        log_call(&h, "c", tool, SLUG, &ok());
        let recorded = h
            .ledger
            .record(&ctx(), tool_claim(tool, "c", Some("f")))
            .await
            .unwrap();
        assert_eq!(recorded.entry.status, Status::Asserted, "{tool}");
        assert_eq!(recorded.entry.evidence_origin, EvidenceOrigin::Collected);
    }
}

#[tokio::test]
async fn supplied_evidence_never_earns_checked_or_formalised() {
    let h = harness().await;
    log_call(&h, "c-py", "py_run", SLUG, &ok());
    teach(&h, clean("aa"));
    let supplied_ctx = ctx().with_evidence_origin(EvidenceOrigin::Supplied);

    let tool = h
        .ledger
        .record(&supplied_ctx, tool_claim("py_run", "c-py", Some("f")))
        .await
        .unwrap();
    assert_eq!(tool.entry.status, Status::Asserted);
    assert_eq!(tool.entry.evidence_origin, EvidenceOrigin::Supplied);

    let lean = h
        .ledger
        .record(&supplied_ctx, lean_claim("thm", "aa"))
        .await
        .unwrap();
    assert_eq!(lean.entry.status, Status::Asserted);
    assert!(lean.entry.lean.is_none());
    assert_eq!(
        h.lean.calls().len(),
        0,
        "no check runs for supplied evidence"
    );
}

#[tokio::test]
async fn a_clean_lean_check_earns_formalised_with_alignment_not_reviewed() {
    let h = harness().await;
    teach(&h, clean("aa"));
    let recorded = h
        .ledger
        .record(&ctx(), lean_claim("thm", "aa"))
        .await
        .unwrap();
    assert_eq!(recorded.entry.status, Status::Formalised);
    assert_eq!(recorded.entry.evidence_origin, EvidenceOrigin::Collected);
    assert_eq!(recorded.entry.alignment, Alignment::NotReviewed);
    let lean = recorded.entry.lean.unwrap();
    assert_eq!(lean.verdict, LeanVerdictKind::Checked);
    assert_eq!(lean.file_sha256, "aa");
    assert_eq!(recorded.notes.len(), 0);
}

#[tokio::test]
async fn a_lean_check_that_falls_short_leaves_the_claim_asserted() {
    let h = harness().await;
    let shortfalls = [
        LeanVerdict {
            verdict: LeanVerdictKind::CheckedWithSorry,
            sorries: 1,
            ..clean("aa")
        },
        LeanVerdict {
            verdict: LeanVerdictKind::Failed,
            ..clean("aa")
        },
        LeanVerdict {
            verdict: LeanVerdictKind::Unknown,
            ..clean("aa")
        },
        LeanVerdict {
            debt: vec!["Main.thm".into()],
            ..clean("aa")
        },
        FakeLeanEvidence::clean("aa", "Main.thm", &["propext", "sorryAx"]),
        LeanVerdict {
            axioms: std::collections::BTreeMap::new(),
            ..clean("aa")
        },
    ];
    for (n, verdict) in shortfalls.into_iter().enumerate() {
        teach(&h, verdict);
        let recorded = h
            .ledger
            .record(&ctx(), lean_claim(&format!("thm {n}"), "aa"))
            .await
            .unwrap();
        assert_eq!(recorded.entry.status, Status::Asserted, "case {n}");
        assert!(
            recorded.entry.lean.is_some(),
            "the verdict is still kept, case {n}"
        );
        assert_eq!(recorded.notes.len(), 1);
    }
}

#[tokio::test]
async fn a_digest_the_file_no_longer_has_is_a_conflict() {
    let h = harness().await;
    teach(&h, clean("bb"));
    let error = h
        .ledger
        .record(&ctx(), lean_claim("thm", "aa"))
        .await
        .unwrap_err();
    assert!(
        matches!(error, Error::StaleDigest { given, current, .. } if given == "aa" && current == "bb")
    );
    assert_eq!(h.cortex.events().len(), 0);
}

#[tokio::test]
async fn lean_failures_surface_as_errors_and_record_nothing() {
    let h = harness().await;
    h.lean.set(
        "lean/Main.lean",
        "Main.thm",
        Err(Error::EvidenceUnavailable("down".into())),
    );
    assert!(matches!(
        h.ledger
            .record(&ctx(), lean_claim("thm", "aa"))
            .await
            .unwrap_err(),
        Error::EvidenceUnavailable(_)
    ));
    h.lean
        .set("lean/Main.lean", "Main.thm", Err(Error::EvidenceTimeout));
    assert!(matches!(
        h.ledger
            .record(&ctx(), lean_claim("thm", "aa"))
            .await
            .unwrap_err(),
        Error::EvidenceTimeout
    ));
    assert_eq!(h.cortex.events().len(), 0);
}

#[tokio::test]
async fn recording_again_after_a_failed_recheck_lowers_the_claim_with_a_supersedes() {
    let h = harness().await;
    teach(&h, clean("aa"));
    let first = h
        .ledger
        .record(&ctx(), lean_claim("thm", "aa"))
        .await
        .unwrap()
        .entry;
    assert_eq!(first.status, Status::Formalised);

    teach(
        &h,
        LeanVerdict {
            verdict: LeanVerdictKind::Failed,
            ..clean("aa")
        },
    );
    let second = h
        .ledger
        .record(&ctx(), lean_claim("thm", "aa"))
        .await
        .unwrap();
    assert!(second.created);
    assert_eq!(second.entry.id, first.id);
    assert_eq!(second.entry.revision, 2);
    assert_eq!(second.entry.status, Status::Asserted);
    assert_eq!(second.entry.supersedes, Some(format!("{}#1", first.id)));

    let read = h.ledger.get(SLUG, &first.id).await.unwrap().unwrap();
    assert_eq!(read.revision, 2, "readers take the newest revision");
}

#[tokio::test]
async fn re_recording_keeps_alignment_and_merges_new_uses() {
    let h = harness().await;
    let a = h.ledger.record(&ctx(), supplied("a")).await.unwrap().entry;
    let b = h.ledger.record(&ctx(), supplied("b")).await.unwrap().entry;
    let mut claim = supplied("c");
    claim.uses = vec![a.id.clone()];
    let c = h.ledger.record(&ctx(), claim.clone()).await.unwrap().entry;
    h.ledger
        .set_alignment(&ctx(), &c.id, Alignment::Accepted)
        .await
        .unwrap();

    claim.uses = vec![b.id.clone()];
    let merged = h.ledger.record(&ctx(), claim).await.unwrap();
    assert!(merged.created);
    assert_eq!(merged.entry.uses, [a.id, b.id]);
    assert_eq!(merged.entry.alignment, Alignment::Accepted);
}

#[tokio::test]
async fn effective_status_is_the_minimum_over_everything_a_claim_reaches() {
    let h = harness().await;
    let weak = h
        .ledger
        .record(&ctx(), supplied("lemma"))
        .await
        .unwrap()
        .entry;
    teach(&h, clean("aa"));
    let mut main = lean_claim("main theorem", "aa");
    main.uses = vec![weak.id.clone()];
    let main = h.ledger.record(&ctx(), main).await.unwrap().entry;
    assert_eq!(
        main.status,
        Status::Formalised,
        "stored status is what the check earned"
    );

    let effective = h
        .ledger
        .effective(SLUG, std::slice::from_ref(&main))
        .await
        .unwrap();
    assert_eq!(effective[0].status, Status::Asserted);
    assert_eq!(effective[0].chain, [main.id.clone(), weak.id.clone()]);

    let alone = h
        .ledger
        .effective(SLUG, std::slice::from_ref(&weak))
        .await
        .unwrap();
    assert_eq!(alone[0].chain, [weak.id.as_str()]);
}

#[tokio::test]
async fn effective_status_walks_a_longer_chain_to_the_weakest_link() {
    let h = harness().await;
    let a = h.ledger.record(&ctx(), supplied("a")).await.unwrap().entry;
    log_call(&h, "c", "py_run", SLUG, &ok());
    let mut b = tool_claim("py_run", "c", Some("n < 9"));
    b.uses = vec![a.id.clone()];
    let b = h.ledger.record(&ctx(), b).await.unwrap().entry;
    let mut c = Claim::new(
        Kind::Fact,
        "c",
        Source::Arxiv {
            id: "1".into(),
            version: "v1".into(),
            label: None,
            section: Some("2".into()),
        },
    );
    c.uses = vec![b.id.clone()];
    let c = h.ledger.record(&ctx(), c).await.unwrap().entry;
    assert_eq!(c.status, Status::Cited);
    let eff = h
        .ledger
        .effective(SLUG, std::slice::from_ref(&c))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(eff.status, Status::Asserted);
    assert_eq!(eff.chain, [c.id, b.id, a.id]);
}

#[tokio::test]
async fn a_changed_file_drops_a_formalised_claim_to_asserted() {
    let h = harness().await;
    teach(&h, clean("aa"));
    let claim = h
        .ledger
        .record(&ctx(), lean_claim("thm", "aa"))
        .await
        .unwrap()
        .entry;
    let eff = |h: &Harness, e: Entry| {
        let ledger = h.ledger.clone();
        async move { ledger.effective(SLUG, &[e]).await.unwrap().remove(0) }
    };
    assert_eq!(eff(&h, claim.clone()).await.status, Status::Formalised);

    teach(&h, clean("bb"));
    let after = eff(&h, claim.clone()).await;
    assert_eq!(after.status, Status::Asserted);
    assert!(after.notes[0].contains("changed"), "{:?}", after.notes);

    teach(
        &h,
        LeanVerdict {
            verdict: LeanVerdictKind::CheckedWithSorry,
            sorries: 1,
            ..clean("aa")
        },
    );
    let after = eff(&h, claim.clone()).await;
    assert_eq!(after.status, Status::Asserted);
    assert!(after.notes[0].contains("no longer checks"));

    h.lean.set(
        "lean/Main.lean",
        "Main.thm",
        Err(Error::EvidenceUnavailable("down".into())),
    );
    let after = eff(&h, claim).await;
    assert_eq!(after.status, Status::Asserted);
    assert!(after.notes[0].contains("could not be re-checked"));
}

#[tokio::test]
async fn effective_status_tolerates_missing_and_cyclic_edges() {
    let h = harness().await;
    let memory = Memory::new(h.cortex.url(), "k", "ws:t");
    let project = MemoryProject::new(SLUG).unwrap();
    let base = h
        .ledger
        .record(&ctx(), supplied("base"))
        .await
        .unwrap()
        .entry;
    let mut dangling = base.clone();
    dangling.id = "clm_dangling".into();
    dangling.status = Status::Cited;
    dangling.uses = vec!["clm_nowhere".into()];
    let mut a = base.clone();
    a.id = "clm_a".into();
    a.status = Status::Cited;
    a.uses = vec!["clm_b".into()];
    let mut b = a.clone();
    b.id = "clm_b".into();
    b.uses = vec!["clm_a".into()];
    for entry in [&dangling, &a, &b] {
        memory
            .remember(&project, &serde_json::to_string(entry).unwrap(), &[])
            .await
            .unwrap();
    }
    let eff = h.ledger.effective(SLUG, &[dangling, a, b]).await.unwrap();
    assert_eq!(eff[0].status, Status::Asserted);
    assert_eq!(eff[0].chain, ["clm_dangling", "clm_nowhere"]);
    assert!(eff[0].notes[0].contains("not in the ledger"));
    assert_eq!(
        eff[1].status,
        Status::Cited,
        "a cycle is cut, not followed forever"
    );
    assert_eq!(eff[2].status, Status::Cited);
}

#[tokio::test]
async fn link_records_an_edge_once_and_refuses_cycles_and_unknown_claims() {
    let h = harness().await;
    let a = h.ledger.record(&ctx(), supplied("a")).await.unwrap().entry;
    let b = h.ledger.record(&ctx(), supplied("b")).await.unwrap().entry;
    let c = h.ledger.record(&ctx(), supplied("c")).await.unwrap().entry;

    let linked = h.ledger.link(&ctx(), &a.id, &b.id).await.unwrap();
    assert!(linked.created);
    assert_eq!(linked.entry.uses, [b.id.as_str()]);
    assert_eq!(linked.entry.revision, 2);
    assert_eq!(linked.entry.supersedes, Some(format!("{}#1", a.id)));

    let again = h.ledger.link(&ctx(), &a.id, &b.id).await.unwrap();
    assert!(!again.created);

    h.ledger.link(&ctx(), &b.id, &c.id).await.unwrap();
    for (from, to) in [
        (&a.id, &a.id),
        (&b.id, &a.id),
        (&c.id, &a.id),
        (&c.id, &b.id),
    ] {
        let error = h.ledger.link(&ctx(), from, to).await.unwrap_err();
        assert!(matches!(error, Error::LinkCycle { .. }), "{from} -> {to}");
    }
    for (from, to) in [("clm_x", a.id.as_str()), (a.id.as_str(), "clm_x")] {
        let error = h.ledger.link(&ctx(), from, to).await.unwrap_err();
        assert!(matches!(error, Error::ClaimNotFound(id) if id == "clm_x"));
    }
    let eff = h.ledger.effective(SLUG, &[linked.entry]).await.unwrap();
    assert_eq!(
        eff[0].chain.len(),
        1,
        "all asserted: nothing limits a below"
    );
}

#[tokio::test]
async fn alignment_changes_only_through_set_alignment() {
    let h = harness().await;
    teach(&h, clean("aa"));
    let entry = h
        .ledger
        .record(&ctx(), lean_claim("thm", "aa"))
        .await
        .unwrap()
        .entry;
    assert_eq!(entry.alignment, Alignment::NotReviewed);
    let accepted = h
        .ledger
        .set_alignment(&ctx(), &entry.id, Alignment::Disputed)
        .await
        .unwrap();
    assert_eq!(accepted.alignment, Alignment::Disputed);
    assert_eq!(accepted.revision, 2);
    let same = h
        .ledger
        .set_alignment(&ctx(), &entry.id, Alignment::Disputed)
        .await
        .unwrap();
    assert_eq!(same.revision, 2, "no new revision for no change");
    assert!(matches!(
        h.ledger
            .set_alignment(&ctx(), "clm_x", Alignment::Accepted)
            .await
            .unwrap_err(),
        Error::ClaimNotFound(_)
    ));
}

fn offer(direction: Direction, value: i64, text: &str) -> BoundRequest {
    BoundRequest {
        quantity: "Sendov  radius".into(),
        direction,
        value: Number::from(value),
        source: Source::Supplied(text.into()),
        frame: None,
    }
}

#[tokio::test]
async fn bounds_only_improve_and_a_worse_one_is_kept_as_a_fact() {
    let h = harness().await;
    let first = h
        .ledger
        .bound(&ctx(), offer(Direction::Upper, 10, "paper A"))
        .await
        .unwrap();
    assert!(first.improved);
    assert_eq!(first.recorded.entry.kind, Kind::Bound);
    assert_eq!(first.recorded.entry.statement, "Sendov radius ≤ 10");
    assert_eq!(first.best.id, first.recorded.entry.id);

    let worse = h
        .ledger
        .bound(&ctx(), offer(Direction::Upper, 12, "paper B"))
        .await
        .unwrap();
    assert!(!worse.improved);
    assert_eq!(worse.recorded.entry.kind, Kind::Fact);
    assert_eq!(worse.best.id, first.best.id);

    let equal = h
        .ledger
        .bound(&ctx(), offer(Direction::Upper, 10, "paper C"))
        .await
        .unwrap();
    assert!(!equal.improved);
    assert_eq!(equal.recorded.entry.kind, Kind::Fact);

    let better = h
        .ledger
        .bound(&ctx(), offer(Direction::Upper, 7, "paper D"))
        .await
        .unwrap();
    assert!(better.improved);
    assert_eq!(better.best.bound.as_ref().unwrap().value, Number::from(7));

    let repeat = h
        .ledger
        .bound(&ctx(), offer(Direction::Upper, 7, "paper D"))
        .await
        .unwrap();
    assert!(
        !repeat.improved,
        "offering the best bound again changes nothing"
    );
    assert!(!repeat.recorded.created);
    assert_eq!(repeat.best.id, better.best.id);
}

#[tokio::test]
async fn lower_and_upper_bounds_are_tracked_separately() {
    let h = harness().await;
    h.ledger
        .bound(&ctx(), offer(Direction::Upper, 10, "u"))
        .await
        .unwrap();
    let lower = h
        .ledger
        .bound(&ctx(), offer(Direction::Lower, 3, "l"))
        .await
        .unwrap();
    assert!(lower.improved);
    assert_eq!(lower.recorded.entry.statement, "Sendov radius ≥ 3");
    let higher = h
        .ledger
        .bound(&ctx(), offer(Direction::Lower, 5, "l2"))
        .await
        .unwrap();
    assert!(higher.improved);
    let lower_again = h
        .ledger
        .bound(&ctx(), offer(Direction::Lower, 4, "l3"))
        .await
        .unwrap();
    assert!(!lower_again.improved);
    assert_eq!(lower_again.best.id, higher.best.id);
}

#[tokio::test]
async fn an_exhausted_search_reaches_checked_on_its_frame_and_a_cut_short_one_is_refused() {
    let h = harness().await;
    let frame = "every integer from 0 to 100";
    let exhausted = Envelope::ok(json!({"result": "exhausted", "frame_description": frame}));
    log_call(
        &h,
        "c-exhausted",
        "py_search_counterexample",
        SLUG,
        &exhausted,
    );
    log_call(
        &h,
        "c-limit",
        "py_search_counterexample",
        SLUG,
        &Envelope::unknown("the search hit the time limit"),
    );

    let mut bound = tool_claim("py_search_counterexample", "c-exhausted", Some(frame));
    bound.kind = Kind::Bound;
    let recorded = h.ledger.record(&ctx(), bound).await.unwrap();
    assert_eq!(recorded.entry.status, Status::Checked);
    assert_ne!(recorded.entry.status, Status::Formalised);
    assert_eq!(recorded.entry.frame.as_deref(), Some(frame));
    assert_eq!(recorded.entry.evidence_origin, EvidenceOrigin::Collected);

    let mut result = tool_claim("py_search_counterexample", "c-exhausted", Some(frame));
    result.statement = "no counterexample in the frame".into();
    let recorded = h.ledger.record(&ctx(), result).await.unwrap();
    assert_eq!(recorded.entry.status, Status::Checked);

    let mut cut_short = tool_claim("py_search_counterexample", "c-limit", Some(frame));
    cut_short.kind = Kind::Bound;
    let error = h.ledger.record(&ctx(), cut_short).await.unwrap_err();
    assert!(matches!(error, Error::CallNotUsable { .. }));
}

#[tokio::test]
async fn a_bound_is_earned_like_any_claim_and_needs_a_name() {
    let h = harness().await;
    log_call(&h, "c", "py_run", SLUG, &ok());
    let mut tooled = offer(Direction::Upper, 4, "");
    tooled.source = Source::Tool {
        tool: "py_run".into(),
        call_id: "c".into(),
        args_digest: None,
    };
    assert!(matches!(
        h.ledger.bound(&ctx(), tooled.clone()).await.unwrap_err(),
        Error::MissingFrame
    ));
    tooled.frame = Some("degree <= 6".into());
    let outcome = h.ledger.bound(&ctx(), tooled).await.unwrap();
    assert_eq!(outcome.recorded.entry.status, Status::Checked);

    let mut nameless = offer(Direction::Upper, 4, "x");
    nameless.quantity = "  ".into();
    assert!(matches!(
        h.ledger.bound(&ctx(), nameless).await.unwrap_err(),
        Error::InvalidArguments(_)
    ));
}

#[test]
fn numbers_compare_exactly_as_integers_and_by_value_as_floats() {
    let big = Number::from(u64::MAX);
    let less = Number::from(u64::MAX - 1);
    assert_eq!(compare(&big, &less), std::cmp::Ordering::Greater);
    assert_eq!(compare(&Number::from(-1), &big), std::cmp::Ordering::Less);
    let half = Number::from_f64(0.5).unwrap();
    assert_eq!(compare(&half, &Number::from(1)), std::cmp::Ordering::Less);
    assert!(improves(Direction::Upper, &half, &Number::from(1)));
    assert!(!improves(Direction::Upper, &Number::from(1), &half));
    assert!(improves(Direction::Lower, &Number::from(1), &half));
}

#[test]
fn the_highest_revision_wins_and_order_is_kept() {
    let base = |id: &str, revision: u32| Entry {
        id: id.into(),
        revision,
        kind: Kind::Lead,
        statement: format!("{id} r{revision}"),
        status: Status::Asserted,
        evidence_origin: EvidenceOrigin::Supplied,
        source: Source::Supplied("s".into()),
        lean: None,
        alignment: Alignment::NotReviewed,
        uses: vec![],
        supersedes: None,
        rationale: None,
        reason: None,
        refutes: None,
        frame: None,
        bound: None,
    };
    let kept =
        latest_revisions([base("a", 2), base("b", 1), base("a", 1), base("b", 3)].into_iter());
    let seen: Vec<String> = kept.iter().map(|e| e.statement.clone()).collect();
    assert_eq!(seen, ["a r2", "b r3"]);
}

#[tokio::test]
async fn reads_skip_notes_that_are_not_entries_and_stay_inside_the_project() {
    let h = harness().await;
    let memory = Memory::new(h.cortex.url(), "k", "ws:t");
    memory
        .remember(
            &MemoryProject::new(SLUG).unwrap(),
            "a stray free-text note",
            &[],
        )
        .await
        .unwrap();
    h.ledger.record(&ctx(), supplied("real")).await.unwrap();
    let other = CallContext::new(Project::new("other", "ws:t").unwrap(), "c");
    h.ledger
        .record(&other, supplied("elsewhere"))
        .await
        .unwrap();

    let mine = h.ledger.entries(SLUG, "anything").await.unwrap();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].statement, "real");
    assert!(h.ledger.get(SLUG, "clm_nope").await.unwrap().is_none());
    assert!(matches!(
        h.ledger.entries("!!", "q").await.unwrap_err(),
        Error::InvalidProject(_)
    ));
}

#[tokio::test]
async fn a_claim_stays_findable_by_id_after_entries_that_cite_it_outrank_it() {
    // CortexDB recall ranks by similarity and returns only the best few.
    // Reviews and leads that cite a claim mention its id more often than the
    // claim's own JSON does, so a lookup by recalling the id lost the claim
    // once it had been reviewed: a live duel's second `agent_settle` answered
    // NOT_FOUND for the claim the first one had just settled on.
    let h = harness().await;
    let claim = h
        .ledger
        .record(&ctx(), supplied("every primorial has a prime partner"))
        .await
        .unwrap()
        .entry;
    for review in 0..3 {
        let mut fact = Claim::new(
            Kind::Fact,
            format!("Review {review} of {} ({}): accepted.", claim.id, claim.id),
            Source::Supplied("a reviewer".into()),
        );
        fact.uses = vec![claim.id.clone()];
        h.ledger.record(&ctx(), fact).await.unwrap();
    }
    h.cortex.rank_recall(Some(2));

    let found = h.ledger.get(SLUG, &claim.id).await.unwrap();
    assert_eq!(found.map(|entry| entry.id), Some(claim.id.clone()));
    assert!(
        h.ledger
            .get(SLUG, "clm_0000000000000000")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn the_window_bounds_a_read_and_a_dead_server_is_an_error() {
    let h = harness().await;
    for n in 0..3 {
        h.ledger
            .record(&ctx(), supplied(&format!("claim {n}")))
            .await
            .unwrap();
    }
    let narrow = h.ledger.clone().with_window(2);
    assert_eq!(narrow.window(), 2);
    assert_eq!(narrow.entries(SLUG, "q").await.unwrap().len(), 2);
    assert_eq!(h.ledger.clone().with_window(0).window(), 1);
    assert_eq!(h.ledger.window(), super::DEFAULT_WINDOW);
    assert!(format!("{:?}", h.ledger).contains("Ledger"));

    h.cortex.refuse_with(Some(503));
    assert!(matches!(
        h.ledger.entries(SLUG, "q").await.unwrap_err(),
        Error::MemoryRefused { .. }
    ));
    h.cortex.refuse_with(None);
    h.cortex.skip_index(true);
    assert!(matches!(
        h.ledger
            .record(&ctx(), supplied("unindexed"))
            .await
            .unwrap_err(),
        Error::MemoryNotIndexed { .. }
    ));
}

#[test]
fn claim_ids_depend_on_kind_statement_and_source_only() {
    let s = Source::Supplied("x".into());
    let a = Ledger::claim_id(Kind::Result, "t", &s);
    assert_eq!(a, Ledger::claim_id(Kind::Result, "t", &s));
    assert_ne!(a, Ledger::claim_id(Kind::Fact, "t", &s));
    assert_ne!(a, Ledger::claim_id(Kind::Result, "u", &s));
    assert_ne!(
        a,
        Ledger::claim_id(Kind::Result, "t", &Source::Supplied("y".into()))
    );
}
