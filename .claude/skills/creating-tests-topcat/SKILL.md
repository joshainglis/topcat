---
name: creating-tests-topcat
description: Guides test creation in Topcat following existing patterns for unit tests, integration tests, and test utilities. Use when writing new tests, debugging test failures, or improving test coverage.
---

# Creating Tests in Topcat

## Quick Start

### Test Organization

```
tests/
├── analysis_tests.rs      # Dependency analysis integration tests
├── clean_tests.rs          # File cleanup integration tests
└── schema_tests.rs         # Schema analysis integration tests

src/
├── file_node.rs
│   └── #[cfg(test)] mod tests { }   # Inline unit tests
└── stable_topo.rs
    └── #[cfg(test)] mod tests { }   # Inline unit tests
```

## Integration Test Template

```rust
use std::fs;
use tempfile::TempDir;
use topcat::config::Config;
use topcat::file_dag::TCGraph;

fn create_test_file(dir: &TempDir, path: &str, content: &str) {
    let file_path = dir.path().join(path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let content_with_newline = format!("{content}\n");
    fs::write(&file_path, content_with_newline).unwrap();
}

fn build_test_graph(dir: &TempDir) -> TCGraph {
    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        output_file: None,
        include_file_extensions: Some(&["sql"]),
        comment_str: "--",
        layers: &["normal"],
        ..Default::default()
    };

    let mut graph = TCGraph::new(&config);
    graph.build_graph().unwrap();
    graph
}

#[test]
fn test_basic_dependency() {
    let dir = TempDir::new().unwrap();

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

    let graph = build_test_graph(&dir);

    assert_eq!(graph.node_count(), 2);

    let order = graph.topological_sort().unwrap();
    assert_eq!(order[0].name, "b");
    assert_eq!(order[1].name, "a");
}
```

## Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = "test input";

        // Act
        let result = function_to_test(input);

        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    fn test_error_case() {
        let result = function_that_fails();

        assert!(result.is_err());
        match result {
            Err(ExpectedError::Variant(msg)) => {
                assert_eq!(msg, "expected message");
            }
            _ => panic!("Wrong error type"),
        }
    }
}
```

## Common Test Patterns

### Pattern 1: TempDir File Setup

```rust
#[test]
fn test_multi_file_setup() {
    let dir = TempDir::new().unwrap();

    // Create directory structure
    create_test_file(&dir, "schema/a.sql", "-- name: schema.a");
    create_test_file(&dir, "schema/b.sql", "-- name: schema.b\n-- requires: schema.a");
    create_test_file(&dir, "data/c.sql", "-- name: data.c");

    let graph = build_test_graph(&dir);
    assert_eq!(graph.node_count(), 3);
}
```

### Pattern 2: Error Validation

```rust
#[test]
fn test_missing_dependency_error() {
    let dir = TempDir::new().unwrap();
    create_test_file(&dir, "a.sql", "-- name: a\n-- requires: missing");

    let config = Config { /* ... */ };
    let mut graph = TCGraph::new(&config);

    let result = graph.build_graph();
    assert!(result.is_err());

    match result {
        Err(TopCatError::MissingDependency(from, to)) => {
            assert_eq!(from, "a");
            assert_eq!(to, "missing");
        }
        _ => panic!("Expected MissingDependency error"),
    }
}
```

### Pattern 3: Graph Validation

```rust
#[test]
fn test_topological_order() {
    let dir = TempDir::new().unwrap();

    create_test_file(&dir, "a.sql", "-- name: a\n-- requires: b c");
    create_test_file(&dir, "b.sql", "-- name: b\n-- requires: c");
    create_test_file(&dir, "c.sql", "-- name: c");

    let graph = build_test_graph(&dir);
    let order = graph.topological_sort().unwrap();

    let names: Vec<_> = order.iter().map(|n| &n.name).collect();

    // c must come before b, b must come before a
    let pos_c = names.iter().position(|&n| n == "c").unwrap();
    let pos_b = names.iter().position(|&n| n == "b").unwrap();
    let pos_a = names.iter().position(|&n| n == "a").unwrap();

    assert!(pos_c < pos_b);
    assert!(pos_b < pos_a);
}
```

### Pattern 4: Collection Assertions

```rust
#[test]
fn test_orphan_detection() {
    let dir = TempDir::new().unwrap();

    create_test_file(&dir, "orphan.sql", "-- name: orphan");
    create_test_file(&dir, "a.sql", "-- name: a\n-- requires: b");
    create_test_file(&dir, "b.sql", "-- name: b");

    let graph = build_test_graph(&dir);
    let analyzer = OrphanAnalyzer::new(&graph);
    let orphans = analyzer.find_orphans().unwrap();

    assert_eq!(orphans.len(), 1);
    assert!(orphans.contains(&"orphan".to_string()));
}
```

## Testing Checklist

```
Test Development:
- [ ] Create TempDir for isolated file tests
- [ ] Use create_test_file helper for file setup
- [ ] Test happy path first
- [ ] Add error case tests
- [ ] Validate error messages/types
- [ ] Test edge cases (empty, cycles, duplicates)
- [ ] Use descriptive test names (test_what_when_then)
- [ ] Add comments for complex test scenarios

Before Committing:
- [ ] Run: cargo test
- [ ] Run: cargo test --test integration_tests
- [ ] All tests pass
- [ ] No warnings in test output
```

## Running Tests

```bash
# All tests
cargo test

# Specific test file
cargo test --test analysis_tests

# Specific test function
cargo test test_find_orphans

# With output
cargo test test_name -- --nocapture

# With logging
RUST_LOG=topcat::file_dag=debug cargo test test_name -- --nocapture
```

## Reference Documentation

**Unit Tests**: See [reference/unit-tests.md](reference/unit-tests.md)
**Integration Tests**: See [reference/integration-tests.md](reference/integration-tests.md)
**Test Helpers**: See [reference/test-helpers.md](reference/test-helpers.md)
**Test Templates**: See [scripts/test-template.rs](scripts/test-template.rs)

## Best Practices

1. **Use TempDir** for all file-based tests (automatic cleanup)
2. **Test one thing** per test function
3. **Name descriptively**: `test_finds_orphan_when_no_dependencies`
4. **Arrange-Act-Assert** structure
5. **Match errors explicitly** instead of `assert!(is_err())`
6. **Test both success and failure** paths
7. **Use unwrap() in tests** (it's ok, tests should panic on setup failure)
8. **Add newline to file content** (create_test_file pattern)
