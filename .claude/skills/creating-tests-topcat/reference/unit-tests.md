# Unit Tests

## Overview

Unit tests in Topcat are inline tests placed at the bottom of each module using `#[cfg(test)]`. They test individual functions and methods in isolation.

## Structure

```rust
// Module code
pub fn parse_dependencies(line: &str) -> Vec<String> {
    // implementation
}

// Tests at bottom of file
#[cfg(test)]
mod tests {
    use super::*;  // Import parent module items

    #[test]
    fn test_parse_dependencies_single() {
        let result = parse_dependencies("requires: table_a");
        assert_eq!(result, vec!["table_a"]);
    }

    #[test]
    fn test_parse_dependencies_multiple() {
        let result = parse_dependencies("requires: table_a table_b table_c");
        assert_eq!(result, vec!["table_a", "table_b", "table_c"]);
    }

    #[test]
    fn test_parse_dependencies_empty() {
        let result = parse_dependencies("requires:");
        assert!(result.is_empty());
    }
}
```

## Examples from Topcat

### Testing String Processing (from `file_node.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_dependencies_single() {
        let line = "table_a";
        let deps = split_dependencies(line);
        assert_eq!(deps, vec!["table_a"]);
    }

    #[test]
    fn test_split_dependencies_multiple_spaces() {
        let line = "table_a  table_b   table_c";
        let deps = split_dependencies(line);
        assert_eq!(deps, vec!["table_a", "table_b", "table_c"]);
    }

    #[test]
    fn test_split_dependencies_commas() {
        let line = "table_a,table_b,table_c";
        let deps = split_dependencies(line);
        assert_eq!(deps, vec!["table_a", "table_b", "table_c"]);
    }

    #[test]
    fn test_split_dependencies_mixed_separators() {
        let line = "table_a, table_b  table_c,  table_d";
        let deps = split_dependencies(line);
        assert_eq!(deps, vec!["table_a", "table_b", "table_c", "table_d"]);
    }

    #[test]
    fn test_split_dependencies_empty_string() {
        let line = "";
        let deps = split_dependencies(line);
        assert!(deps.is_empty());
    }

    #[test]
    fn test_split_dependencies_whitespace_only() {
        let line = "   ";
        let deps = split_dependencies(line);
        assert!(deps.is_empty());
    }
}
```

### Testing Enum Parsing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_source_from_str() {
        assert_eq!(NameSource::from_str("header"), Ok(NameSource::Header));
        assert_eq!(NameSource::from_str("discovered"), Ok(NameSource::Discovered));
    }

    #[test]
    fn test_name_source_from_str_case_insensitive() {
        assert_eq!(NameSource::from_str("HEADER"), Ok(NameSource::Header));
        assert_eq!(NameSource::from_str("Header"), Ok(NameSource::Header));
    }

    #[test]
    fn test_name_source_from_str_invalid() {
        let result = NameSource::from_str("invalid");
        assert!(result.is_err());
    }
}
```

### Testing Struct Methods

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_file_node_new() {
        let node = FileNode::new(
            "test_table".to_string(),
            PathBuf::from("test.sql"),
        );

        assert_eq!(node.name, "test_table");
        assert_eq!(node.path, PathBuf::from("test.sql"));
        assert!(node.deps.is_empty());
        assert_eq!(node.name_source, NameSource::Header);
    }

    #[test]
    fn test_file_node_add_dependency() {
        let mut node = FileNode::new(
            "test".to_string(),
            PathBuf::from("test.sql"),
        );

        node.add_dependency("dep1".to_string());
        node.add_dependency("dep2".to_string());

        assert_eq!(node.deps.len(), 2);
        assert!(node.deps.contains("dep1"));
        assert!(node.deps.contains("dep2"));
    }

    #[test]
    fn test_file_node_equality() {
        let node1 = FileNode::new("a".to_string(), PathBuf::from("path1.sql"));
        let node2 = FileNode::new("a".to_string(), PathBuf::from("path2.sql"));
        let node3 = FileNode::new("b".to_string(), PathBuf::from("path1.sql"));

        assert_eq!(node1, node2);  // Same name, different path
        assert_ne!(node1, node3);  // Different name
    }

    #[test]
    fn test_file_node_ordering() {
        let mut nodes = vec![
            FileNode::new("c".to_string(), PathBuf::from("c.sql")),
            FileNode::new("a".to_string(), PathBuf::from("a.sql")),
            FileNode::new("b".to_string(), PathBuf::from("b.sql")),
        ];

        nodes.sort();

        assert_eq!(nodes[0].name, "a");
        assert_eq!(nodes[1].name, "b");
        assert_eq!(nodes[2].name, "c");
    }
}
```

### Testing Error Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_layer_valid() {
        let result = validate_layer("normal", &["prepend", "normal", "append"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_layer_invalid() {
        let result = validate_layer("invalid", &["prepend", "normal", "append"]);
        assert!(result.is_err());

        match result {
            Err(FileNodeError::InvalidLayer(_, layer)) => {
                assert_eq!(layer, "invalid");
            }
            _ => panic!("Expected InvalidLayer error"),
        }
    }

    #[test]
    fn test_parse_header_no_name() {
        let content = "-- requires: something\nCREATE TABLE test();";
        let result = parse_file_header(content, "--");

        assert!(result.is_err());
        assert!(matches!(result, Err(FileNodeError::NoNameDefined(_))));
    }

    #[test]
    fn test_parse_header_too_many_names() {
        let content = "-- name: first\n-- name: second\nCREATE TABLE test();";
        let result = parse_file_header(content, "--");

        assert!(result.is_err());
        match result {
            Err(FileNodeError::TooManyNames(_, names)) => {
                assert_eq!(names.len(), 2);
                assert!(names.contains(&"first".to_string()));
                assert!(names.contains(&"second".to_string()));
            }
            _ => panic!("Expected TooManyNames error"),
        }
    }
}
```

