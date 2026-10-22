//! The four tools: the tool contract, then each behaviour in the spec.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use serde_json::{Value, json};
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{Budget, CallContext, Envelope, ErrorCode, Outcome, Project, Tool};

use super::{FileEdit, FileList, FileRead, FileWrite, file_tools};
use crate::observer::{FileObserver, LastResult};
use crate::path::{Language, ProjectPath};
use crate::workspace::{MAX_FILE_BYTES, Workspace};

const LEAN: &str = "lean/Supermath/Sendov/Main.lean";
const SOURCE: &str =
    "import Mathlib\n\ntheorem main : True := by\n  sorry\n\ndef helper : Nat := 1\n";

struct Fx {
    dir: tempfile::TempDir,
    ws: Arc<Workspace>,
}

fn fx() -> Fx {
    let dir = tempfile::tempdir().unwrap();
    let ws = Arc::new(Workspace::new(dir.path()));
    Fx { dir, ws }
}

fn ctx_for(slug: &str, call: &str) -> CallContext {
    CallContext::new(
        Project::new(slug, format!("ws:test/project:{slug}")).unwrap(),
        call,
    )
}

fn ctx() -> CallContext {
    ctx_for("demo", "call-1")
}

async fn call(tool: &dyn Tool, args: Value) -> Envelope {
    tool.call(&ctx(), args).await
}

async fn write(fx: &Fx, path: &str, content: &str) -> String {
    let envelope = call(
        &FileWrite::new(fx.ws.clone()),
        json!({"path": path, "content": content}),
    )
    .await;
    assert!(envelope.is_ok(), "{envelope:?}");
    envelope.data["sha256"].as_str().unwrap().to_owned()
}

fn code(envelope: &Envelope) -> ErrorCode {
    envelope
        .code()
        .unwrap_or_else(|| panic!("expected an error, got {envelope:?}"))
}

struct Fake {
    language: Language,
    result: Option<LastResult>,
}

impl FileObserver for Fake {
    fn language(&self) -> Language {
        self.language
    }
    fn last_result(&self, project: &str, _path: &ProjectPath) -> Option<LastResult> {
        assert_eq!(project, "demo");
        self.result.clone()
    }
}

