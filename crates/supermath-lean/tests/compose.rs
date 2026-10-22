//! The acceptance criterion that the `lean` service has no network at runtime,
//! asserted on `compose.yaml` itself.
//!
//! The file is read as text: the repository has no YAML dependency and the
//! service block's layout is fixed by the file's own conventions (two-space
//! service names, `networks:` as a list).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

fn compose() -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "compose.yaml"]
        .iter()
        .collect();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The lines of one top-level block (a service, or `networks:`).
fn block<'a>(text: &'a str, header: &str) -> Vec<&'a str> {
    let mut lines = text.lines();
    let mut found = Vec::new();
    for line in lines.by_ref() {
        if line == header {
            break;
        }
    }
    let indent = header.len() - header.trim_start().len();
    for line in lines {
        let own = line.len() - line.trim_start().len();
        if !line.trim().is_empty() && own <= indent {
            break;
        }
        found.push(line);
    }
    found
}

#[test]
fn the_lean_service_sits_only_on_the_internal_network() {
    let text = compose();
    let lean = block(&text, "  lean:");
    assert!(!lean.is_empty(), "compose.yaml has no `lean` service");
    let joined = lean.join("\n");
    let listed: Vec<&str> = joined
        .lines()
        .skip_while(|l| *l != "    networks:")
        .skip(1)
        .take_while(|l| l.starts_with("      - "))
        .map(|l| l.trim().trim_start_matches("- "))
        .collect();
    assert_eq!(
        listed,
        ["internal"],
        "lean must be on no network but `internal`"
    );
}

#[test]
fn the_internal_network_has_no_egress_and_lean_publishes_no_ports() {
    let text = compose();
    let networks = block(&text, "networks:");
    assert!(
        networks.iter().any(|l| l.trim() == "internal: true"),
        "the `internal` network must be `internal: true`: {networks:?}"
    );
    let lean = block(&text, "  lean:");
    assert!(
        !lean.iter().any(|l| l.trim() == "ports:"),
        "lean must publish no ports"
    );
    assert!(lean.iter().any(|l| l.trim() == "read_only: true"));
    assert!(
        lean.iter().any(|l| l.trim() == "- ALL"),
        "lean drops every capability"
    );
}

#[test]
fn the_projects_volume_is_mounted_by_lean() {
    let text = compose();
    let lean = block(&text, "  lean:");
    assert!(lean.iter().any(|l| l.trim() == "- projects:/projects"));
}
