//! Workspace behaviour on a scratch projects root.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{MAX_FILE_BYTES, Workspace, WriteMeta, sha256_hex};
use crate::error::Error;
use crate::path::ProjectPath;

const LEAN: &str = "lean/Supermath/Sendov/Main.lean";

fn path(raw: &str) -> ProjectPath {
    ProjectPath::parse(raw).unwrap()
}

fn meta() -> WriteMeta {
    WriteMeta::new("file_write", "call-1")
}

fn scratch() -> (tempfile::TempDir, Workspace) {
    let dir = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(dir.path());
    (dir, workspace)
}

fn put(ws: &Workspace, raw: &str, content: &str) -> String {
    ws.write("demo", &path(raw), content, None, &meta())
        .unwrap()
        .sha256
}

#[test]
fn sha256_hex_is_lowercase_hex_of_the_bytes() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn project_dir_requires_a_valid_slug() {
    let (dir, ws) = scratch();
    assert_eq!(ws.root(), dir.path());
    assert_eq!(ws.project_dir("a-1").unwrap(), dir.path().join("a-1"));
    for bad in ["", "A", "a/b", "a b", ".."] {
        assert!(
            matches!(ws.project_dir(bad), Err(Error::InvalidSlug(_))),
            "{bad:?}"
        );
    }
    assert!(matches!(ws.write_log("A"), Err(Error::InvalidSlug(_))));
}

#[test]
fn writes_a_new_file_and_reads_it_back() {
    let (dir, ws) = scratch();
    let outcome = ws
        .write(
            "demo",
            &path(LEAN),
            "import Mathlib\n\ntheorem t : True := trivial\n",
            None,
            &meta(),
        )
        .unwrap();
    assert_eq!(outcome.previous_sha256, None);
    assert_eq!(outcome.lines, 3);
    assert!(dir.path().join("demo").join(LEAN).is_file());
    let read = ws.read("demo", &path(LEAN)).unwrap();
    assert_eq!(read.sha256, outcome.sha256);
    assert_eq!(read.line_count(), 3);
    assert_eq!(ws.sha256("demo", &path(LEAN)).unwrap(), outcome.sha256);
    assert!(ws.exists("demo", &path(LEAN)).unwrap());
    assert!(
        !ws.exists("demo", &path("lean/Supermath/Other.lean"))
            .unwrap()
    );
}

