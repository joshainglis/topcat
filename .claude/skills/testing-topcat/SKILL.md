---
name: testing-topcat
description: Guides testing and debugging of Topcat including running tests, writing new test cases, and debugging issues. Use when working with Topcat's test suite, debugging dependency problems, analyzing performance, or setting up continuous integration.
---

# Testing Topcat

## Quick Commands

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output for debugging
cargo test -- --nocapture

# Run single-threaded (for flaky tests)
cargo test -- --test-threads=1

# Run tests for specific module
cargo test file_node::
cargo test sql_parser::
```

## Debugging Workflow

Use this checklist when debugging issues:

```
Debug Checklist:
- [ ] Run with verbose mode: topcat -v
- [ ] Check debug output: RUST_LOG=debug cargo run
- [ ] Visualize graph: Generate DOT file and render
- [ ] Test single file: Isolate problematic file
- [ ] Review error context: Check full error message
```

### Verbose Mode

```bash
# See detailed processing
cargo run -- -i input/ -o output.sql -v

# Save debug output
cargo run -- -i input/ -o output.sql -v 2> debug.log
```

### Visualize Dependencies

```bash
# Generate and view dependency graph
cargo run -- -i input/ -o output.sql -v 2>&1 | \
  grep "digraph" -A 1000 > graph.dot
dot -Tpng graph.dot -o graph.png
open graph.png  # View the graph
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

- **Unit tests**: In each module file
- **Integration tests**: In `tests/` directory
- **Test data**: In `tests/input/` subdirectories
- **Benchmarks**: Using `cargo bench`

## Troubleshooting

For common issues and solutions, see [reference/troubleshooting.md](reference/troubleshooting.md).

For CI/CD setup, see [reference/ci.md](reference/ci.md).