#[tokio::test]
async fn all_four_tools_keep_the_tool_contract() {
    let fx = fx();
    let sha = write(&fx, LEAN, SOURCE).await;
    // A regular file where the project directory should be makes every disk
    // operation for that project fail with an I/O error.
    std::fs::write(fx.dir.path().join("blocked"), "x").unwrap();
    let io_ctx = ctx_for("blocked", "call-io");
    let tiny = ctx().with_budget(Budget {
        bytes: 8,
        ..Budget::default()
    });
    let new = |p: &str| json!({"path": p, "content": "import Mathlib\n", "if_sha256": null});

    contract::check(
        &FileWrite::new(fx.ws.clone()),
        "file",
        &[
            Fixture::with_ctx(ctx(), new("lean/Supermath/New.lean")),
            Fixture::with_ctx(ctx(), new("../escape")),
            Fixture::with_ctx(ctx(), new(LEAN)),
            Fixture::with_ctx(tiny.clone(), new("lean/Supermath/Tiny.lean")),
            Fixture::with_ctx(io_ctx.clone(), new(LEAN)),
        ],
    )
    .await
    .unwrap();

    let edit = |path: &str, old: &str, sha: &str| json!({"path": path, "edits": [{"old": old, "new": "trivial"}], "if_sha256": sha});
    contract::check(
        &FileEdit::new(fx.ws.clone()),
        "file",
        &[
            Fixture::with_ctx(tiny.clone(), edit(LEAN, "Mathlib", &sha)),
            Fixture::with_ctx(ctx(), edit(LEAN, "sorry", &sha)),
            Fixture::with_ctx(ctx(), edit("nope", "x", &sha)),
            Fixture::with_ctx(ctx(), edit("lean/Supermath/Missing.lean", "x", &sha)),
            Fixture::with_ctx(ctx(), edit(LEAN, "trivial", "stale")),
            Fixture::with_ctx(io_ctx.clone(), edit(LEAN, "x", &sha)),
        ],
    )
    .await
    .unwrap();

    contract::check(
        &FileRead::new(fx.ws.clone()),
        "file",
        &[
            Fixture::with_ctx(ctx(), json!({"path": LEAN})),
            Fixture::with_ctx(ctx(), json!({"path": "nope"})),
            Fixture::with_ctx(ctx(), json!({"path": "lean/Supermath/Missing.lean"})),
            Fixture::with_ctx(tiny, json!({"path": LEAN, "start_line": 1})),
            Fixture::with_ctx(io_ctx.clone(), json!({"path": LEAN})),
        ],
    )
    .await
    .unwrap();

    contract::check(
        &FileList::new(fx.ws.clone(), Vec::new()),
        "file",
        &[
            Fixture::with_ctx(ctx(), json!({})),
            Fixture::with_ctx(ctx(), json!({"prefix": "../x"})),
            Fixture::with_ctx(io_ctx, json!({})),
        ],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn file_tools_builds_the_four_in_order() {
    let fx = fx();
    let tools = file_tools(&fx.ws, Vec::new());
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert_eq!(names, ["file_write", "file_edit", "file_read", "file_list"]);
    let permissions: Vec<bool> = tools
        .iter()
        .map(|t| t.permission() == supermath_tool::Permission::Write)
        .collect();
    assert_eq!(permissions, [true, true, false, false]);
    assert!(
        tools
            .iter()
            .all(|t| t.exposure() == supermath_tool::Exposure::Deferred)
    );
    assert!(format!("{:?}", FileList::new(fx.ws.clone(), Vec::new())).contains("FileList"));
}

#[tokio::test]
async fn write_creates_replaces_and_suggests_the_checker() {
    let fx = fx();
    let tool = FileWrite::new(fx.ws.clone());
    let created = call(
        &tool,
        json!({"path": LEAN, "content": SOURCE, "if_sha256": null}),
    )
    .await;
    assert!(created.is_ok());
    assert_eq!(created.data["lines"], 6);
    assert_eq!(
        created.next,
        [format!("lean_check {{\"path\":\"{LEAN}\"}}")]
    );
    let sha = created.data["sha256"].as_str().unwrap().to_owned();

    let again = call(&tool, json!({"path": LEAN, "content": SOURCE})).await;
    assert_eq!(code(&again), ErrorCode::Conflict);
    let replaced = call(
        &tool,
        json!({"path": LEAN, "content": "import Mathlib\n", "if_sha256": sha}),
    )
    .await;
    assert!(replaced.is_ok());

    let py = call(
        &tool,
        json!({"path": "python/work/s.py", "content": "import math\n"}),
    )
    .await;
    assert_eq!(py.next, ["py_run {\"path\":\"python/work/s.py\"}"]);
}

#[tokio::test]
async fn write_refuses_bad_paths_imports_generated_dirs_and_unknown_arguments() {
    let fx = fx();
    let tool = FileWrite::new(fx.ws.clone());
    for args in [
        json!({"path": "lean/Supermath/../x.lean", "content": ""}),
        json!({"path": "/etc/passwd", "content": ""}),
        json!({"path": LEAN, "content": "import Lean\n"}),
        json!({"path": "python/work/a.py", "content": "import requests\n"}),
        json!({"path": "lean/Supermath/Witnesses/W.lean", "content": "import Mathlib\n"}),
        json!({"path": LEAN}),
        json!({"path": LEAN, "content": "x", "extra": 1}),
    ] {
        assert_eq!(
            code(&call(&tool, args.clone()).await),
            ErrorCode::InvalidInput,
            "{args}"
        );
    }
    let oversize = call(
        &tool,
        json!({"path": LEAN, "content": "x".repeat(MAX_FILE_BYTES + 1)}),
    )
    .await;
    assert_eq!(code(&oversize), ErrorCode::BudgetExceeded);
}

#[tokio::test]
async fn write_logs_the_calling_tool_and_call_id() {
    let fx = fx();
    let tool = FileWrite::new(fx.ws.clone());
    tool.call(
        &ctx_for("demo", "turn-9"),
        json!({"path": LEAN, "content": SOURCE}),
    )
    .await;
    let records = fx.ws.write_log("demo").unwrap().for_call("turn-9").unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].tool, "file_write");
    assert_eq!(records[0].path, LEAN);
}

#[tokio::test]
async fn edit_replaces_one_sorry_and_reports_changed_lines() {
    let fx = fx();
    let sha = write(&fx, LEAN, SOURCE).await;
    let tool = FileEdit::new(fx.ws.clone());
    let edited = call(
        &tool,
        json!({"path": LEAN, "edits": [{"old": "sorry", "new": "nlinarith [sq_nonneg 1]"}], "if_sha256": sha}),
    )
    .await;
    assert!(edited.is_ok(), "{edited:?}");
    assert_eq!(
        edited.data["changed"],
        json!([{"start_line": 4, "end_line": 4}])
    );
    assert_ne!(edited.data["sha256"], json!(sha));
    let text = fx
        .ws
        .read("demo", &ProjectPath::parse(LEAN).unwrap())
        .unwrap()
        .text;
    assert!(text.contains("nlinarith"));
    assert_eq!(
        fx.ws
            .write_log("demo")
            .unwrap()
            .entries()
            .unwrap()
            .last()
            .unwrap()
            .tool,
        "file_edit"
    );
}

