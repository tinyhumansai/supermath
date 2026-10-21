//! Unit tests for the toolkit: catalogue shape and each tool's answers.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use openhuman_embed::{Tool, ToolExposure};
use serde_json::{Value, json};

use super::{FACTOR_LIMIT, RECALL_TOOL, Toolkit, factorize, gcd};
use crate::memory::Memory;
use crate::problem::Problem;

fn problem() -> Problem {
    Problem::new("odd-sums", "Show 1+3+...+(2n-1) = n^2.").unwrap()
}

/// A memory client pointed at a port nothing listens on.
async fn unreachable_memory() -> Memory {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    Memory::new(url, "k", "ws:t")
}

fn tool(name: &str, memory: Memory) -> Box<dyn Tool> {
    Toolkit::new(memory, problem())
        .tools()
        .into_iter()
        .find(|tool| tool.name() == name)
        .unwrap()
}

async fn call(name: &str, arguments: Value) -> String {
    tool(name, Memory::new("http://x", "k", "ws:t"))
        .execute(arguments)
        .await
        .unwrap()
        .output()
}

#[test]
fn catalogue_keeps_memory_direct_and_defers_the_number_tools() {
    let toolkit = Toolkit::new(Memory::new("http://x", "k", "ws:t"), problem());
    let catalogue: Vec<(String, ToolExposure)> = toolkit
        .tools()
        .iter()
        .map(|tool| (tool.name().to_string(), tool.exposure()))
        .collect();
    assert_eq!(
        catalogue,
        vec![
            (RECALL_TOOL.to_string(), ToolExposure::Direct),
            ("factor_integer".to_string(), ToolExposure::Deferred),
            ("gcd_lcm".to_string(), ToolExposure::Deferred),
        ]
    );
    for tool in toolkit.tools() {
        assert!(tool.description().len() > 20, "{}", tool.name());
        assert_eq!(tool.parameters_schema()["type"], "object");
    }
}

#[test]
fn factorize_is_exact_and_reconstructs_its_input() {
    assert_eq!(factorize(2), vec![(2, 1)]);
    assert_eq!(factorize(360), vec![(2, 3), (3, 2), (5, 1)]);
    assert_eq!(factorize(999_983), vec![(999_983, 1)]);
    // The defining invariant: the product of the factorization is the input.
    for n in [2_u64, 97, 1_024, 600_851_475_143, FACTOR_LIMIT] {
        let product: u64 = factorize(n).iter().map(|(p, e)| p.pow(*e)).product();
        assert_eq!(product, n);
    }
}

#[test]
fn gcd_matches_its_definition() {
    assert_eq!(gcd(0, 0), 0);
    assert_eq!(gcd(0, 7), 7);
    assert_eq!(gcd(12, 18), 6);
    assert_eq!(gcd(17, 5), 1);
}

#[tokio::test]
async fn factor_integer_reports_primality_and_divisor_count() {
    let output = call("factor_integer", json!({ "n": 360 })).await;
    let parsed: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["factorization"], "2^3 * 3^2 * 5");
    assert_eq!(parsed["is_prime"], false);
    assert_eq!(parsed["divisor_count"], 24);

    let output = call("factor_integer", json!({ "n": 97 })).await;
    let parsed: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["is_prime"], true);
}

#[tokio::test]
async fn factor_integer_refuses_out_of_range_and_untyped_input() {
    for arguments in [
        json!({ "n": 1 }),
        json!({ "n": FACTOR_LIMIT + 1 }),
        json!({ "n": "360" }),
        json!({ "n": -4 }),
        json!({}),
    ] {
        let output = call("factor_integer", arguments.clone()).await;
        assert!(output.contains("`n` must be"), "{arguments}: {output}");
    }
}

#[tokio::test]
async fn gcd_lcm_answers_exactly_without_overflow() {
    let output = call("gcd_lcm", json!({ "a": 12, "b": 18 })).await;
    let parsed: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["gcd"], 6);
    assert_eq!(parsed["lcm"], "36");
    assert_eq!(parsed["coprime"], false);

    let big = u64::MAX;
    let output = call("gcd_lcm", json!({ "a": big, "b": big - 1 })).await;
    let parsed: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["gcd"], 1);
    assert_eq!(
        parsed["lcm"],
        (u128::from(big) * u128::from(big - 1)).to_string()
    );

    let output = call("gcd_lcm", json!({ "a": 0, "b": 0 })).await;
    let parsed: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["lcm"], "0");
}

#[tokio::test]
async fn gcd_lcm_refuses_a_missing_argument() {
    let output = call("gcd_lcm", json!({ "a": 3 })).await;
    assert!(output.contains("`b` must be"), "{output}");
}

#[tokio::test]
async fn recall_memory_rejects_a_missing_or_blank_query() {
    let tool = tool(RECALL_TOOL, unreachable_memory().await);
    for arguments in [json!({}), json!({ "query": "  " }), json!({ "query": 3 })] {
        let output = tool.execute(arguments).await.unwrap().output();
        assert!(output.contains("non-empty"), "{output}");
    }
}

#[tokio::test]
async fn recall_memory_reports_unavailable_memory_to_the_model() {
    let tool = tool(RECALL_TOOL, unreachable_memory().await);
    let output = tool
        .execute(json!({ "query": "odd" }))
        .await
        .unwrap()
        .output();
    assert!(output.contains("memory is unavailable"), "{output}");
}
