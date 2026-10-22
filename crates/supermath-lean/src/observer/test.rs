//! Unit tests for the observer over a hand-filled cache.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::collections::BTreeMap;
use std::sync::Arc;

use supermath_files::{FileObserver, Language, ProjectPath, Workspace, WriteMeta};

use super::LeanObserver;
use crate::checker::VerdictCache;
use crate::verdict::{CheckReport, Provenance, Verdict};

fn report(path: &str, file: &str, imports: &str, verdict: Verdict) -> CheckReport {
    CheckReport {
        verdict,
        unknown: None,
        provenance: Provenance {
            toolchain: "t".into(),
            mathlib: "m".into(),
            path: path.into(),
            file_sha256: file.into(),
            imports_sha256: imports.into(),
            elapsed_ms: 1,
            axioms: BTreeMap::default(),
            debt: Vec::new(),
        },
        errors: Vec::new(),
        error_count: 0,
        sorries: Vec::new(),
        targets: Vec::new(),
        requested: Vec::new(),
        uses: BTreeMap::default(),
    }
}

#[test]
fn reports_the_verdict_and_the_digest_it_was_for() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = Arc::new(Workspace::new(dir.path()));
    let path = ProjectPath::parse("lean/Supermath/A.lean").unwrap();
    let meta = WriteMeta::new("file_write", "c1");
    let written = workspace
        .write(
            "demo",
            &path,
            "import Mathlib\ntheorem t : True := trivial\n",
            None,
            &meta,
        )
        .unwrap();
    let imports = workspace.imports_digest("demo", &path).unwrap().sha256;

    let cache = Arc::new(VerdictCache::default());
    let observer = LeanObserver::new(workspace, cache.clone());
    assert_eq!(observer.language(), Language::Lean);
    assert!(observer.last_result("demo", &path).is_none());

    cache.put(
        "demo",
        path.as_str(),
        report(path.as_str(), &written.sha256, &imports, Verdict::Checked),
        false,
    );
    let last = observer.last_result("demo", &path).unwrap();
    assert_eq!(last.result, "checked");
    assert_eq!(last.sha256, written.sha256);
}

#[test]
fn a_changed_import_makes_the_digest_unmatchable() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = Arc::new(Workspace::new(dir.path()));
    let path = ProjectPath::parse("lean/Supermath/A.lean").unwrap();
    let meta = WriteMeta::new("file_write", "c1");
    let written = workspace
        .write(
            "demo",
            &path,
            "import Supermath.B\ntheorem t : True := trivial\n",
            None,
            &meta,
        )
        .unwrap();
    let cache = Arc::new(VerdictCache::default());
    cache.put(
        "demo",
        path.as_str(),
        report(
            path.as_str(),
            &written.sha256,
            "an older imports digest",
            Verdict::Failed,
        ),
        false,
    );
    let observer = LeanObserver::new(workspace, cache);
    let last = observer.last_result("demo", &path).unwrap();
    assert_eq!(last.result, "failed");
    assert_eq!(last.sha256, format!("imports-changed:{}", written.sha256));
    assert_ne!(last.sha256, written.sha256);
}
