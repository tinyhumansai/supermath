//! Unit tests for `lit_extract_statements`'s description, schema, and metadata.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use supermath_tool::{Exposure, Permission, Tool};

use crate::{Literature, LiteratureConfig};

fn tool() -> std::sync::Arc<dyn Tool> {
    Literature::new(&LiteratureConfig::default())
        .unwrap()
        .tools()
        .into_iter()
        .find(|t| t.name() == "lit_extract_statements")
        .unwrap()
}

#[test]
fn the_tool_declares_its_exposure_permission_and_first_sentence() {
    let tool = tool();
    assert_eq!(tool.exposure(), Exposure::Deferred);
    assert_eq!(tool.permission(), Permission::Read);
    assert!(tool.description().starts_with("Given"));
    assert!(tool.description().split_once('.').is_some());
}

#[test]
fn the_schema_is_closed_and_its_required_fields_are_declared() {
    let schema = tool().schema();
    assert_eq!(schema["additionalProperties"], false);
    let properties = schema["properties"].as_object().unwrap();
    for name in schema["required"].as_array().into_iter().flatten() {
        assert!(properties.contains_key(name.as_str().unwrap()));
    }
}
