//! Tests for admission and the budget's wire form.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::{Budget, Demand};
use crate::{Envelope, ErrorCode};

#[test]
fn default_budget_has_the_pinned_wire_form() {
    assert_eq!(
        serde_json::to_value(Budget::default()).unwrap(),
        json!({"wall_time_ms": 30_000, "rows": 50, "bytes": 1_048_576,
               "iterations": null, "tokens": null})
    );
}

#[test]
fn admits_a_demand_within_every_limit() {
    assert!(Budget::default().admit(&Demand::default()).is_ok());
    let demand = Demand {
        rows: 50,
        bytes: 1 << 20,
        wall_time_ms: 30_000,
        ..Demand::default()
    };
    assert!(Budget::default().admit(&demand).is_ok());
}

#[test]
fn refuses_each_exhausted_resource_by_name() {
    let budget = Budget::agentic(4, 1000);
    let cases = [
        (
            Demand {
                wall_time_ms: 30_001,
                ..Demand::default()
            },
            "wall_time_ms",
        ),
        (
            Demand {
                rows: 51,
                ..Demand::default()
            },
            "rows",
        ),
        (
            Demand {
                bytes: (1 << 20) + 1,
                ..Demand::default()
            },
            "bytes",
        ),
        (
            Demand {
                iterations: 5,
                ..Demand::default()
            },
            "iterations",
        ),
        (
            Demand {
                tokens: 1001,
                ..Demand::default()
            },
            "tokens",
        ),
    ];
    for (demand, resource) in cases {
        let refusal = budget.admit(&demand).unwrap_err();
        assert_eq!(refusal.resource, resource);
        assert!(refusal.requested > refusal.limit);
    }
}

#[test]
fn primitive_budgets_do_not_limit_model_resources() {
    let demand = Demand {
        iterations: 99,
        tokens: 1 << 40,
        ..Demand::default()
    };
    assert!(Budget::default().admit(&demand).is_ok());
}

#[test]
fn a_refusal_becomes_a_budget_exceeded_envelope() {
    let refusal = Budget::default()
        .admit(&Demand {
            rows: 51,
            ..Demand::default()
        })
        .unwrap_err();
    let envelope = Envelope::from(refusal);
    assert_eq!(envelope.code(), Some(ErrorCode::BudgetExceeded));
}
