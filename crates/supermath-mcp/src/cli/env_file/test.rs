//! Env-file parsing and reading.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::EnvFile;
use crate::error::Error;

#[test]
fn parses_assignments_and_skips_the_rest() {
    let file = EnvFile::parse(
        "# a comment\n\
         \n\
         LEAN_URL=http://127.0.0.1:3150\n\
         export CORTEX_API_KEY=\"supermath-local\"\n\
         SINGLE='quoted value'\n\
         MIXED=\"half'\n\
         EMPTY=\n\
         not an assignment\n\
         =nameless\n\
         SPACED = padded \n",
    );
    assert_eq!(file.get("LEAN_URL"), Some("http://127.0.0.1:3150"));
    assert_eq!(file.get("CORTEX_API_KEY"), Some("supermath-local"));
    assert_eq!(file.get("SINGLE"), Some("quoted value"));
    assert_eq!(file.get("MIXED"), Some("\"half'"));
    assert_eq!(file.get("EMPTY"), Some(""));
    assert_eq!(file.get("SPACED"), Some("padded"));
    assert_eq!(file.get(""), None);
    assert_eq!(file.get("missing"), None);
}

#[test]
fn reads_a_file_from_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("supermath.env");
    std::fs::write(&path, "PYTHON_URL=http://127.0.0.1:3160\n").unwrap();
    let file = EnvFile::read(&path).unwrap();
    assert_eq!(file.get("PYTHON_URL"), Some("http://127.0.0.1:3160"));
}

#[test]
fn a_missing_file_is_an_env_file_error() {
    let dir = tempfile::tempdir().unwrap();
    let error = EnvFile::read(&dir.path().join("absent.env")).unwrap_err();
    assert!(matches!(error, Error::EnvFile { .. }), "{error:?}");
    assert!(error.to_string().starts_with("env file "), "{error}");
}
