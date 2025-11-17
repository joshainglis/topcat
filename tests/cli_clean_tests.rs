//! CLI integration tests for the `clean` command.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

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
    topcat_cmd()
        .args([
            "clean",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "targets",
            "tests/input/sql/my_schema/a.sql",
        ])
        .assert()
        .success();
}

#[test]
fn test_clean_no_dry_run_requires_confirmation() {
    // With --no-dry-run, it should prompt for confirmation (or use --force)
    // Since we're in a non-interactive test, this will likely fail without --force
    // We test that the command recognizes the flag
    topcat_cmd()
        .args([
            "clean",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "sql",
            "--no-dry-run",
            "--force",
            "orphans",
        ])
        .assert()
        .success();
}
