//! Unit tests for the runner's `OpenHuman` configuration.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{fixed_config, jev_credential, runtime_config};
use crate::config::Config;

fn config() -> Config {
    Config::from_lookup(|name| match name {
        "LADDER_API_KEY" => Some("ladder-key".into()),
        "OPENROUTER_API_KEY" => Some("sk-or-test".into()),
        "CORTEX_API_KEY" => Some("cortex-key".into()),
        "SUPERMATH_REASONING_EFFORT" => Some("high".into()),
        _ => None,
    })
    .unwrap()
}

#[test]
fn runtime_config_routes_tool_search_through_jev_on_openrouter() {
    let runtime = runtime_config(&config());
    assert_eq!(runtime.agent.tool_search.ranker, "jev");
    assert_eq!(runtime.agent.tool_search.jev_route, "openrouter");
    assert_eq!(runtime.runtime.reasoning_effort.as_deref(), Some("high"));
    assert!(!runtime.local_ai.runtime_enabled);
}

#[tokio::test]
async fn the_ranker_reads_the_runner_config_and_only_the_openrouter_key() {
    let runtime = runtime_config(&config());
    let loaded = fixed_config(runtime)().await.unwrap();
    assert_eq!(loaded.agent.tool_search.jev_route, "openrouter");

    let env = jev_credential("sk-or-test".into());
    assert_eq!(env("OPENROUTER_API_KEY").as_deref(), Some("sk-or-test"));
    assert_eq!(env("TYPESAFE_API_KEY"), None);
    assert_eq!(env("PATH"), None);
}
