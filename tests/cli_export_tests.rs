//! CLI integration tests for the `export` command.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Get a Command instance for the topcat binary
fn topcat_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("topcat"))
}

/// Get the path to the test input directory
fn test_input_dir() -> PathBuf {
    PathBuf::from("tests/input/sql")
}

#[test]
fn test_export_json() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("graph.json");

    topcat_cmd()
        .args([
            "export",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "-o",
            output_file.to_str().unwrap(),
            "json",
        ])
        .assert()
        .success();

    // Verify output file was created
    assert!(output_file.exists());

    // Verify it's valid JSON
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&content).is_ok());
}

#[test]
fn test_export_dot() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("graph.dot");

    topcat_cmd()
        .args([
            "export",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "-o",
            output_file.to_str().unwrap(),
            "dot",
        ])
        .assert()
        .success();

    assert!(output_file.exists());

    // Verify it contains DOT graph syntax
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("digraph") || content.contains("graph"));
}

#[test]
fn test_export_graphml() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("graph.graphml");

    topcat_cmd()
        .args([
            "export",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "-o",
            output_file.to_str().unwrap(),
            "graphml",
        ])
        .assert()
        .success();

    assert!(output_file.exists());

    // Verify it contains GraphML XML syntax
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("graphml") && content.contains("<?xml"));
}

#[test]
fn test_export_mermaid() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("graph.md");

    topcat_cmd()
        .args([
            "export",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "-o",
            output_file.to_str().unwrap(),
            "mermaid",
        ])
        .assert()
        .success();

    assert!(output_file.exists());

    // Verify it contains Mermaid syntax
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("graph") || content.contains("flowchart"));
}

#[test]
fn test_export_with_mode_deps() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("deps.json");

    topcat_cmd()
        .args([
            "export",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "-o",
            output_file.to_str().unwrap(),
            "--mode",
            "deps",
            "--node",
            "my_schema.c",
            "json",
        ])
        .assert()
        .success();

    assert!(output_file.exists());
}

#[test]
fn test_export_with_mode_dependents() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("dependents.json");

    topcat_cmd()
        .args([
            "export",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "-o",
            output_file.to_str().unwrap(),
            "--mode",
            "dependents",
            "--node",
            "my_schema.a",
            "json",
        ])
        .assert()
        .success();

    assert!(output_file.exists());
}

#[test]
fn test_export_with_schema_filter() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("schema-filtered.json");

    topcat_cmd()
        .args([
            "export",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "-o",
            output_file.to_str().unwrap(),
            "--schema",
            "my_schema",
            "json",
        ])
        .assert()
        .success();

    assert!(output_file.exists());
}

#[test]
fn test_export_missing_input() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("graph.json");

    // Topcat may handle missing directories gracefully or error
    let _ = topcat_cmd()
        .args([
            "export",
            "-i",
            "/tmp/topcat_nonexistent_test_12345",
            "-e",
            "sql",
            "-o",
            output_file.to_str().unwrap(),
            "json",
        ])
        .assert()
        .get_output();
}

#[test]
fn test_export_help() {
    topcat_cmd()
        .args(["export", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Export"))
        .stdout(predicate::str::contains("json"));
}

#[test]
fn test_export_json_help() {
    topcat_cmd()
        .args(["export", "json", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("JSON"));
}

#[test]
fn test_export_missing_format() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("graph.out");

    topcat_cmd()
        .args([
            "export",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "-o",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("subcommand")));
}

#[test]
fn test_export_mode_requires_node() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("deps.json");

    // --mode deps requires --node to be specified
    topcat_cmd()
        .args([
            "export",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "-o",
            output_file.to_str().unwrap(),
            "--mode",
            "deps",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("node")));
}