#[tokio::test]
async fn edit_refuses_a_stale_digest_and_ambiguous_or_absent_text() {
    let fx = fx();
    let sha = write(&fx, LEAN, SOURCE).await;
    let tool = FileEdit::new(fx.ws.clone());
    let stale = call(
        &tool,
        json!({"path": LEAN, "edits": [{"old": "sorry", "new": "x"}], "if_sha256": "00"}),
    )
    .await;
    assert_eq!(code(&stale), ErrorCode::Conflict);
    for edits in [
        json!([{"old": "Nat", "new": "Int"}, {"old": "nowhere", "new": "x"}]),
        json!([{"old": "e", "new": "E"}]),
        json!([{"old": "", "new": "E"}]),
        json!([]),
    ] {
        let refused = call(
            &tool,
            json!({"path": LEAN, "edits": edits, "if_sha256": sha}),
        )
        .await;
        assert_eq!(code(&refused), ErrorCode::InvalidInput, "{edits}");
    }
    // All or none: the file is unchanged after the refusals.
    let current = fx
        .ws
        .sha256("demo", &ProjectPath::parse(LEAN).unwrap())
        .unwrap();
    assert_eq!(current, sha);
}

#[tokio::test]
async fn edit_must_keep_the_import_rules() {
    let fx = fx();
    let sha = write(&fx, LEAN, SOURCE).await;
    let tool = FileEdit::new(fx.ws.clone());
    let refused = call(&tool, json!({"path": LEAN, "edits": [{"old": "import Mathlib", "new": "import Lean"}], "if_sha256": sha})).await;
    assert_eq!(code(&refused), ErrorCode::InvalidInput);
}

#[tokio::test]
async fn read_returns_numbered_lines_a_cursor_and_the_digest() {
    let fx = fx();
    let body = (1..=500).fold(String::new(), |acc, n| acc + &format!("-- line {n}\n"));
    let sha = write(
        &fx,
        "lean/Supermath/Long.lean",
        &format!("import Mathlib\n{body}"),
    )
    .await;
    let tool = FileRead::new(fx.ws.clone());
    let page = call(
        &tool,
        json!({"path": "lean/Supermath/Long.lean", "start_line": 1}),
    )
    .await;
    assert_eq!(page.data["sha256"], json!(sha));
    assert_eq!(page.data["lines"], 501);
    assert_eq!(page.data["end_line"], 400);
    assert_eq!(page.cursor.as_deref(), Some("401"));
    assert!(
        page.data["text"]
            .as_str()
            .unwrap()
            .starts_with("1: import Mathlib\n2: -- line 1\n")
    );

    let rest = call(
        &tool,
        json!({"path": "lean/Supermath/Long.lean", "cursor": "401"}),
    )
    .await;
    assert_eq!(rest.data["start_line"], 401);
    assert_eq!(rest.data["end_line"], 501);
    assert_eq!(rest.cursor, None);

    let slice = call(
        &tool,
        json!({"path": "lean/Supermath/Long.lean", "start_line": 3, "end_line": 4}),
    )
    .await;
    assert_eq!(slice.data["text"], "3: -- line 2\n4: -- line 3\n");
    assert_eq!(slice.cursor, None);
    let only_end = call(
        &tool,
        json!({"path": "lean/Supermath/Long.lean", "end_line": 2}),
    )
    .await;
    assert_eq!(only_end.data["start_line"], 1);
}

#[tokio::test]
async fn read_without_a_range_returns_the_outline() {
    let fx = fx();
    write(&fx, LEAN, SOURCE).await;
    write(&fx, "python/work/s.py", "def f(x):\n    return x\n").await;
    let tool = FileRead::new(fx.ws.clone());
    let lean = call(&tool, json!({"path": LEAN})).await;
    assert_eq!(lean.data["language"], "lean");
    assert_eq!(
        lean.data["outline"],
        json!([
            {"kind": "theorem", "name": "main", "start_line": 3, "end_line": 4, "has_sorry": true},
            {"kind": "def", "name": "helper", "start_line": 6, "end_line": 6, "has_sorry": false},
        ])
    );
    let py = call(&tool, json!({"path": "python/work/s.py"})).await;
    assert_eq!(py.data["outline"][0]["signature"], "def f(x):");
}