#[test]
fn no_temporary_file_is_left_beside_the_target() {
    let (dir, ws) = scratch();
    put(&ws, LEAN, "import Mathlib\n");
    let names: Vec<String> = std::fs::read_dir(dir.path().join("demo/lean/Supermath/Sendov"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["Main.lean"]);
}

#[test]
fn replacing_requires_the_current_digest() {
    let (_dir, ws) = scratch();
    let first = put(&ws, LEAN, "import Mathlib\n");
    let second = ws
        .write(
            "demo",
            &path(LEAN),
            "import Mathlib\n-- 2\n",
            Some(&first.to_ascii_uppercase()),
            &meta(),
        )
        .unwrap();
    assert_eq!(second.previous_sha256.as_deref(), Some(first.as_str()));
    // The old digest is now stale.
    let stale = ws
        .write(
            "demo",
            &path(LEAN),
            "import Mathlib\n-- 3\n",
            Some(&first),
            &meta(),
        )
        .unwrap_err();
    assert!(matches!(stale, Error::Conflict { .. }), "{stale}");
    assert_eq!(ws.read("demo", &path(LEAN)).unwrap().sha256, second.sha256);
}

#[test]
fn a_new_file_conflicts_with_an_existing_one_and_a_digest_with_none() {
    let (_dir, ws) = scratch();
    let sha = put(&ws, LEAN, "import Mathlib\n");
    let exists = ws
        .write("demo", &path(LEAN), "import Mathlib\n", None, &meta())
        .unwrap_err();
    assert!(matches!(exists, Error::Conflict { ref actual, .. } if *actual == sha));
    let missing = ws
        .write(
            "demo",
            &path("lean/Supermath/New.lean"),
            "import Mathlib\n",
            Some(&sha),
            &meta(),
        )
        .unwrap_err();
    assert!(matches!(missing, Error::Conflict { ref actual, .. } if actual == "no file"));
}

#[test]
fn write_log_records_every_success_and_no_refusal() {
    let (_dir, ws) = scratch();
    let first = put(&ws, LEAN, "import Mathlib\n");
    ws.write(
        "demo",
        &path(LEAN),
        "import Mathlib\n-- 2\n",
        Some(&first),
        &WriteMeta::new("file_edit", "call-2"),
    )
    .unwrap();
    let _ = ws.write("demo", &path(LEAN), "import Lean\n", Some(&first), &meta());
    let _ = ws.write("demo", &path(LEAN), "import Lean\n", None, &meta());
    let records = ws.write_log("demo").unwrap().entries().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].old_sha256, None);
    assert_eq!(records[0].tool, "file_write");
    assert_eq!(records[1].old_sha256.as_deref(), Some(first.as_str()));
    assert_eq!(records[1].tool, "file_edit");
    assert_eq!(
        ws.write_log("demo")
            .unwrap()
            .for_call("call-2")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn refuses_forbidden_imports_oversize_content_and_generated_paths() {
    let (_dir, ws) = scratch();
    let forbidden = ws
        .write("demo", &path(LEAN), "import Lean\n", None, &meta())
        .unwrap_err();
    assert!(matches!(forbidden, Error::ForbiddenImport { .. }));
    let big = "-".repeat(MAX_FILE_BYTES + 1);
    let large = ws
        .write("demo", &path(LEAN), &big, None, &meta())
        .unwrap_err();
    assert!(matches!(large, Error::TooLarge { .. }));
    let witness = path("lean/Supermath/Witnesses/W.lean");
    let generated = ws
        .write("demo", &witness, "import Mathlib\n", None, &meta())
        .unwrap_err();
    assert!(matches!(generated, Error::GeneratedPath(_)));
    ws.write(
        "demo",
        &witness,
        "import Mathlib\n",
        None,
        &meta().allowing_generated(),
    )
    .unwrap();
    assert!(!ws.exists("demo", &path(LEAN)).unwrap());
}

#[test]
fn reading_reports_missing_oversize_and_non_utf8_files() {
    let (dir, ws) = scratch();
    assert!(matches!(
        ws.read("demo", &path(LEAN)),
        Err(Error::NotFound(_))
    ));
    put(&ws, LEAN, "import Mathlib\n");
    let real = dir.path().join("demo").join(LEAN);
    std::fs::write(&real, vec![0xff, 0xfe]).unwrap();
    assert!(matches!(
        ws.read("demo", &path(LEAN)),
        Err(Error::NotUtf8(_))
    ));
    std::fs::write(&real, vec![b'a'; MAX_FILE_BYTES + 1]).unwrap();
    assert!(matches!(
        ws.read("demo", &path(LEAN)),
        Err(Error::TooLarge { .. })
    ));
}

#[test]
fn disk_failures_are_io_errors() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, "x").unwrap();
    let ws = Workspace::new(&blocker);
    assert!(matches!(
        ws.read("demo", &path(LEAN)),
        Err(Error::Io { .. })
    ));
    assert!(matches!(ws.list("demo"), Err(Error::Io { .. })));
    let written = ws.write("demo", &path(LEAN), "import Mathlib\n", None, &meta());
    assert!(matches!(written, Err(Error::Io { .. })));
    // A directory where the file should be cannot be read as a file.
    let ws = Workspace::new(dir.path());
    std::fs::create_dir_all(dir.path().join("demo").join(LEAN)).unwrap();
    assert!(matches!(
        ws.read("demo", &path(LEAN)),
        Err(Error::Io { .. })
    ));
    let overwritten = ws.write("demo", &path(LEAN), "import Mathlib\n", None, &meta());
    assert!(matches!(overwritten, Err(Error::Io { .. })));
}

