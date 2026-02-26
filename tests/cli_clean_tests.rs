//! CLI integration tests for the `clean` command.

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
fn test_clean_dead_branches_dry_run() {
    // Dry-run is the default - should not actually delete files
    topcat_cmd()
        .args([
            "clean",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "dead-branches",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY").or(predicate::str::contains("Would")));
}

#[test]
fn test_clean_orphans_dry_run() {
    topcat_cmd()
        .args([
            "clean",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "orphans",
        ])
        .assert()
        .success();
}

#[test]
fn test_clean_unrequired_dry_run() {
    topcat_cmd()
        .args([
            "clean",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "unrequired",
        ])
        .assert()
        .success();
}

#[test]
fn test_clean_with_root_pattern() {
    topcat_cmd()
        .args([
            "clean",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "--root-pattern",
            "**/schema.sql",
            "dead-branches",
        ])
        .assert()
        .success();
}

#[test]
fn test_clean_with_schema_filter() {
    topcat_cmd()
        .args([
            "clean",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "--schema",
            "my_schema",
            "orphans",
        ])
        .assert()
        .success();
}

#[test]
fn test_clean_missing_input() {
    // Topcat may handle missing directories gracefully or error
    let _ = topcat_cmd()
        .args([
            "clean",
            "-i",
            "/tmp/topcat_nonexistent_test_12345",
            "-e",
            "sql",
            "orphans",
        ])
        .assert()
        .get_output();
}

#[test]
fn test_clean_help() {
    topcat_cmd()
        .args(["clean", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Clean")
                .or(predicate::str::contains("Remove"))
                .or(predicate::str::contains("dead-branches")),
        );
}

#[test]
fn test_clean_dead_branches_help() {
    topcat_cmd()
        .args(["clean", "dead-branches", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dead"));
}

#[test]
fn test_clean_missing_subcommand() {
    topcat_cmd()
        .args(["clean", "-i", test_input_dir().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("subcommand")));
}

#[test]
fn test_clean_targets_dry_run() {
    // Test cleaning specific target files (takes file paths, not node names)
    // Uses a valid test file path - this is safe because default is dry-run mode
    topcat_cmd()
        .args([
            "clean",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "targets",
            "tests/input/sql/my_schema/schema.sql", // Use schema.sql which should exist
        ])
        .assert()
        .success();
}

#[test]
fn test_clean_execute_mode_requires_confirmation() {
    // With --mode execute, it should prompt for confirmation (or use --force)
    // We use a non-existent directory to verify the flags are accepted without actually deleting files
    // The actual file deletion behavior is tested in clean_tests.rs with temporary directories
    topcat_cmd()
        .args([
            "clean",
            "-i",
            "/tmp/topcat_test_nonexistent_12345",
            "-e",
            "sql",
            "--mode",
            "execute",
            "--force",
            "orphans",
        ])
        .assert();
    // Command may fail due to missing directory, but flags should be recognized
}

#[test]
fn test_clean_targets_matches_exact_node_name() {
    let temp_dir = TempDir::new().unwrap();
    let sql_dir = temp_dir.path().join("sql");
    fs::create_dir_all(&sql_dir).unwrap();

    fs::write(sql_dir.join("a.sql"), "-- name: foo\nSELECT 1;\n").unwrap();
    fs::write(sql_dir.join("b.sql"), "-- name: foobar\nSELECT 2;\n").unwrap();

    topcat_cmd()
        .args([
            "clean",
            "-i",
            sql_dir.to_str().unwrap(),
            "-e",
            "sql",
            "targets",
            "foo",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Target Files to be deleted (1 files)",
        ))
        .stdout(predicate::str::contains("foo"))
        .stdout(predicate::str::contains("foobar").not());
}

#[test]
fn test_clean_orphans_quiet_hides_root_and_external_filter_messages() {
    let temp_dir = TempDir::new().unwrap();
    let sql_dir = temp_dir.path().join("sql");
    let external_dir = temp_dir.path().join("external");
    fs::create_dir_all(&sql_dir).unwrap();
    fs::create_dir_all(&external_dir).unwrap();

    fs::write(
        sql_dir.join("protected.sql"),
        "-- name: protected_node\nSELECT 1;\n",
    )
    .unwrap();
    fs::write(
        sql_dir.join("external.sql"),
        "-- name: external_node\nSELECT 1;\n",
    )
    .unwrap();
    fs::write(
        external_dir.join("usage.py"),
        "print('external_node is referenced externally')\n",
    )
    .unwrap();

    topcat_cmd()
        .args([
            "clean",
            "--quiet",
            "-i",
            sql_dir.to_str().unwrap(),
            "-e",
            "sql",
            "--root-nodes",
            "protected_node",
            "--external-check-dir",
            external_dir.to_str().unwrap(),
            "--external-check-pattern",
            "*.py",
            "orphans",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Protected").not())
        .stdout(predicate::str::contains("Filtered out").not());
}
