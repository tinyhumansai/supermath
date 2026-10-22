//! Tests against a real Lean service. They skip unless `LEAN_URL` and
//! `SUPERMATH_PROJECTS` are both set, so a plain `cargo test` never touches
//! the network. The service reads files from its own projects mount, so
//! `SUPERMATH_PROJECTS` must name the directory it mounts at `/projects`:
//!
//! ```sh
//! LEAN_URL=http://localhost:3150 SUPERMATH_PROJECTS=./.supermath/projects \
//!     cargo test -p supermath-lean --test live
//! ```

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

mod support;

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;
use supermath_files::{ProjectPath, Workspace, WriteMeta};
use supermath_lean::Lean;
use supermath_tool::{CallContext, Project};
use support::config;

/// Removes the test's project from the shared projects directory.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn live_lean_check() {
    let (Ok(url), Ok(root)) = (
        std::env::var("LEAN_URL"),
        std::env::var("SUPERMATH_PROJECTS"),
    ) else {
        eprintln!("skipping live_lean_check: LEAN_URL and SUPERMATH_PROJECTS are not both set");
        return;
    };
    let root = PathBuf::from(root);
    std::fs::create_dir_all(&root).unwrap();
    let slug = format!("live-lean-check-{}", std::process::id());
    let _cleanup = Cleanup(root.join(&slug));
    let workspace = Arc::new(Workspace::new(&root));
    let lean = Lean::new(config(Some(&url)), workspace.clone()).unwrap();
    let health = lean.health().await.unwrap();
    assert!(!health.toolchain.is_empty());
    assert!(!health.mathlib.is_empty());

    let path = "lean/Supermath/LiveCheck.lean";
    let meta = WriteMeta::new("file_write", "live").allowing_generated();
    workspace
        .write(
            &slug,
            &ProjectPath::parse(path).unwrap(),
            "import Mathlib\n\ntheorem two_add_two : 2 + 2 = 4 := by\n  norm_num\n",
            None,
            &meta,
        )
        .unwrap();
    let tool = lean
        .tools()
        .into_iter()
        .find(|t| t.name() == "lean_check")
        .unwrap();
    let ctx = CallContext::new(
        Project::new(&slug, "ws:live/project:live").unwrap(),
        "live-call",
    );
    let envelope = tool.call(&ctx, json!({"path": path})).await;
    assert_eq!(envelope.data["verdict"], "checked", "{envelope:?}");
    assert_eq!(envelope.data["provenance"]["toolchain"], health.toolchain);
    assert_eq!(envelope.data["provenance"]["mathlib"], health.mathlib);
    eprintln!(
        "live_lean_check: toolchain {} mathlib {}",
        health.toolchain, health.mathlib
    );
}