#[cfg(unix)]
#[test]
fn a_write_the_disk_refuses_leaves_the_file_and_log_untouched() {
    use std::os::unix::fs::PermissionsExt;

    let (dir, ws) = scratch();
    let sha = put(&ws, LEAN, "import Mathlib\n");
    let folder = dir.path().join("demo/lean/Supermath/Sendov");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o555)).unwrap();
    let refused = ws.write(
        "demo",
        &path(LEAN),
        "import Mathlib\n-- 2\n",
        Some(&sha),
        &meta(),
    );
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(refused, Err(Error::Io { .. })));
    assert_eq!(ws.sha256("demo", &path(LEAN)).unwrap(), sha);
    assert_eq!(ws.write_log("demo").unwrap().entries().unwrap().len(), 1);
}

#[cfg(unix)]
mod symlinks {
    use std::os::unix::fs::symlink;

    use super::*;

    #[test]
    fn a_symlinked_file_is_refused_for_read_and_write() {
        let (dir, ws) = scratch();
        put(&ws, LEAN, "import Mathlib\n");
        let outside = dir.path().join("outside.lean");
        std::fs::write(&outside, "secret").unwrap();
        let link = dir.path().join("demo/lean/Supermath/Link.lean");
        symlink(&outside, &link).unwrap();
        let linked = path("lean/Supermath/Link.lean");
        assert!(matches!(
            ws.read("demo", &linked),
            Err(Error::InvalidPath { .. })
        ));
        assert!(matches!(
            ws.write("demo", &linked, "import Mathlib\n", None, &meta()),
            Err(Error::InvalidPath { .. })
        ));
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "secret");
    }

    #[test]
    fn a_symlinked_directory_out_of_the_project_is_refused() {
        let (dir, ws) = scratch();
        put(&ws, LEAN, "import Mathlib\n");
        let outside = dir.path().join("elsewhere");
        std::fs::create_dir(&outside).unwrap();
        symlink(&outside, dir.path().join("demo/lean/Supermath/Out")).unwrap();
        let sneaky = path("lean/Supermath/Out/Evil.lean");
        let written = ws.write("demo", &sneaky, "import Mathlib\n", None, &meta());
        assert!(matches!(written, Err(Error::InvalidPath { .. })));
        assert!(!outside.join("Evil.lean").exists());
    }

    #[test]
    fn a_symlinked_directory_within_the_project_is_followed() {
        let (dir, ws) = scratch();
        put(&ws, LEAN, "import Mathlib\n");
        symlink(
            dir.path().join("demo/lean/Supermath/Sendov"),
            dir.path().join("demo/lean/Supermath/Alias"),
        )
        .unwrap();
        assert!(
            ws.read("demo", &path("lean/Supermath/Alias/Main.lean"))
                .is_ok()
        );
    }

    #[test]
    fn listing_skips_symlinks() {
        let (dir, ws) = scratch();
        put(&ws, LEAN, "import Mathlib\n");
        symlink(
            dir.path().join("demo/lean/Supermath/Sendov/Main.lean"),
            dir.path().join("demo/lean/Supermath/Link.lean"),
        )
        .unwrap();
        let listed: Vec<String> = ws
            .list("demo")
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(listed, [LEAN]);
    }
}

