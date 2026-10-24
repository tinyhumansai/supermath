//! A loop folder from the compiled-in template.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use serde_json::Value;

use super::{STATEMENT_PATH, TEMPLATE, create};
use crate::cli::Program;
use crate::error::Error;
use crate::{Config, Toolkit};

const STATEMENT: &str = "import Mathlib\n\nnamespace Supermath\n\n\
                         def Conjecture : Prop := ∀ n : ℕ, n + 0 = n\n\nend Supermath\n";

fn program() -> Program {
    Program {
        binary: "/bin/supermath-mcp".into(),
        project: "demo".into(),
        env_file: Some("/work/supermath.env".into()),
    }
}

fn toolkit() -> Toolkit {
    Toolkit::compose(&Config::from_lookup(|_| None).unwrap(), "demo").unwrap()
}

fn scaffold() -> (tempfile::TempDir, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("loops/demo");
    create(&dir, STATEMENT, &program(), &toolkit().registry()).unwrap();
    (root, dir)
}

fn json(dir: &Path, path: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join(path)).unwrap()).unwrap()
}

#[test]
fn writes_every_template_file_and_the_statement_with_no_placeholder_left() {
    let (_root, dir) = scaffold();
    for (path, _) in TEMPLATE {
        let text = std::fs::read_to_string(dir.join(path)).unwrap();
        for ours in [
            "{{slug}}",
            "{{binary}}",
            "{{env_file_arg}}",
            "{{statement}}",
        ] {
            assert!(!text.contains(ours), "{path} keeps {ours}");
        }
    }
    assert_eq!(
        std::fs::read_to_string(dir.join(STATEMENT_PATH)).unwrap(),
        STATEMENT
    );
    let brief = std::fs::read_to_string(dir.join("brief.md")).unwrap();
    assert!(brief.contains("def Conjecture : Prop"), "{brief}");
    // rsi-ralph fills its own placeholders each epoch.
    assert!(brief.contains("{{incumbent}}") && brief.contains("{{failures}}"));
}

#[test]
fn the_manifest_runs_this_binary_as_its_scorer_and_freezes_the_statement() {
    let (_root, dir) = scaffold();
    let manifest = json(&dir, "rsi.json");
    assert_eq!(manifest["name"], "demo");
    assert_eq!(
        manifest["score"]["scorers"][0]["run"],
        "/bin/supermath-mcp score proof demo --env-file /work/supermath.env"
    );
    let frozen = manifest["subject"]["frozen"].as_array().unwrap();
    assert!(frozen.contains(&Value::from("lean/Supermath/Statement.lean")));
}

#[test]
fn the_hive_holds_the_toolkit_less_the_file_tools_in_domain_toolpacks() {
    let (_root, dir) = scaffold();
    let hive = json(&dir, "hive/hive.json");
    assert_eq!(hive["id"], "demo");
    let ids: Vec<&str> = hive["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), toolkit().registry().iter().count() - 5);
    assert!(ids.contains(&"lean_check") && ids.contains(&"web_research"));
    assert!(!ids.contains(&"file_write") && !ids.contains(&"web_search"));
    let pack = |id: &str| -> Vec<String> {
        hive["toolpacks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pack| pack["id"] == id)
            .unwrap()["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool.as_str().unwrap().to_owned())
            .collect()
    };
    assert!(pack("lean").iter().all(|tool| tool.starts_with("lean_")));
    assert!(pack("python").contains(&"calc_factor".to_owned()));
    assert!(pack("ledger").contains(&"recall_memory".to_owned()));
    assert_eq!(pack("ledger-read"), ["ledger_claims", "recall_memory"]);
    // Every toolpack a seat names exists.
    for seat in hive["seats"].as_array().unwrap() {
        for name in seat["toolpacks"].as_array().unwrap() {
            pack(name.as_str().unwrap());
        }
    }
}

#[test]
fn without_an_env_file_the_scorer_takes_none() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("demo");
    let program = Program {
        env_file: None,
        ..program()
    };
    create(&dir, STATEMENT, &program, &toolkit().registry()).unwrap();
    assert_eq!(
        json(&dir, "rsi.json")["score"]["scorers"][0]["run"],
        "/bin/supermath-mcp score proof demo"
    );
}

#[test]
fn an_existing_folder_is_never_overwritten() {
    let (_root, dir) = scaffold();
    let error = create(&dir, STATEMENT, &program(), &toolkit().registry()).unwrap_err();
    assert!(matches!(error, Error::LoopExists(_)), "{error:?}");
}

#[test]
fn a_statement_without_the_conjecture_is_refused() {
    let root = tempfile::tempdir().unwrap();
    for statement in [
        "theorem t : True := trivial\n",
        "def Conjecture : Prop := True\n",
    ] {
        let error = create(
            &root.path().join("x"),
            statement,
            &program(),
            &toolkit().registry(),
        )
        .unwrap_err();
        assert!(matches!(error, Error::InvalidStatement(_)), "{error:?}");
    }
    assert!(!root.path().join("x").exists());
}

#[test]
fn an_unwritable_folder_is_an_io_error() {
    let root = tempfile::tempdir().unwrap();
    let blocker = root.path().join("blocker");
    std::fs::write(&blocker, "").unwrap();
    let error = create(
        &blocker.join("demo"),
        STATEMENT,
        &program(),
        &toolkit().registry(),
    )
    .unwrap_err();
    assert!(matches!(error, Error::Io { .. }), "{error:?}");
}

#[test]
fn the_subject_package_matches_the_lean_service() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let toolchain = |path: &Path| std::fs::read_to_string(path).unwrap().trim().to_owned();
    assert_eq!(
        toolchain(&workspace.join("docker/lean/lean-toolchain")),
        toolchain(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("template/subject/lean/lean-toolchain")
        )
    );
    let server = std::fs::read_to_string(workspace.join("docker/lean/server.py")).unwrap();
    let lakefile = TEMPLATE
        .iter()
        .find(|(path, _)| *path == "subject/lean/lakefile.toml")
        .unwrap()
        .1;
    assert!(
        server.contains(&format!("LAKEFILE = \"\"\"{lakefile}\"\"\"")),
        "the template's lakefile.toml must be the one the service writes"
    );
}
