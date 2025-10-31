# Testing and Debugging Guide

This skill provides comprehensive guidance for testing and debugging Topcat.

## Test Organization

### Module-Level Tests

Tests are organized alongside their respective modules:

- **file_node.rs**: Header parsing, layer handling, dependency extraction
- **file_dag.rs**: Graph construction, validation, cycle detection
- **stable_topo.rs**: Topological sort correctness and stability
- **io_utils.rs**: Directory walking, glob matching, filtering
- **output.rs**: File suffix handling, output generation
- **sql_parser.rs**: SQL parsing, dependency discovery
- **sql_config.rs**: Configuration parsing and merging
- **header_generator.rs**: Header generation and updates

### Integration Tests

Located in `tests/` directory:
- End-to-end workflow tests
- Complex dependency scenarios
- Layer interaction tests
- SQL discovery integration

## Running Tests

### Basic Commands

```bash
# Run all tests
cargo test

# Run specific test by name
cargo test test_name

# Run tests for specific module
cargo test file_node::
cargo test sql_parser::

# Run with output (useful for debugging)
cargo test -- --nocapture

# Run with specific number of threads
cargo test -- --test-threads=1

# Run ignored tests
cargo test -- --ignored

# Run all tests including ignored
cargo test -- --include-ignored
```

### Test Coverage

```bash
# Install tarpaulin (if not installed)
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html

# With specific features
cargo tarpaulin --features sql_discovery
```

## Writing Tests

### Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_feature_name() {
        // Arrange
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.sql");

        // Act
        let result = function_under_test(&file_path);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_value);
    }
}
```

### Using Temporary Files

```rust
use tempfile::{TempDir, NamedTempFile};
use std::io::Write;

#[test]
fn test_with_temp_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sql");

    let content = r#"
-- name: test_file
-- requires: dependency1, dependency2
SELECT * FROM table;
"#;

    std::fs::write(&file_path, content).unwrap();

    // Test with the temporary file
    let node = FileNode::from_file(&file_path, "--").unwrap();
    assert_eq!(node.name, "test_file");
}
```

### Testing Error Cases

```rust
#[test]
fn test_cycle_detection() {
    let mut graph = TCGraph::new(vec!["normal".to_string()]);

    // Create a cycle
    graph.add_node("a", "normal").unwrap();
    graph.add_node("b", "normal").unwrap();
    graph.add_edge("a", "b").unwrap();
    graph.add_edge("b", "a").unwrap();

    let result = graph.validate();
    assert!(result.is_err());

    match result.unwrap_err() {
        TopCatError::CycleDetected { cycle, .. } => {
            assert!(cycle.contains(&"a".to_string()));
            assert!(cycle.contains(&"b".to_string()));
        }
        _ => panic!("Expected CycleDetected error"),
    }
}
```

## Debugging Techniques

### Verbose Mode

Use `-v` flag for detailed debug output:

```bash
# Run with verbose output
cargo run -- -i tests/input/sql -o output.sql -v

# Redirect to file for analysis
cargo run -- -i tests/input/sql -o output.sql -v 2> debug.log
```

### DOT Graph Visualization

Verbose mode outputs DOT format graphs:

```bash
# Generate and view graph
cargo run -- -i input/ -o output.sql -v 2>&1 | grep "digraph" -A 1000 > graph.dot
dot -Tpng graph.dot -o graph.png
open graph.png  # macOS
# or
xdg-open graph.png  # Linux
```

### Debug Logging

Set environment variables for detailed logs:

```bash
# Enable all debug logs
RUST_LOG=debug cargo run -- -i input/ -o output.sql

# Enable specific module logs
RUST_LOG=topcat::file_dag=debug cargo run -- -i input/ -o output.sql

# Trace level (most verbose)
RUST_LOG=trace cargo test test_name
```

### Using LLDB/GDB

```bash
# Build with debug symbols
cargo build

# Run with lldb (macOS)
lldb target/debug/topcat
(lldb) r -i tests/input/sql -o output.sql
(lldb) b file_dag.rs:123  # Set breakpoint
(lldb) c  # Continue

