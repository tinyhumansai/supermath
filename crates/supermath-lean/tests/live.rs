//! Tests against a real Lean service. They skip unless `LEAN_URL` is set, so a
//! plain `cargo test` never touches the network.
//!
//! ```sh
//! LEAN_URL=http://localhost:3150 cargo test -p supermath-lean --test live
//! ```

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

mod support;

use serde_json::json;
use support::{Env, config};

#[tokio::test]
async fn live_lean_check() {
    let Ok(url) = std::env::var("LEAN_URL") else {
        eprintln!("skipping live_lean_check: LEAN_URL is not set");
        return;
    };
    let env = Env::new(config(Some(&url)));
    let health = env.lean.health().await.unwrap();
    assert!(!health.toolchain.is_empty());
    assert!(!health.mathlib.is_empty());

    let path = "lean/Supermath/LiveCheck.lean";
    env.put(
        path,
        "import Mathlib\n\ntheorem two_add_two : 2 + 2 = 4 := by\n  norm_num\n",
    );
    let envelope = env.call("lean_check", json!({"path": path})).await;
    assert_eq!(envelope.data["verdict"], "checked", "{envelope:?}");
    assert_eq!(envelope.data["provenance"]["toolchain"], health.toolchain);
    assert_eq!(envelope.data["provenance"]["mathlib"], health.mathlib);
    eprintln!(
        "live_lean_check: toolchain {} mathlib {}",
        health.toolchain, health.mathlib
    );
}
