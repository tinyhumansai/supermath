//! Tests for budget arithmetic.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use serde_json::json;

use super::{BudgetArg, Limits, budget_schema};

#[test]
fn the_documented_defaults_hold() {
    assert_eq!(Limits::DEFAULT, Limits::new(24, 200_000, 600));
    assert_eq!(Limits::REVIEW, Limits::new(12, 100_000, 300));
    assert_eq!(Limits::MAX, Limits::new(64, 1_000_000, 1_800));
    assert_eq!(Limits::DEFAULT.with_iterations(48).max_iterations, 48);
}

#[test]
fn an_override_replaces_only_the_fields_it_names() {
    let arg: BudgetArg = serde_json::from_value(json!({"wall_secs": 900})).unwrap();
    let limits = Limits::DEFAULT.apply(arg).unwrap();
    assert_eq!(limits, Limits::new(24, 200_000, 900));
    assert_eq!(limits.wall(), Duration::from_mins(15));
}

#[test]
fn every_field_can_be_overridden_up_to_the_maximum() {
    let arg = BudgetArg {
        max_iterations: Some(64),
        max_tokens: Some(1_000_000),
        wall_secs: Some(1_800),
    };
    assert_eq!(Limits::DEFAULT.apply(arg).unwrap(), Limits::MAX);
}

#[test]
fn zero_and_over_maximum_values_are_refused_with_the_field_named() {
    for (arg, field) in [
        (
            BudgetArg {
                max_iterations: Some(0),
                ..BudgetArg::default()
            },
            "max_iterations",
        ),
        (
            BudgetArg {
                max_tokens: Some(1_000_001),
                ..BudgetArg::default()
            },
            "max_tokens",
        ),
        (
            BudgetArg {
                wall_secs: Some(1_801),
                ..BudgetArg::default()
            },
            "wall_secs",
        ),
        (
            BudgetArg {
                wall_secs: Some(0),
                ..BudgetArg::default()
            },
            "wall_secs",
        ),
    ] {
        let message = Limits::DEFAULT.apply(arg).unwrap_err();
        assert!(message.contains(field), "{message}");
    }
}

#[test]
fn unknown_budget_fields_are_refused_when_parsing() {
    assert!(serde_json::from_value::<BudgetArg>(json!({"minutes": 3})).is_err());
}

#[test]
fn the_demand_mirrors_the_limits_in_milliseconds() {
    let demand = Limits::new(5, 7, 3).demand();
    assert_eq!(demand.wall_time_ms, 3000);
    assert_eq!(demand.iterations, 5);
    assert_eq!(demand.tokens, 7);
}

#[test]
fn the_schema_closes_the_object() {
    let schema = budget_schema();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["wall_secs"]["maximum"], 1800);
}