#[tokio::test]
async fn read_refuses_bad_ranges_and_cursors() {
    let fx = fx();
    write(&fx, LEAN, SOURCE).await;
    let tool = FileRead::new(fx.ws.clone());
    for args in [
        json!({"path": LEAN, "start_line": 0}),
        json!({"path": LEAN, "start_line": 99}),
        json!({"path": LEAN, "start_line": 5, "end_line": 2}),
        json!({"path": LEAN, "cursor": "abc"}),
    ] {
        assert_eq!(
            code(&call(&tool, args.clone()).await),
            ErrorCode::InvalidInput,
            "{args}"
        );
    }
    write(&fx, "lean/Supermath/Empty.lean", "").await;
    let empty = call(
        &tool,
        json!({"path": "lean/Supermath/Empty.lean", "start_line": 1}),
    )
    .await;
    assert_eq!(empty.data["text"], "");
}

#[tokio::test]
async fn list_shows_files_with_last_results_and_marks_stale_ones() {
    let fx = fx();
    let sha = write(&fx, LEAN, SOURCE).await;
    write(&fx, "lean/Supermath/Other.lean", "import Mathlib\n").await;
    write(&fx, "python/work/s.py", "import math\n").await;
    let lean = Fake {
        language: Language::Lean,
        result: Some(LastResult {
            result: "ok".into(),
            sha256: sha.clone(),
        }),
    };
    let stale = Fake {
        language: Language::Python,
        result: Some(LastResult {
            result: "failed".into(),
            sha256: "old".into(),
        }),
    };
    let tool = FileList::new(fx.ws.clone(), vec![Arc::new(lean), Arc::new(stale)]);
    let listing = call(&tool, json!({})).await;
    assert_eq!(listing.data["total"], 3);
    let rows = listing.data["files"].as_array().unwrap();
    let main = rows.iter().find(|r| r["path"] == LEAN).unwrap();
    assert_eq!(main["last_result"], "ok");
    assert_eq!(main["result_sha256"], json!(sha));
    assert_eq!(main["stale"], false);
    assert_eq!(main["language"], "lean");
    assert_eq!(main["lines"], 6);
    let py = rows
        .iter()
        .find(|r| r["path"] == "python/work/s.py")
        .unwrap();
    assert_eq!(py["stale"], true);
    let fresh = rows
        .iter()
        .find(|r| r["path"] == "lean/Supermath/Other.lean")
        .unwrap();
    assert_eq!(fresh["stale"], true, "a result for another digest is stale");
}

#[tokio::test]
async fn list_without_an_observer_has_null_results_and_filters_by_prefix() {
    let fx = fx();
    write(&fx, LEAN, SOURCE).await;
    write(&fx, "python/work/s.py", "import math\n").await;
    let tool = FileList::new(fx.ws.clone(), Vec::new());
    let lean_only = call(&tool, json!({"prefix": "lean/Supermath/Sendov/"})).await;
    let rows = lean_only.data["files"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["last_result"], Value::Null);
    assert_eq!(rows[0]["stale"], Value::Null);
    let none = call(
        &FileList::new(fx.ws.clone(), Vec::new()),
        json!({"prefix": "nothing"}),
    )
    .await;
    assert_eq!(none.data["files"], json!([]));
    assert_eq!(
        code(&call(&tool, json!({"prefix": "/abs"})).await),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(&call(&tool, json!({"prefix": "a\\b"})).await),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(&call(&tool, json!({"cursor": "x"})).await),
        ErrorCode::InvalidInput
    );
}

#[tokio::test]
async fn list_pages_and_respects_the_row_budget() {
    let fx = fx();
    for name in ["A", "B", "C"] {
        write(
            &fx,
            &format!("lean/Supermath/{name}.lean"),
            "import Mathlib\n",
        )
        .await;
    }
    let tool = FileList::new(fx.ws.clone(), Vec::new());
    let first = call(&tool, json!({"limit": 2})).await;
    assert_eq!(first.data["files"].as_array().unwrap().len(), 2);
    assert_eq!(first.cursor.as_deref(), Some("2"));
    let second = call(&tool, json!({"limit": 2, "cursor": "2"})).await;
    assert_eq!(second.data["files"][0]["path"], "lean/Supermath/C.lean");
    assert_eq!(second.cursor, None);

    let narrow = ctx().with_budget(Budget {
        rows: 1,
        ..Budget::default()
    });
    let one = tool.call(&narrow, json!({})).await;
    assert_eq!(one.data["files"].as_array().unwrap().len(), 1);
    assert_eq!(one.outcome, Outcome::Ok);
}

