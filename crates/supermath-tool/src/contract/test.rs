//! The contract checker against a passing tool and one broken per rule.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};

use super::{Fixture, Violation, check, violations, wire_form_holds};
use crate::fake::{FakeTool, good_schema};
use crate::{Backend, Envelope, Error, ErrorCode, Kind, Usage};

fn fixtures() -> Vec<Fixture> {
    vec![Fixture::new(json!({"n": 1})), Fixture::new(json!({}))]
}

fn no_violations() -> Vec<Violation> {
    Vec::new()
}

async fn found(tool: &FakeTool, domain: &str) -> Vec<Violation> {
    violations(tool, domain, &fixtures()).await
}

#[tokio::test]
async fn a_conforming_tool_passes() {
    check(&FakeTool::passing(), "calc", &fixtures())
        .await
        .unwrap();
}

#[tokio::test]
async fn fixtures_can_carry_their_own_context() {
    let fixture = Fixture::with_ctx(crate::fake::ctx(), json!({"n": 1}));
    assert_eq!(fixture.ctx.call_id, "call-1");
    let both = [fixture, Fixture::new(json!({}))];
    check(&FakeTool::passing(), "calc", &both).await.unwrap();
}

#[tokio::test]
async fn check_reports_every_violation_in_one_error() {
    let tool = FakeTool {
        name: "Bad",
        description: "",
        ..FakeTool::passing()
    };
    let Err(Error::Contract {
        tool: name,
        violations,
    }) = check(&tool, "calc", &fixtures()).await
    else {
        panic!("expected a contract error");
    };
    assert_eq!(name, "Bad");
    assert!(violations.contains(&Violation::BadName("Bad".into())));
    assert!(violations.contains(&Violation::EmptyDescription));
}

#[tokio::test]
async fn rejects_names_outside_the_pattern() {
    for name in ["echo", "calc_", "calc_Echo", "calc_a1", "zzz_echo"] {
        let tool = FakeTool {
            name,
            ..FakeTool::passing()
        };
        assert_eq!(
            found(&tool, "calc").await,
            [Violation::BadName(name.into())],
            "{name}"
        );
    }
}

#[tokio::test]
async fn rejects_another_domains_prefix() {
    let tool = FakeTool {
        name: "lean_echo",
        ..FakeTool::passing()
    };
    assert_eq!(
        found(&tool, "calc").await,
        [Violation::WrongDomain {
            name: "lean_echo".into(),
            domain: "calc".into()
        }]
    );
}

#[tokio::test]
async fn rejects_an_unknown_domain() {
    assert_eq!(
        found(&FakeTool::passing(), "nope").await,
        [Violation::UnknownDomain("nope".into())]
    );
}

#[tokio::test]
async fn recall_memory_belongs_to_the_ledger_only() {
    let tool = FakeTool {
        name: "recall_memory",
        ..FakeTool::passing()
    };
    assert_eq!(found(&tool, "ledger").await, no_violations());
    assert_eq!(
        found(&tool, "calc").await,
        [Violation::WrongDomain {
            name: "recall_memory".into(),
            domain: "calc".into()
        }]
    );
}

#[tokio::test]
async fn agentic_descriptions_must_begin_runs_a() {
    let tool = FakeTool {
        name: "agent_echo",
        kind: Kind::Agentic(Backend::Gemini),
        reply: |args| {
            let mut envelope = (FakeTool::passing().reply)(args);
            envelope.usage = Some(Usage::default());
            envelope
        },
        ..FakeTool::passing()
    };
    assert_eq!(found(&tool, "agent").await, [Violation::AgenticDescription]);
    let ok = FakeTool {
        description: "Runs a turn that echoes.",
        ..tool
    };
    assert_eq!(found(&ok, "agent").await, no_violations());
}

#[tokio::test]
async fn primitive_descriptions_must_begin_given() {
    for description in [
        "Returns twice an integer.",
        "given an integer, returns twice it.",
        "Givenness of an integer.",
        "Runs a turn that doubles.",
    ] {
        let tool = FakeTool {
            description,
            ..FakeTool::passing()
        };
        assert_eq!(
            found(&tool, "calc").await,
            [Violation::PrimitiveDescription],
            "{description}"
        );
    }
    let padded = FakeTool {
        description: "  Given an integer, returns twice it.",
        ..FakeTool::passing()
    };
    assert_eq!(found(&padded, "calc").await, no_violations());
}

