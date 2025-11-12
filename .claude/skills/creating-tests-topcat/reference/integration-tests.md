# Integration Tests

## Overview

Integration tests in Topcat are located in the `tests/` directory and test complete workflows involving multiple components. They use real files (via TempDir) and test end-to-end functionality.

## Structure

```
tests/
├── analysis_tests.rs      # Tests for analysis commands (orphans, dead branches, etc.)
├── clean_tests.rs          # Tests for file cleanup operations
└── schema_tests.rs         # Tests for schema-aware analysis
```

## Standard Integration Test Pattern

### Complete Example

```rust
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use topcat::config::Config;
use topcat::file_dag::TCGraph;
use topcat::analysis::orphans::OrphanAnalyzer;

// Helper: Create a test file in TempDir
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

// Helper: Build graph from test directory
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

#[test]
fn test_find_orphans_simple() {
    // Arrange: Set up test directory with files
    let dir = TempDir::new().unwrap();

    create_test_file(
        &dir,
        "orphan.sql",
        "-- name: orphan\nCREATE TABLE orphan();",
    );

    create_test_file(
        &dir,
        "a.sql",
        "-- name: a\n-- requires: b\nCREATE TABLE a();",
    );

    create_test_file(
        &dir,
        "b.sql",
        "-- name: b\nCREATE TABLE b();",
    );

    // Act: Build graph and find orphans
    let graph = build_test_graph(&dir);
    let analyzer = OrphanAnalyzer::new(&graph);
    let orphans = analyzer.find_orphans().unwrap();

    // Assert: Check results
    assert_eq!(orphans.len(), 1);
    assert!(orphans.contains(&"orphan".to_string()));
    assert!(!orphans.contains(&"a".to_string()));
    assert!(!orphans.contains(&"b".to_string()));
}
```

## Common Test Scenarios

### Testing Dependency Analysis

```rust
#[test]
fn test_dead_branches_detection() {
    let dir = TempDir::new().unwrap();

    // Create root that nothing depends on
    create_test_file(
        &dir,
        "dead.sql",
        "-- name: dead\n-- requires: helper\nCREATE TABLE dead();",
    );

    create_test_file(
        &dir,
        "helper.sql",
        "-- name: helper\nCREATE TABLE helper();",
    );

    // Create main dependency chain
    create_test_file(
        &dir,
        "main.sql",
        "-- name: main\n-- requires: dep\nCREATE TABLE main();",
    );

    create_test_file(
        &dir,
        "dep.sql",
        "-- name: dep\nCREATE TABLE dep();",
    );

    let graph = build_test_graph(&dir);
    let analyzer = DeadBranchAnalyzer::new(&graph, vec!["main".to_string()]);
    let dead_branches = analyzer.find_dead_branches().unwrap();

    assert_eq!(dead_branches.len(), 2);
    assert!(dead_branches.contains(&"dead".to_string()));
    assert!(dead_branches.contains(&"helper".to_string()));
}
```

### Testing Topological Sort

```rust
#[test]
fn test_topological_sort_respects_dependencies() {
    let dir = TempDir::new().unwrap();

    create_test_file(
        &dir,
        "a.sql",
        "-- name: a\n-- requires: b c\nCREATE TABLE a();",
    );

    create_test_file(
        &dir,
        "b.sql",
        "-- name: b\n-- requires: c\nCREATE TABLE b();",
    );

    create_test_file(
        &dir,
        "c.sql",
        "-- name: c\nCREATE TABLE c();",
    );

    let graph = build_test_graph(&dir);
    let sorted = graph.topological_sort().unwrap();

    let names: Vec<String> = sorted.iter().map(|n| n.name.clone()).collect();

    // Find positions
    let pos_c = names.iter().position(|n| n == "c").unwrap();
    let pos_b = names.iter().position(|n| n == "b").unwrap();
    let pos_a = names.iter().position(|n| n == "a").unwrap();

    // Verify order: c before b, b before a
    assert!(pos_c < pos_b, "c must come before b");
    assert!(pos_b < pos_a, "b must come before a");
}
```

### Testing Layer System

```rust
#[test]
fn test_layers_enforce_ordering() {
    let dir = TempDir::new().unwrap();

    create_test_file(
        &dir,
        "append.sql",
        "-- name: append_file\n-- layer: append\nCREATE VIEW append_file AS SELECT 1;",
    );

    create_test_file(
        &dir,
        "normal.sql",
        "-- name: normal_file\n-- layer: normal\nCREATE TABLE normal_file();",
    );

    create_test_file(
        &dir,
        "prepend.sql",
        "-- name: prepend_file\n-- layer: prepend\nCREATE EXTENSION IF NOT EXISTS uuid;",
    );

    let graph = build_test_graph(&dir);
    let sorted = graph.topological_sort().unwrap();

    let names: Vec<String> = sorted.iter().map(|n| n.name.clone()).collect();

    let pos_prepend = names.iter().position(|n| n == "prepend_file").unwrap();
    let pos_normal = names.iter().position(|n| n == "normal_file").unwrap();
    let pos_append = names.iter().position(|n| n == "append_file").unwrap();

    // Verify layer ordering
    assert!(pos_prepend < pos_normal);
    assert!(pos_normal < pos_append);
}
```