#[tokio::test]
async fn list_skips_files_that_stopped_being_text() {
    let fx = fx();
    write(&fx, LEAN, SOURCE).await;
    std::fs::write(
        fx.dir.path().join("demo/lean/Supermath/Binary.lean"),
        [0xff_u8, 0xfe],
    )
    .unwrap();
    let tool = FileList::new(fx.ws.clone(), Vec::new());
    let listing = call(&tool, json!({})).await;
    assert_eq!(listing.outcome, Outcome::Ok);
    assert_eq!(listing.data["total"], 2);
    assert_eq!(listing.data["files"].as_array().unwrap().len(), 1);
    // `total` counts the skipped file, so the reply says which one it is.
    assert_eq!(listing.unknowns.len(), 1, "{:?}", listing.unknowns);
    assert!(listing.unknowns[0].contains("lean/Supermath/Binary.lean"));
}

#[tokio::test]
async fn list_reports_every_skipped_file_by_path() {
    let fx = fx();
    write(&fx, LEAN, SOURCE).await;
    let dir = fx.dir.path().join("demo/lean/Supermath");
    std::fs::write(dir.join("Binary.lean"), [0xff_u8, 0xfe]).unwrap();
    std::fs::write(dir.join("Huge.lean"), "x".repeat(MAX_FILE_BYTES + 1)).unwrap();
    let listing = call(&FileList::new(fx.ws.clone(), Vec::new()), json!({})).await;
    assert_eq!(listing.data["total"], 3);
    assert_eq!(listing.data["files"].as_array().unwrap().len(), 1);
    let skipped = listing.unknowns.join("\n");
    assert!(skipped.contains("lean/Supermath/Binary.lean"), "{skipped}");
    assert!(skipped.contains("lean/Supermath/Huge.lean"), "{skipped}");
}

#[cfg(unix)]
#[tokio::test]
async fn list_is_backend_unavailable_when_a_listed_file_cannot_be_read() {
    use std::os::unix::fs::PermissionsExt;
    let fx = fx();
    write(&fx, LEAN, SOURCE).await;
    let file = fx.dir.path().join("demo").join(LEAN);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let listing = call(&FileList::new(fx.ws.clone(), Vec::new()), json!({})).await;
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    // A disk failure is not a skipped file: the listing would be silently
    // short, so the call fails instead.
    assert_eq!(code(&listing), ErrorCode::BackendUnavailable);
}

#[tokio::test]
async fn list_pages_fifty_rows_by_default_and_clamps_a_larger_limit() {
    let fx = fx();
    for n in 0..55 {
        write(
            &fx,
            &format!("lean/Supermath/F{n:02}.lean"),
            "import Mathlib\n",
        )
        .await;
    }
    let tool = FileList::new(fx.ws.clone(), Vec::new());
    for args in [json!({}), json!({"limit": 500})] {
        let page = call(&tool, args.clone()).await;
        assert_eq!(page.data["files"].as_array().unwrap().len(), 50, "{args}");
        assert_eq!(page.data["total"], 55);
        assert_eq!(page.cursor.as_deref(), Some("50"), "{args}");
    }
    let rest = call(&tool, json!({"cursor": "50"})).await;
    assert_eq!(rest.data["files"].as_array().unwrap().len(), 5);
    assert_eq!(rest.cursor, None);
}

#[tokio::test]
async fn list_refuses_a_cursor_that_is_not_an_offset_and_ends_past_the_end() {
    let fx = fx();
    write(&fx, LEAN, SOURCE).await;
    let tool = FileList::new(fx.ws.clone(), Vec::new());
    for cursor in ["x", "-1", "1.5", "", " 2"] {
        let refused = call(&tool, json!({"cursor": cursor})).await;
        assert_eq!(code(&refused), ErrorCode::InvalidInput, "{cursor:?}");
    }
    let past = call(&tool, json!({"cursor": "999"})).await;
    assert_eq!(past.outcome, Outcome::Ok);
    assert_eq!(past.data["files"], json!([]));
    assert_eq!(past.data["total"], 1);
    assert_eq!(past.cursor, None);
}

