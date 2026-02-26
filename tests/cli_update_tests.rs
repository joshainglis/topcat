//! CLI integration tests for the `update` command.

use assert_cmd::Command;
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
fn test_update_dry_run_uppercase_extension_filter() {
    topcat_cmd()
        .args([
            "update",
            "-i",
            test_input_dir().to_str().unwrap(),
            "-e",
            "SQL",
            "--mode",
            "dry-run",
        ])
        .assert()
        .success();
}