### Testing Cycle Detection

```rust
#[test]
fn test_cycle_detection() {
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
    let result = graph.build_graph();

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
```

### Testing Schema-Aware Analysis

```rust
#[test]
fn test_schema_filtering() {
    let dir = TempDir::new().unwrap();

    create_test_file(
        &dir,
        "schema_a/table1.sql",
        "-- name: schema_a.table1\nCREATE TABLE schema_a.table1();",
    );

    create_test_file(
        &dir,
        "schema_a/table2.sql",
        "-- name: schema_a.table2\n-- requires: schema_a.table1\nCREATE TABLE schema_a.table2();",
    );

    create_test_file(
        &dir,
        "schema_b/table1.sql",
        "-- name: schema_b.table1\nCREATE TABLE schema_b.table1();",
    );

    let graph = build_test_graph(&dir);

    // Get all nodes from schema_a
    let schema_a_nodes: Vec<_> = graph
        .nodes()
        .filter(|n| n.name.starts_with("schema_a."))
        .collect();

    assert_eq!(schema_a_nodes.len(), 2);
}
```

### Testing File Cleanup

```rust
#[test]
fn test_delete_orphans() {
    let dir = TempDir::new().unwrap();

    create_test_file(&dir, "orphan.sql", "-- name: orphan\nCREATE TABLE orphan();");
    create_test_file(&dir, "a.sql", "-- name: a\n-- requires: b\nCREATE TABLE a();");
    create_test_file(&dir, "b.sql", "-- name: b\nCREATE TABLE b();");

    let graph = build_test_graph(&dir);
    let analyzer = OrphanAnalyzer::new(&graph);
    let orphans = analyzer.find_orphans().unwrap();

    // Delete orphan files
    for orphan_name in &orphans {
        if let Some(node) = graph.get_node_by_name(orphan_name) {
            fs::remove_file(&node.path).unwrap();
        }
    }

    // Verify file was deleted
    let orphan_path = dir.path().join("orphan.sql");
    assert!(!orphan_path.exists());

    // Verify other files still exist
    assert!(dir.path().join("a.sql").exists());
    assert!(dir.path().join("b.sql").exists());
}
```

## Test Helpers Reference

### create_test_file

```rust
fn create_test_file(dir: &TempDir, path: &str, content: &str) {
    let file_path = dir.path().join(path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let content_with_newline = format!("{content}\n");
    fs::write(&file_path, content_with_newline).unwrap();
}
```

**Usage**:
- `path`: Relative path within TempDir (e.g., "schema/table.sql")
- `content`: File content without trailing newline (added automatically)
- Creates parent directories as needed
- Always adds newline to match real file behavior

### build_test_graph

```rust
fn build_test_graph(dir: &TempDir) -> TCGraph {
    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        // ... standard test config
    };
    let mut graph = TCGraph::new(&config);
    graph.build_graph().unwrap();
    graph
}
```

**Usage**:
- Takes TempDir reference
- Returns built graph ready for testing
- Uses `unwrap()` (tests should panic on build failure)

### Custom Config Builder

```rust
fn build_graph_with_config(dir: &TempDir, layers: &[&str]) -> TCGraph {
    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        include_file_extensions: Some(&["sql"]),
        comment_str: "--",
        layers,
        ..Default::default()
    };
    let mut graph = TCGraph::new(&config);
    graph.build_graph().unwrap();
    graph
}
```

## Debugging Integration Tests

### Run with logging

```bash
RUST_LOG=topcat=debug cargo test test_name -- --nocapture
RUST_LOG=topcat::file_dag=trace cargo test test_name -- --nocapture
```

### Print TempDir contents

```rust
#[test]
fn test_with_debug() {
    let dir = TempDir::new().unwrap();
    create_test_file(&dir, "test.sql", "-- name: test");

    // Print directory contents
    println!("Test directory: {:?}", dir.path());
    for entry in fs::read_dir(dir.path()).unwrap() {
        println!("  {:?}", entry.unwrap().path());
    }

    // ... rest of test
}
```

### Keep TempDir for inspection

```rust
#[test]
fn test_persist_temp_dir() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    // Prevent automatic cleanup
    let _ = dir.into_path();

    println!("Test files at: {:?}", path);

    // Run test...
    // Manually inspect files after test
}
```

## Best Practices

### DO

✓ Use TempDir for all file operations
✓ Test complete workflows (end-to-end)
✓ Test with realistic file structures
✓ Verify both positive and negative cases
✓ Use descriptive test names
✓ Clean up is automatic (TempDir drops)
✓ Test error propagation
✓ Verify file system state changes

### DON'T

✗ Write to non-temp locations
✗ Depend on test execution order
✗ Share state between tests
✗ Leave files around after tests
✗ Use hardcoded paths
✗ Skip negative test cases

## Running Integration Tests

```bash
# All integration tests
cargo test --test analysis_tests
cargo test --test clean_tests
cargo test --test schema_tests

# Specific test
cargo test --test analysis_tests test_find_orphans

# With output
cargo test --test analysis_tests test_name -- --nocapture

# With logging
RUST_LOG=topcat::file_dag=debug cargo test --test analysis_tests -- --nocapture
```