#[tokio::test]
async fn read_refuses_an_outline_over_the_byte_budget() {
    let fx = fx();
    write(&fx, LEAN, SOURCE).await;
    let tiny = ctx().with_budget(Budget {
        bytes: 8,
        ..Budget::default()
    });
    let refused = FileRead::new(fx.ws.clone())
        .call(&tiny, json!({"path": LEAN}))
        .await;
    assert_eq!(code(&refused), ErrorCode::BudgetExceeded);
}

#[tokio::test]
async fn read_refuses_a_file_over_the_size_cap() {
    let fx = fx();
    write(&fx, LEAN, SOURCE).await;
    let huge = "lean/Supermath/Huge.lean";
    std::fs::write(
        fx.dir.path().join("demo").join(huge),
        "x".repeat(MAX_FILE_BYTES + 1),
    )
    .unwrap();
    let tool = FileRead::new(fx.ws.clone());
    for args in [json!({"path": huge}), json!({"path": huge, "start_line": 1})] {
        assert_eq!(
            code(&call(&tool, args.clone()).await),
            ErrorCode::BudgetExceeded,
            "{args}"
        );
    }
}

#[tokio::test]
async fn edit_tells_an_absent_old_string_from_an_empty_one() {
    let fx = fx();
    let sha = write(&fx, LEAN, SOURCE).await;
    let tool = FileEdit::new(fx.ws.clone());
    let message = |envelope: Envelope| {
        assert_eq!(code(&envelope), ErrorCode::InvalidInput);
        envelope.error.unwrap().message
    };
    let absent = message(
        call(
            &tool,
            json!({"path": LEAN, "edits": [{"old": "nowhere", "new": "x"}], "if_sha256": sha}),
        )
        .await,
    );
    let empty = message(
        call(
            &tool,
            json!({"path": LEAN, "edits": [{"old": "", "new": "x"}], "if_sha256": sha}),
        )
        .await,
    );
    assert_ne!(absent, empty);
    assert!(absent.contains("does not occur"), "{absent}");
    assert!(empty.contains("empty"), "{empty}");
    assert!(!absent.contains("empty"), "{absent}");
}

#[tokio::test]
async fn edit_refuses_a_result_over_the_size_cap() {
    let fx = fx();
    let content = format!("-- {}\n", "x".repeat(MAX_FILE_BYTES - 10));
    let path = "lean/Supermath/Big.lean";
    let sha = write(&fx, path, &content).await;
    let tool = FileEdit::new(fx.ws.clone());
    let refused = call(
        &tool,
        json!({"path": path, "edits": [{"old": "-- ", "new": "-- grown past the cap "}], "if_sha256": sha}),
    )
    .await;
    assert_eq!(code(&refused), ErrorCode::BudgetExceeded);
    let current = fx.ws.sha256("demo", &ProjectPath::parse(path).unwrap()).unwrap();
    assert_eq!(current, sha);
}

#[tokio::test]
async fn edit_refuses_a_file_in_a_generated_directory() {
    let fx = fx();
    let generated = "lean/Supermath/Witnesses/W.lean";
    let disk = fx.dir.path().join("demo").join(generated);
    std::fs::create_dir_all(disk.parent().unwrap()).unwrap();
    std::fs::write(&disk, SOURCE).unwrap();
    let sha = fx
        .ws
        .sha256("demo", &ProjectPath::parse(generated).unwrap())
        .unwrap();
    let refused = call(
        &FileEdit::new(fx.ws.clone()),
        json!({"path": generated, "edits": [{"old": "sorry", "new": "trivial"}], "if_sha256": sha}),
    )
    .await;
    assert_eq!(code(&refused), ErrorCode::InvalidInput);
    assert_eq!(std::fs::read_to_string(&disk).unwrap(), SOURCE);
}

#[tokio::test]
async fn edit_requires_if_sha256() {
    let fx = fx();
    write(&fx, LEAN, SOURCE).await;
    let tool = FileEdit::new(fx.ws.clone());
    for missing in [
        json!({"path": LEAN, "edits": [{"old": "sorry", "new": "trivial"}]}),
        json!({"path": LEAN, "edits": [{"old": "sorry", "new": "trivial"}], "if_sha256": null}),
    ] {
        assert_eq!(
            code(&call(&tool, missing.clone()).await),
            ErrorCode::InvalidInput,
            "{missing}"
        );
    }
    let text = fx
        .ws
        .read("demo", &ProjectPath::parse(LEAN).unwrap())
        .unwrap()
        .text;
    assert_eq!(text, SOURCE);
}
