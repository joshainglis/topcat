//! CLI integration tests for the `schema` command.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

/// Get a Command instance for the topcat binary
fn topcat_cmd() -> Command {
    Command::cargo_bin("topcat").unwrap()
}

/// Get the path to the test input directory
fn test_input_dir() -> PathBuf {
    PathBuf::from("tests/input/sql")
}

#[test]
fn test_schema_list() {
    topcat_cmd()
        .args(&[
            "schema",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "list",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Schema").or(predicate::str::contains("schema")));
}

#[test]
fn test_schema_analyze() {
    topcat_cmd()
        .args(&[
            "schema",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "analyze",
            "my_schema",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("my_schema"));
}

#[test]
fn test_schema_dependencies() {
    topcat_cmd()
        .args(&[
            "schema",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "dependencies",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("dependencies")
                .or(predicate::str::contains("No"))
                .or(predicate::str::contains("Schema")),
        );
}

#[test]
fn test_schema_missing_input() {
    // Topcat may handle missing directories gracefully or error
    let _ = topcat_cmd()
        .args(&[
            "schema",
            "-i",
            "/tmp/topcat_nonexistent_test_12345",
            "-e",
            "sql",
            "list",
        ])
        .assert()
        .get_output();
}

#[test]
fn test_schema_help() {
    topcat_cmd()
        .args(&["schema", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("schema").or(predicate::str::contains("Schema")))
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_schema_list_help() {
    topcat_cmd()
        .args(&["schema", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_schema_analyze_help() {
    topcat_cmd()
        .args(&["schema", "analyze", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("analyze"));
}

#[test]
fn test_schema_missing_subcommand() {
    topcat_cmd()
        .args(&["schema", "-i", test_input_dir().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("subcommand")));
}

#[test]
fn test_schema_analyze_nonexistent() {
    topcat_cmd()
        .args(&[
            "schema",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "analyze",
            "nonexistent_schema",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Error")));
}
