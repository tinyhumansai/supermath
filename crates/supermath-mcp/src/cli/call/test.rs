//! One call as one process: arguments from text, the answer body.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::call;
use crate::{Config, Toolkit};

fn toolkit(dir: &tempfile::TempDir) -> Toolkit {
    let mut config = Config::from_lookup(|_| None).unwrap();
    config.files.project_dir = Some(dir.path().to_path_buf());
    Toolkit::compose_persistent(&config, "demo").unwrap()
}

#[tokio::test]
async fn answers_the_envelope_with_its_call_id() {
    let dir = tempfile::tempdir().unwrap();
    let toolkit = toolkit(&dir);
    let body = call(&toolkit, "calc_factor", r#"{"n": 360}"#).await;
    assert_eq!(body["outcome"], "ok", "{body}");
    let call_id = body["call_id"].as_str().unwrap();
    assert!(
        call_id.starts_with("call-") && call_id.ends_with("-1"),
        "{call_id}"
    );
    assert!(toolkit.call_log().get(call_id).is_some());
    assert!(dir.path().join(".log/calls.jsonl").is_file());
}

#[tokio::test]
async fn empty_input_is_an_empty_argument_object() {
    let dir = tempfile::tempdir().unwrap();
    let body = call(&toolkit(&dir), "file_list", " \n").await;
    assert_eq!(body["outcome"], "ok", "{body}");
}

#[tokio::test]
async fn an_error_envelope_is_still_an_answer() {
    let dir = tempfile::tempdir().unwrap();
    let body = call(&toolkit(&dir), "no_such_tool", "{}").await;
    assert_eq!(body["outcome"], "error");
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert!(body["call_id"].is_string(), "the failed call was logged");
}

#[tokio::test]
async fn input_that_is_not_an_object_never_reaches_a_tool() {
    let dir = tempfile::tempdir().unwrap();
    let toolkit = toolkit(&dir);
    for (input, message) in [("[1]", "must be a JSON object"), ("{", "not JSON")] {
        let body = call(&toolkit, "calc_factor", input).await;
        assert_eq!(body["outcome"], "error", "{body}");
        assert_eq!(body["error"]["code"], "INVALID_INPUT");
        assert!(
            body["error"]["message"].as_str().unwrap().contains(message),
            "{body}"
        );
        assert_eq!(body.get("call_id"), None);
    }
    assert_eq!(toolkit.call_log().all(), []);
}
