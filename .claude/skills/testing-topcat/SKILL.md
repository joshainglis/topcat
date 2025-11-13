---
name: testing-topcat
description: Guides testing and debugging of Topcat including running tests, writing new test cases, and debugging issues. Use when working with Topcat's test suite, testing analysis commands, debugging dependency problems, analyzing performance, or setting up continuous integration.
---

# Testing Topcat

## Quick Commands

```bash
# Run all tests (83 total: 40 unit + 25 analysis + 11 clean + 7 schema)
cargo test

# Run specific test file
cargo test --test analysis_tests
cargo test --test clean_tests
cargo test --test schema_tests

# Run specific test
cargo test test_name

# Run with output for debugging
cargo test -- --nocapture

# Run single-threaded (for flaky tests)
cargo test -- --test-threads=1

# Run tests for specific module
cargo test file_node::
cargo test analysis::
cargo test commands::analyze
```

## Debugging Workflow

Use this checklist when debugging issues:

```
Debug Checklist:
- [ ] Run analysis commands: topcat analyze cycles, missing
- [ ] Export and visualize: topcat export -o graph.dot dot
- [ ] Check debug output: RUST_LOG=debug cargo run
- [ ] Analyze specific file: topcat analyze file path.sql
- [ ] Test with verbose: topcat concat -v or RUST_LOG=topcat::file_dag=debug
- [ ] Review error context: Check full error message
```

### Analysis Commands for Debugging

```bash
# Check for cycles
./target/debug/topcat analyze -i tests/input/sql -e sql cycles

# Check for missing dependencies
./target/debug/topcat analyze -i tests/input/sql -e sql missing

# Analyze specific file
./target/debug/topcat analyze -i tests/input/sql -e sql file tests/input/sql/my_file.sql

# Find dead code
./target/debug/topcat analyze -i tests/input/sql -e sql dead-branches
```

### Visualize Dependencies

```bash
# Modern approach: Use export command
./target/debug/topcat export -i tests/input/sql -e sql -o /tmp/graph.dot dot
dot -Tpng /tmp/graph.dot -o /tmp/graph.png
open /tmp/graph.png

# Legacy approach: Extract from verbose output
cargo run -- concat -i input/ -o output.sql -v 2>&1 | \
  grep "digraph" -A 1000 > graph.dot
dot -Tpng graph.dot -o graph.png
```

### Debug Logging

```bash
# Debug specific module
RUST_LOG=topcat::file_dag=debug cargo run -- analyze -i sql/ -e sql cycles

# Debug analysis
RUST_LOG=topcat::analysis=debug cargo run -- analyze -i sql/ -e sql dead-branches

# Full debug output
RUST_LOG=topcat=debug cargo run -- concat -i sql/ -o output.sql
```

## Writing Tests

### Quick Test Template

```rust
#[test]
fn test_feature() {
    // Arrange
    let input = create_test_input();

    // Act
    let result = function_under_test(input);

    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);
}
```

For detailed templates, see [reference/writing-tests.md](reference/writing-tests.md).

## Common Test Scenarios

### Integration Tests with TempDir

**CRITICAL**: TempDir creates directories starting with a dot (e.g., `.tmpXXXX`), which topcat treats as hidden by default!

```rust
#[test]
fn test_analysis() {
    let dir = TempDir::new().unwrap();

    // Create test files
    fs::write(dir.path().join("a.sql"), "-- name: a").unwrap();

    // MUST set include_hidden: true for TempDir!
    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        include_extensions: Some(&extensions),
        include_hidden: true,  // CRITICAL for TempDir
        ..Default::default()
    };

    let graph = TCGraph::build_graph(&config).unwrap();
    assert!(graph.nodes.contains_key("a"));
}
```

### Dependency Chain

```rust
#[test]
fn test_dependency_order() {
    let files = vec![
        ("a.sql", "-- name: a\n-- requires: b"),
        ("b.sql", "-- name: b"),
    ];

    let sorted = process_files(files);
    assert_eq!(sorted, vec!["b", "a"]);
}
```

### Analysis Tests

```rust
#[test]
fn test_find_orphans() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("orphan.sql"), "-- name: orphan").unwrap();
    fs::write(dir.path().join("a.sql"), "-- name: a\n-- requires: b").unwrap();
    fs::write(dir.path().join("b.sql"), "-- name: b").unwrap();

    let extensions = vec!["sql".to_string()];
    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        include_extensions: Some(&extensions),
        include_hidden: true,  // Required for TempDir
        ..Default::default()
    };

    let graph = TCGraph::build_graph(&config).unwrap();
    let orphans = graph.find_orphans(None);

    assert!(orphans.contains("orphan"));
    assert!(!orphans.contains("a"));
}
```

### Schema Tests

```rust
#[test]
fn test_schema_extraction() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.sql"), "-- name: auth.users").unwrap();
    fs::write(dir.path().join("b.sql"), "-- name: billing.invoices").unwrap();

    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        include_extensions: Some(&vec!["sql".to_string()]),
        include_hidden: true,
        ..Default::default()
    };

    let graph = TCGraph::build_graph(&config).unwrap();
    let schemas = graph.get_schema_names();

    assert!(schemas.contains(&"auth".to_string()));
    assert!(schemas.contains(&"billing".to_string()));
}
```

### Cycle Detection

```rust
#[test]
fn test_cycle_error() {
    let files = vec![
        ("a.sql", "-- name: a\n-- requires: b"),
        ("b.sql", "-- name: b\n-- requires: a"),
    ];

    let result = process_files(files);
    assert!(matches!(result, Err(TopCatError::CycleDetected{..})));
}
```

## Performance Analysis

### Quick Profiling

```bash
# Time execution
time cargo run --release -- -i large_input/ -o output.sql

# Memory usage
/usr/bin/time -l cargo run -- -i input/ -o output.sql
```

For detailed profiling, see [reference/performance.md](reference/performance.md).

## Test Organization

- **Unit tests**: In each module file (40 tests total)
  - `file_node.rs` - schema extraction tests
  - `analysis/root_matcher.rs` - pattern matching tests
  - Other modules with unit tests

- **Integration tests**: In `tests/` directory (43 tests total)
  - `tests/analysis_tests.rs` - 25 analysis tests (cycles, missing, dead branches, orphans, etc.)
  - `tests/clean_tests.rs` - 11 cleanup tests (safe deletion, root protection, dependency validation)
  - `tests/schema_tests.rs` - 7 schema tests (extraction, grouping, cross-schema deps)
  - Other existing integration tests

- **Test data**: In `tests/input/` subdirectories
  - `tests/input/sql/` - SQL test files with various dependency patterns

- **Benchmarks**: Using `cargo bench`

**Total test count**: 83 tests (as of Phase 6)

## Troubleshooting

For common issues and solutions, see [reference/troubleshooting.md](reference/troubleshooting.md).

For CI/CD setup, see [reference/ci.md](reference/ci.md).