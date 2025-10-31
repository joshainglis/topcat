# Writing Tests

## Unit Test Patterns

### Basic Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_functionality() {
        // Arrange
        let input = setup_test_data();

        // Act
        let result = function_under_test(input);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_value);
    }
}
```

### Testing with Temporary Files

```rust
use tempfile::TempDir;
use std::fs::write;

#[test]
fn test_file_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sql");

    let content = r#"
-- name: test_table
-- requires: dependency
CREATE TABLE test_table (id INT);
"#;

    write(&file_path, content).unwrap();

    let node = FileNode::from_file(&file_path, "--").unwrap();
    assert_eq!(node.name, "test_table");
    assert_eq!(node.requires, vec!["dependency"]);
}
```

### Testing Error Cases

```rust
#[test]
fn test_missing_dependency() {
    let mut graph = TCGraph::new(vec!["normal".to_string()]);

    graph.add_node("a", "normal").unwrap();
    let result = graph.add_edge("a", "missing");

    assert!(result.is_err());
    match result.unwrap_err() {
        TopCatError::MissingDependency { file, dep } => {
            assert_eq!(file, "a");
            assert_eq!(dep, "missing");
        }
        _ => panic!("Wrong error type"),
    }
}
```

## Integration Test Patterns

### End-to-End Testing

```rust
// tests/integration_test.rs
use topcat::{Config, process_files};
use tempfile::TempDir;

#[test]
fn test_full_workflow() {
    let temp_dir = TempDir::new().unwrap();
    create_test_files(&temp_dir);

    let config = Config {
        input_dir: temp_dir.path().to_path_buf(),
        output_file: None,
        dry_run: true,
        ..Default::default()
    };

    let result = process_files(config);
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.contains("expected content"));
}
```

### Testing SQL Discovery

```rust
#[test]
fn test_sql_discovery_integration() {
    let content = r#"
-- name: user_view
CREATE VIEW app.user_view AS
SELECT u.*, r.name as role_name
FROM app.users u
JOIN app.roles r ON u.role_id = r.id;
"#;

    let analyzer = SqlAnalyzer::new("app_\\w+");
    let deps = analyzer.analyze(content);

    assert_eq!(deps, HashSet::from([
        "app.users".to_string(),
        "app.roles".to_string(),
    ]));
}
```

## Test Data Management

### Creating Test Fixtures

```rust
fn create_test_scenario(dir: &Path) -> Vec<PathBuf> {
    let files = vec![
        ("schema.sql", "-- name: schema\nCREATE SCHEMA app;"),
        ("table.sql", "-- name: table\n-- requires: schema\nCREATE TABLE app.users;"),
        ("view.sql", "-- name: view\n-- requires: table\nCREATE VIEW app.user_view;"),
    ];

    files.iter().map(|(name, content)| {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }).collect()
}
```

### Parameterized Tests

```rust
use rstest::rstest;

#[rstest]
#[case("prepend", 0)]
#[case("normal", 1)]
#[case("append", 2)]
fn test_layer_ordering(#[case] layer: &str, #[case] expected_index: usize) {
    let node = FileNode {
        layer: Some(layer.to_string()),
        ..Default::default()
    };

    let index = get_layer_index(&node);
    assert_eq!(index, expected_index);
}
```

## Assertion Helpers

### Custom Assertions

```rust
fn assert_file_order(output: &str, first: &str, second: &str) {
    let first_pos = output.find(first)
        .expect(&format!("{} not found", first));
    let second_pos = output.find(second)
        .expect(&format!("{} not found", second));

    assert!(
        first_pos < second_pos,
        "{} should come before {}", first, second
    );
}
```

### Testing Graph Properties

```rust
fn assert_valid_topological_order(graph: &TCGraph, order: &[String]) {
    for (i, node) in order.iter().enumerate() {
        let deps = graph.get_dependencies(node);

        for dep in deps {
            let dep_pos = order.iter().position(|n| n == dep);
            assert!(
                dep_pos.map_or(false, |p| p < i),
                "{} depends on {} but comes before it", node, dep
            );
        }
    }
}
```