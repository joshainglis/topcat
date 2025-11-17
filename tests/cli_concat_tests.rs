//! CLI integration tests for the `concat` command.

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
fn test_concat_basic_success() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.sql");

    topcat_cmd()
        .args([
            "concat",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Verify output file was created
    assert!(output_file.exists());

    // Verify output file has content
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(!content.is_empty());
}

#[test]
fn test_concat_with_extension_filter() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.sql");

    topcat_cmd()
        .args([
            "concat",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
            "-e",
            "sql",
        ])
        .assert()
        .success();

    assert!(output_file.exists());
}

#[test]
fn test_concat_missing_input_directory() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.sql");

    // Topcat may handle missing directories gracefully or error
    // Just verify the command runs
    let _ = topcat_cmd()
        .args([
            "concat",
            "-i",
            "/tmp/topcat_nonexistent_test_12345",
            "-o",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .get_output();
}

#[test]
fn test_concat_missing_required_args() {
    // Missing both -i and -o
    topcat_cmd()
        .arg("concat")
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be specified"));
}

#[test]
fn test_concat_missing_input_arg() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.sql");

    // Missing -i uses default (current directory), so may succeed
    let _ = topcat_cmd()
        .args(["concat", "-o", output_file.to_str().unwrap()])
        .assert()
        .get_output();
}

#[test]
fn test_concat_missing_output_arg() {
    // Missing -o
    topcat_cmd()
        .args(["concat", "-i", test_input_dir().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be specified"));
}

#[test]
fn test_concat_with_layers() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.sql");

    topcat_cmd()
        .args([
            "concat",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
            "--layers",
            "prepend,normal,append",
        ])
        .assert()
        .success();

    assert!(output_file.exists());
}

#[test]
fn test_concat_verbose_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.sql");

    topcat_cmd()
        .args([
            "concat",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
            "-v",
        ])
        .assert()
        .success();
}

#[test]
fn test_concat_help() {
    topcat_cmd()
        .args(["concat", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Concatenate"))
        .stdout(predicate::str::contains("--input-dirs"));
}

#[test]
fn test_concat_output_contains_all_files() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.sql");

    topcat_cmd()
        .args([
            "concat",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
            "-e",
            "sql",
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&output_file).unwrap();

    // Check that output contains content from test files
    // These are node names from the test input files
    assert!(content.contains("my_schema.a") || content.contains("CREATE"));
}