#[test]
fn description_violation_checks_the_description_alone() {
    assert_eq!(super::description_violation(&FakeTool::passing()), None);
    let empty = FakeTool {
        description: " ",
        ..FakeTool::passing()
    };
    assert_eq!(
        super::description_violation(&empty),
        Some(Violation::EmptyDescription)
    );
    assert_eq!(
        Violation::PrimitiveDescription.to_string(),
        "primitive description must begin \"Given \""
    );
}

#[tokio::test]
async fn agentic_envelopes_must_carry_usage() {
    let tool = FakeTool {
        name: "web_echo",
        description: "Runs a Gemini turn.",
        kind: Kind::Agentic(Backend::Gemini),
        ..FakeTool::passing()
    };
    assert_eq!(
        found(&tool, "web").await,
        [
            Violation::MissingUsage { fixture: 0 },
            Violation::MissingUsage { fixture: 1 }
        ]
    );
}

#[tokio::test]
async fn schema_must_be_a_closed_object_schema() {
    let cases: [(Value, Violation); 6] = [
        (json!("nope"), Violation::SchemaNotObject),
        (
            json!({"type": "string", "additionalProperties": false}),
            Violation::SchemaNotObject,
        ),
        (
            json!({"type": "object", "properties": 3, "additionalProperties": false}),
            Violation::SchemaNotObject,
        ),
        (
            json!({"type": "object", "properties": {}}),
            Violation::SchemaOpen,
        ),
        (
            json!({"type": "object", "properties": {"n": {}}, "required": ["m"],
                   "additionalProperties": false}),
            Violation::SchemaRequiredUnknown("m".into()),
        ),
        (
            json!({"type": "object", "properties": {"n": {}}, "required": [1, "n"],
                   "additionalProperties": false}),
            Violation::SchemaRequiredInvalid,
        ),
    ];
    for (schema, violation) in cases {
        let tool = FakeTool {
            schema,
            ..FakeTool::passing()
        };
        assert!(
            found(&tool, "calc").await.contains(&violation),
            "{violation}"
        );
    }
    let tool = FakeTool {
        schema: json!({"type": "object", "properties": {},
        "required": "n", "additionalProperties": false}),
        ..FakeTool::passing()
    };
    assert!(
        found(&tool, "calc")
            .await
            .contains(&Violation::SchemaRequiredInvalid)
    );
    let good = FakeTool {
        schema: good_schema(),
        ..FakeTool::passing()
    };
    assert_eq!(found(&good, "calc").await, no_violations());
}

#[tokio::test]
async fn every_declared_code_needs_a_fixture_and_vice_versa() {
    let tool = FakeTool {
        errors: &[ErrorCode::InvalidInput, ErrorCode::Timeout],
        ..FakeTool::passing()
    };
    assert_eq!(
        found(&tool, "calc").await,
        [Violation::CodeNeverProduced(ErrorCode::Timeout)]
    );

    let silent = FakeTool {
        errors: &[],
        ..FakeTool::passing()
    };
    assert_eq!(
        found(&silent, "calc").await,
        [Violation::CodeUndeclared(ErrorCode::InvalidInput)]
    );
}

#[tokio::test]
async fn malformed_envelopes_and_missing_fixtures_are_violations() {
    let tool = FakeTool {
        reply: |_| {
            let mut envelope = Envelope::ok(json!(null));
            envelope.error = Envelope::error(ErrorCode::InvalidInput, "x").error;
            envelope
        },
        ..FakeTool::passing()
    };
    let found = violations(&tool, "calc", &[Fixture::new(json!({}))]).await;
    assert!(found.contains(&Violation::MalformedEnvelope { fixture: 0 }));

    let none = violations(&FakeTool::passing(), "calc", &[]).await;
    assert!(none.contains(&Violation::NoFixtures));
}

#[test]
fn a_tampered_wire_form_is_rejected() {
    let envelope = Envelope::ok(json!({"n": 1}));
    let value = serde_json::to_value(&envelope).unwrap();
    assert!(wire_form_holds(&value, &envelope));

    let mut missing_key = value.clone();
    missing_key.as_object_mut().unwrap().remove("cursor");
    assert!(!wire_form_holds(&missing_key, &envelope));

    let mut changed = value.clone();
    changed["data"] = json!({"n": 2});
    assert!(!wire_form_holds(&changed, &envelope));

    assert!(!wire_form_holds(&json!({"outcome": "bogus"}), &envelope));
}