# Run with gdb (Linux)
gdb target/debug/topcat
(gdb) run -i tests/input/sql -o output.sql
(gdb) break file_dag.rs:123
(gdb) continue
```

## Test Data

### Input Files Location

Test input files are in `tests/input/`:

```
tests/
└── input/
    ├── sql/           # SQL test files
    ├── cycles/        # Files with circular dependencies
    ├── layers/        # Layer system tests
    └── discovery/     # SQL discovery tests
```

### Creating Test Cases

1. Create input files in appropriate subdirectory
2. Include necessary metadata headers
3. Write test to process the files
4. Verify expected output

Example test file:
```sql
-- tests/input/sql/table_a.sql
-- name: table_a
-- requires: schema
CREATE TABLE table_a (
    id INTEGER PRIMARY KEY
);
```

## Common Test Scenarios

### Dependency Resolution

```rust
#[test]
fn test_dependency_chain() {
    // Create files: A -> B -> C
    let files = vec![
        ("a.sql", "-- name: a\n-- requires: b"),
        ("b.sql", "-- name: b\n-- requires: c"),
        ("c.sql", "-- name: c"),
    ];

    // Build graph and sort
    let sorted = process_files(files);

    // Verify order: C, B, A
    assert_eq!(sorted, vec!["c", "b", "a"]);
}
```

### Layer Constraints

```rust
#[test]
fn test_layer_ordering() {
    let files = vec![
        ("a.sql", "-- name: a\n-- layer: append"),
        ("b.sql", "-- name: b\n-- layer: prepend"),
        ("c.sql", "-- name: c\n-- layer: normal"),
    ];

    let sorted = process_files(files);

    // Verify layer order: prepend, normal, append
    assert_eq!(sorted[0], "b");  // prepend
    assert_eq!(sorted[1], "c");  // normal
    assert_eq!(sorted[2], "a");  // append
}
```

### SQL Discovery

```rust
#[test]
fn test_sql_discovery() {
    let content = r#"
-- name: view_users
SELECT * FROM schema.users
JOIN schema.roles ON users.role_id = roles.id
"#;

    let analyzer = SqlAnalyzer::new("schema_\\w+");
    let deps = analyzer.analyze(content);

    assert!(deps.contains("schema.users"));
    assert!(deps.contains("schema.roles"));
}
```

## Performance Testing

### Benchmarking

```rust
#[bench]
fn bench_large_graph(b: &mut Bencher) {
    let files = generate_test_files(1000);  // 1000 files

    b.iter(|| {
        let graph = build_graph(&files);
        let _ = topological_sort(&graph);
    });
}
```

### Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin topcat -- -i large_input/ -o output.sql

# View flamegraph
open flamegraph.svg
```

## Troubleshooting Tests

### Flaky Tests

- Use `--test-threads=1` to eliminate concurrency issues
- Check for file system race conditions
- Ensure proper cleanup in tests

### Test Isolation

```rust
#[test]
fn test_with_isolation() {
    // Each test gets its own temp directory
    let temp_dir = TempDir::new().unwrap();

    // Test operations...

    // Automatic cleanup when temp_dir drops
}
```

### Debugging Failed Tests

```bash
# Run single test with output
cargo test test_name -- --nocapture --test-threads=1

# Use debug prints
#[test]
fn debug_test() {
    dbg!(&variable);  // Debug print
    eprintln!("Value: {:?}", value);  // Error stream print
}
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings
      - run: cargo fmt -- --check
```

## Best Practices

1. **Test Names**: Use descriptive names that explain what's being tested
2. **Assertions**: Include meaningful assertion messages
3. **Setup/Teardown**: Use `tempfile` for automatic cleanup
4. **Test Data**: Keep test files small and focused
5. **Error Cases**: Test both success and failure paths
6. **Documentation**: Comment complex test logic
7. **Performance**: Keep unit tests fast (< 1 second)
8. **Integration Tests**: Test real-world scenarios