#[test]
fn lists_valid_files_of_both_languages_sorted() {
    let (dir, ws) = scratch();
    assert_eq!(ws.list("demo").unwrap(), []);
    put(&ws, "python/work/b.py", "import math\n");
    put(&ws, LEAN, "import Mathlib\n");
    put(&ws, "lean/Supermath/A.lean", "import Mathlib\n");
    // Files that break the path rules, and the log, are not listed.
    std::fs::write(dir.path().join("demo/lean/Supermath/bad name.lean"), "x").unwrap();
    std::fs::write(dir.path().join("demo/lean/lakefile.toml"), "x").unwrap();
    let listed: Vec<String> = ws
        .list("demo")
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(listed, ["lean/Supermath/A.lean", LEAN, "python/work/b.py"]);
}

#[test]
fn imports_digest_follows_lean_imports_transitively() {
    let (_dir, ws) = scratch();
    let b = put(&ws, "lean/Supermath/B.lean", "import Mathlib\n");
    put(
        &ws,
        "lean/Supermath/A.lean",
        "import Mathlib\nimport Supermath.B\nimport Supermath.Gone\n",
    );
    put(&ws, LEAN, "import Mathlib\nimport Supermath.A\n");
    let digest = ws.imports_digest("demo", &path(LEAN)).unwrap();
    let names: Vec<(&str, bool)> = digest
        .modules
        .iter()
        .map(|m| (m.module.as_str(), m.sha256.is_some()))
        .collect();
    assert_eq!(
        names,
        [
            ("Supermath.A", true),
            ("Supermath.B", true),
            ("Supermath.Gone", false)
        ]
    );
    assert_eq!(digest.modules[1].sha256.as_deref(), Some(b.as_str()));

    // Changing a transitive import changes the digest; so does creating a missing one.
    ws.write(
        "demo",
        &path("lean/Supermath/B.lean"),
        "import Mathlib\n-- changed\n",
        Some(&b),
        &meta(),
    )
    .unwrap();
    let changed = ws.imports_digest("demo", &path(LEAN)).unwrap();
    assert_ne!(changed.sha256, digest.sha256);
    put(&ws, "lean/Supermath/Gone.lean", "import Mathlib\n");
    assert_ne!(
        ws.imports_digest("demo", &path(LEAN)).unwrap().sha256,
        changed.sha256
    );
}

#[test]
fn imports_digest_of_a_file_without_project_imports_is_the_empty_digest() {
    let (_dir, ws) = scratch();
    put(&ws, LEAN, "import Mathlib\n");
    let digest = ws.imports_digest("demo", &path(LEAN)).unwrap();
    assert_eq!(digest.modules, []);
    assert_eq!(digest.sha256, sha256_hex(b""));
    assert!(matches!(
        ws.imports_digest("demo", &path("lean/Supermath/No.lean")),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn imports_digest_handles_cycles_and_self_imports() {
    let (_dir, ws) = scratch();
    put(&ws, "lean/Supermath/A.lean", "import Supermath.B\n");
    put(&ws, "lean/Supermath/B.lean", "import Supermath.A\n");
    let digest = ws
        .imports_digest("demo", &path("lean/Supermath/A.lean"))
        .unwrap();
    let names: Vec<&str> = digest.modules.iter().map(|m| m.module.as_str()).collect();
    assert_eq!(names, ["Supermath.B"]);
}

#[test]
fn imports_digest_resolves_python_modules_packages_and_names() {
    let (_dir, ws) = scratch();
    put(&ws, "python/work/util.py", "import math\n");
    put(&ws, "python/work/pkg/inner.py", "import work.util\n");
    put(
        &ws,
        "python/work/main.py",
        "import numpy\nimport work.util\nfrom work.pkg import inner\nfrom work import util as u, absent\nfrom work.nothing import x\n",
    );
    let digest = ws
        .imports_digest("demo", &path("python/work/main.py"))
        .unwrap();
    let names: Vec<(&str, bool)> = digest
        .modules
        .iter()
        .map(|m| (m.module.as_str(), m.sha256.is_some()))
        .collect();
    assert_eq!(
        names,
        [
            ("work.nothing", false),
            ("work.pkg.inner", true),
            ("work.util", true)
        ]
    );
}