### Testing Option/Result Handling

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_schema_with_dot() {
        let name = "myschema.table_name";
        let schema = extract_schema(name);
        assert_eq!(schema, Some("myschema"));
    }

    #[test]
    fn test_extract_schema_no_dot() {
        let name = "table_name";
        let schema = extract_schema(name);
        assert_eq!(schema, None);
    }

    #[test]
    fn test_extract_schema_multiple_dots() {
        let name = "schema.table.column";
        let schema = extract_schema(name);
        assert_eq!(schema, Some("schema"));  // First component only
    }

    #[test]
    fn test_get_extension_present() {
        let path = PathBuf::from("file.sql");
        let ext = get_extension(&path);
        assert_eq!(ext, Some("sql"));
    }

    #[test]
    fn test_get_extension_missing() {
        let path = PathBuf::from("file");
        let ext = get_extension(&path);
        assert_eq!(ext, None);
    }
}
```

## Assertion Patterns

### Equality Assertions

```rust
assert_eq!(actual, expected);
assert_eq!(node.name, "table_a");
assert_eq!(deps.len(), 3);
```

### Boolean Assertions

```rust
assert!(condition);
assert!(result.is_ok());
assert!(result.is_err());
assert!(deps.is_empty());
assert!(map.contains_key(&key));
```

### Inequality Assertions

```rust
assert_ne!(actual, unexpected);
assert_ne!(node1, node2);
```

### Pattern Matching Assertions

```rust
assert!(matches!(result, Ok(value) if value > 0));
assert!(matches!(error, TopCatError::MissingDependency(_, _)));
```

## Testing Private Functions

Private functions can be tested in the same module:

```rust
// Private function
fn internal_helper(input: &str) -> String {
    input.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_helper() {
        // Can access private function
        let result = internal_helper("TEST");
        assert_eq!(result, "test");
    }
}
```

## Mock Objects for Testing

```rust
#[cfg(test)]
struct MockFileSystem {
    files: HashMap<PathBuf, String>,
}

#[cfg(test)]
impl MockFileSystem {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    fn add_file(&mut self, path: &str, content: &str) {
        self.files.insert(PathBuf::from(path), content.to_string());
    }
}

#[cfg(test)]
impl FileSystem for MockFileSystem {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "File not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_mock_filesystem() {
        let mut fs = MockFileSystem::new();
        fs.add_file("test.sql", "-- name: test\nCREATE TABLE test();");

        let content = fs.read_to_string(Path::new("test.sql")).unwrap();
        assert!(content.contains("CREATE TABLE"));
    }
}
```

## Best Practices

### DO

✓ Test edge cases (empty, null, boundary values)
✓ Test error conditions explicitly
✓ Use descriptive test names: `test_function_when_condition_then_result`
✓ Keep tests focused on one behavior
✓ Use `unwrap()` in test setup (tests should panic on setup failure)
✓ Test public API thoroughly
✓ Add tests for bug fixes (regression tests)

### DON'T

✗ Test implementation details (test behavior, not internals)
✗ Write tests that depend on each other (keep independent)
✗ Use random values that make tests non-deterministic
✗ Skip error case testing
✗ Write overly complex tests (if test is complex, simplify function)

## Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Group related tests with comments
    // === Basic parsing tests ===

    #[test]
    fn test_parse_simple_case() { }

    #[test]
    fn test_parse_empty_input() { }

    // === Error handling tests ===

    #[test]
    fn test_parse_invalid_input() { }

    #[test]
    fn test_parse_missing_required_field() { }

    // === Edge cases ===

    #[test]
    fn test_parse_very_long_input() { }
}
```

## Running Unit Tests

```bash
# Run all tests in a module
cargo test file_node

# Run specific test
cargo test test_file_node_equality

# Run tests with output
cargo test -- --nocapture

# Run tests in parallel (default)
cargo test

# Run tests sequentially
cargo test -- --test-threads=1
```
