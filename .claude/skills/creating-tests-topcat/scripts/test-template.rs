// Integration Test Template for Topcat
//
// This file provides a complete template for creating new integration tests.
// Copy this file to tests/your_test_name.rs and modify as needed.

use std::fs;
use tempfile::TempDir;
use topcat::config::Config;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::file_node::FileNode;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test file in the TempDir with automatic directory creation
fn create_test_file(dir: &TempDir, path: &str, content: &str) {
    let file_path = dir.path().join(path);

    // Create parent directories if needed
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    // Add newline to match real file behavior
    let content_with_newline = format!("{content}\n");
    fs::write(&file_path, content_with_newline).unwrap();
}

/// Build a test graph from a TempDir with standard configuration
fn build_test_graph(dir: &TempDir) -> TCGraph {
    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        output_file: None,
        include_file_extensions: Some(&["sql"]),
        exclude_file_extensions: None,
        include_globs: None,
        exclude_globs: None,
        include_prefix: None,
        exclude_prefix: None,
        subdir_filter: None,
        comment_str: "--",
        append_str: None,
        layers: &["prepend", "normal", "append"],
        sql_config: None,
        enable_sql_discovery: false,
        update_headers: false,
    };

    let mut graph = TCGraph::new(&config);
    graph.build_graph().unwrap();
    graph
}

/// Helper to verify topological order between two nodes
fn assert_order(sorted: &[FileNode], before: &str, after: &str) {
    let names: Vec<String> = sorted.iter().map(|n| n.name.clone()).collect();

    let pos_before = names
        .iter()
        .position(|n| n == before)
        .expect(&format!("Node '{}' not found in sorted list", before));

    let pos_after = names
        .iter()
        .position(|n| n == after)
        .expect(&format!("Node '{}' not found in sorted list", after));

    assert!(
        pos_before < pos_after,
        "'{}' should come before '{}', but positions are {} and {}",
        before,
        after,
        pos_before,
        pos_after
    );
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_simple_case() {
    // Arrange
    let dir = TempDir::new().unwrap();

    create_test_file(&dir, "file.sql", "-- name: test_node\nCREATE TABLE test();");

    // Act
    let graph = build_test_graph(&dir);

    // Assert
    assert_eq!(graph.node_count(), 1);
}

#[test]
fn test_dependency_ordering() {
    // Arrange
    let dir = TempDir::new().unwrap();

    create_test_file(
        &dir,
        "a.sql",
        "-- name: a\n-- requires: b\nCREATE TABLE a();",
    );

    create_test_file(&dir, "b.sql", "-- name: b\nCREATE TABLE b();");

    // Act
    let graph = build_test_graph(&dir);
    let sorted = graph.topological_sort().unwrap();

    // Assert
    assert_eq!(graph.node_count(), 2);
    assert_order(&sorted, "b", "a");
}

#[test]
fn test_complex_dependencies() {
    // Arrange
    let dir = TempDir::new().unwrap();

    create_test_file(
        &dir,
        "a.sql",
        "-- name: a\n-- requires: b c\nCREATE TABLE a();",
    );

    create_test_file(
        &dir,
        "b.sql",
        "-- name: b\n-- requires: d\nCREATE TABLE b();",
    );

    create_test_file(
        &dir,
        "c.sql",
        "-- name: c\n-- requires: d\nCREATE TABLE c();",
    );

    create_test_file(&dir, "d.sql", "-- name: d\nCREATE TABLE d();");

    // Act
    let graph = build_test_graph(&dir);
    let sorted = graph.topological_sort().unwrap();

    // Assert
    assert_eq!(graph.node_count(), 4);

    // d must come before both b and c
    assert_order(&sorted, "d", "b");
    assert_order(&sorted, "d", "c");

    // b and c must come before a
    assert_order(&sorted, "b", "a");
    assert_order(&sorted, "c", "a");
}

#[test]
fn test_layer_ordering() {
    // Arrange
    let dir = TempDir::new().unwrap();

    create_test_file(
        &dir,
        "append.sql",
        "-- name: append_file\n-- layer: append\nCREATE VIEW v AS SELECT 1;",
    );

    create_test_file(
        &dir,
        "normal.sql",
        "-- name: normal_file\n-- layer: normal\nCREATE TABLE t();",
    );

    create_test_file(
        &dir,
        "prepend.sql",
        "-- name: prepend_file\n-- layer: prepend\nCREATE EXTENSION uuid;",
    );

    // Act
    let graph = build_test_graph(&dir);
    let sorted = graph.topological_sort().unwrap();

    // Assert
    assert_order(&sorted, "prepend_file", "normal_file");
    assert_order(&sorted, "normal_file", "append_file");
}

#[test]
fn test_missing_dependency_error() {
    // Arrange
    let dir = TempDir::new().unwrap();

    create_test_file(
        &dir,
        "a.sql",
        "-- name: a\n-- requires: missing\nCREATE TABLE a();",
    );

    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        include_file_extensions: Some(&["sql"]),
        comment_str: "--",
        layers: &["normal"],
        ..Default::default()
    };

    let mut graph = TCGraph::new(&config);

    // Act
    let result = graph.build_graph();

    // Assert
    assert!(result.is_err());

    match result {
        Err(TopCatError::MissingDependency(from, to)) => {
            assert_eq!(from, "a");
            assert_eq!(to, "missing");
        }
        _ => panic!("Expected MissingDependency error"),
    }
}

#[test]
fn test_cycle_detection() {
    // Arrange
    let dir = TempDir::new().unwrap();

    create_test_file(
        &dir,
        "a.sql",
        "-- name: a\n-- requires: b\nCREATE TABLE a();",
    );

    create_test_file(
        &dir,
        "b.sql",
        "-- name: b\n-- requires: c\nCREATE TABLE b();",
    );

    create_test_file(
        &dir,
        "c.sql",
        "-- name: c\n-- requires: a\nCREATE TABLE c();",
    );

    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        include_file_extensions: Some(&["sql"]),
        comment_str: "--",
        layers: &["normal"],
        ..Default::default()
    };

    let mut graph = TCGraph::new(&config);

    // Act
    let result = graph.build_graph();

    // Assert
    assert!(result.is_err());

    match result {
        Err(TopCatError::CyclicDependency(cycle)) => {
            assert!(cycle.len() >= 3);
            assert!(cycle.contains(&"a".to_string()));
            assert!(cycle.contains(&"b".to_string()));
            assert!(cycle.contains(&"c".to_string()));
        }
        _ => panic!("Expected CyclicDependency error"),
    }
}

#[test]
fn test_nested_directories() {
    // Arrange
    let dir = TempDir::new().unwrap();

    create_test_file(
        &dir,
        "schema/migrations/001.sql",
        "-- name: migration_001\nCREATE SCHEMA test;",
    );

    create_test_file(
        &dir,
        "schema/migrations/002.sql",
        "-- name: migration_002\n-- requires: migration_001\nCREATE TABLE test.users();",
    );

    create_test_file(
        &dir,
        "schema/views/user_view.sql",
        "-- name: user_view\n-- requires: migration_002\nCREATE VIEW user_view AS SELECT * FROM test.users;",
    );

    // Act
    let graph = build_test_graph(&dir);
    let sorted = graph.topological_sort().unwrap();

    // Assert
    assert_eq!(graph.node_count(), 3);
    assert_order(&sorted, "migration_001", "migration_002");
    assert_order(&sorted, "migration_002", "user_view");
}
