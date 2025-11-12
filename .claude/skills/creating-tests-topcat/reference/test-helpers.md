# Test Helpers

## Standard Helper Functions

### create_test_file

The core helper for creating test files in TempDir.

```rust
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
```

**Features**:
- Creates nested directories automatically
- Adds trailing newline (matches real files)
- Uses unwrap (tests should panic on I/O failure)

**Usage**:
```rust
let dir = TempDir::new().unwrap();

// Simple file
create_test_file(&dir, "test.sql", "-- name: test\nCREATE TABLE test();");

// Nested directory
create_test_file(&dir, "schema/migrations/001.sql", "-- name: migration_001");

// Multiple files
create_test_file(&dir, "a.sql", "-- name: a\n-- requires: b");
create_test_file(&dir, "b.sql", "-- name: b");
```

### build_test_graph

Standard graph builder for tests.

```rust
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
```

**Usage**:
```rust
let dir = TempDir::new().unwrap();
create_test_file(&dir, "a.sql", "-- name: a");

let graph = build_test_graph(&dir);
assert_eq!(graph.node_count(), 1);
```

### Variations

#### Custom Layers

```rust
fn build_graph_with_layers(dir: &TempDir, layers: &[&str]) -> TCGraph {
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

// Usage
let graph = build_graph_with_layers(&dir, &["ddl", "dml", "indexes"]);
```

#### Custom Extensions

```rust
fn build_graph_with_extensions(dir: &TempDir, exts: &[&str]) -> TCGraph {
    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        include_file_extensions: Some(exts),
        comment_str: "--",
        layers: &["normal"],
        ..Default::default()
    };

    let mut graph = TCGraph::new(&config);
    graph.build_graph().unwrap();
    graph
}

// Usage
let graph = build_graph_with_extensions(&dir, &["sql", "ddl", "dml"]);
```

#### With SQL Discovery

```rust
fn build_graph_with_sql_discovery(dir: &TempDir) -> TCGraph {
    let sql_config = SqlDiscoveryConfig {
        enabled: true,
        schema_pattern: Some("(?:app|test)_\\w+".to_string()),
        merge_strategy: Some(MergeStrategy::DiscoveryOnly),
        type_mappings: vec![],
        extension_mappings: vec![],
    };

    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        include_file_extensions: Some(&["sql"]),
        comment_str: "--",
        layers: &["normal"],
        sql_config: Some(&sql_config),
        enable_sql_discovery: true,
        ..Default::default()
    };

    let mut graph = TCGraph::new(&config);
    graph.build_graph().unwrap();
    graph
}
```

## Advanced Helpers

### create_test_files (Batch Creation)

```rust
fn create_test_files(dir: &TempDir, files: &[(&str, &str)]) {
    for (path, content) in files {
        create_test_file(dir, path, content);
    }
}

// Usage
let files = [
    ("a.sql", "-- name: a\n-- requires: b c"),
    ("b.sql", "-- name: b"),
    ("c.sql", "-- name: c"),
];
create_test_files(&dir, &files);
```

### assert_order

Verify topological order of nodes.

```rust
fn assert_order(sorted: &[FileNode], before: &str, after: &str) {
    let names: Vec<String> = sorted.iter().map(|n| n.name.clone()).collect();

    let pos_before = names.iter().position(|n| n == before)
        .expect(&format!("Node '{}' not found in sorted list", before));

    let pos_after = names.iter().position(|n| n == after)
        .expect(&format!("Node '{}' not found in sorted list", after));

    assert!(
        pos_before < pos_after,
        "'{}' should come before '{}', but positions are {} and {}",
        before, after, pos_before, pos_after
    );
}

// Usage
let sorted = graph.topological_sort().unwrap();
assert_order(&sorted, "b", "a");  // b must come before a
assert_order(&sorted, "c", "b");  // c must come before b
```

### assert_contains_nodes

Verify graph contains expected nodes.

```rust
fn assert_contains_nodes(graph: &TCGraph, expected: &[&str]) {
    for name in expected {
        assert!(
            graph.has_node(name),
            "Expected graph to contain node '{}'",
            name
        );
    }
}

// Usage
assert_contains_nodes(&graph, &["a", "b", "c"]);
```

### get_node_names

Extract node names for easier assertions.

```rust
fn get_node_names(nodes: &[FileNode]) -> Vec<String> {
    nodes.iter().map(|n| n.name.clone()).collect()
}

// Usage
let sorted = graph.topological_sort().unwrap();
let names = get_node_names(&sorted);

assert_eq!(names.len(), 3);
assert!(names.contains(&"a".to_string()));
```

### assert_no_cycles

Verify graph has no cycles.

```rust
fn assert_no_cycles(graph: &TCGraph) {
    let result = graph.topological_sort();
    assert!(
        result.is_ok(),
        "Graph should have no cycles, but got error: {:?}",
        result.err()
    );
}

// Usage
assert_no_cycles(&graph);
```

## File Content Helpers

### create_sql_file

Create SQL file with standard header.

```rust
fn create_sql_file(
    dir: &TempDir,
    path: &str,
    name: &str,
    requires: Option<&str>,
    layer: Option<&str>,
    sql: &str,
) {
    let mut header = format!("-- name: {name}");

    if let Some(deps) = requires {
        header.push_str(&format!("\n-- requires: {deps}"));
    }

    if let Some(l) = layer {
        header.push_str(&format!("\n-- layer: {l}"));
    }

    let content = format!("{header}\n{sql}");
    create_test_file(dir, path, &content);
}

// Usage
create_sql_file(
    &dir,
    "users.sql",
    "users_table",
    Some("schema"),
    Some("normal"),
    "CREATE TABLE users (id INT, name TEXT);"
);
```

### create_dependency_chain

Create a chain of dependent files.

```rust
fn create_dependency_chain(dir: &TempDir, names: &[&str]) {
    for (i, name) in names.iter().enumerate() {
        let content = if i == 0 {
            format!("-- name: {name}\nCREATE TABLE {name}();")
        } else {
            let prev = names[i - 1];
            format!("-- name: {name}\n-- requires: {prev}\nCREATE TABLE {name}();")
        };

        create_test_file(dir, &format!("{name}.sql"), &content);
    }
}

// Usage: Creates a -> b -> c -> d
create_dependency_chain(&dir, &["d", "c", "b", "a"]);
```

## Assertion Helpers

### assert_deps

Verify a node has expected dependencies.

```rust
fn assert_deps(graph: &TCGraph, node_name: &str, expected_deps: &[&str]) {
    let node = graph.get_node_by_name(node_name)
        .expect(&format!("Node '{}' not found", node_name));

    let mut actual_deps: Vec<_> = node.deps.iter().cloned().collect();
    actual_deps.sort();

    let mut expected: Vec<String> = expected_deps.iter().map(|s| s.to_string()).collect();
    expected.sort();

    assert_eq!(
        actual_deps, expected,
        "Node '{}' has unexpected dependencies",
        node_name
    );
}

// Usage
assert_deps(&graph, "a", &["b", "c"]);
```

### assert_orphans

Verify orphan detection results.

```rust
fn assert_orphans(graph: &TCGraph, expected_orphans: &[&str]) {
    let analyzer = OrphanAnalyzer::new(&graph);
    let orphans = analyzer.find_orphans().unwrap();

    let mut orphan_names: Vec<_> = orphans.iter().cloned().collect();
    orphan_names.sort();

    let mut expected: Vec<String> = expected_orphans.iter().map(|s| s.to_string()).collect();
    expected.sort();

    assert_eq!(orphan_names, expected, "Unexpected orphan detection results");
}

// Usage
assert_orphans(&graph, &["orphan1", "orphan2"]);
```

## Mock Helpers

### MockFileSystem

For testing without TempDir.

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

    fn remove_file(&mut self, path: &str) {
        self.files.remove(&PathBuf::from(path));
    }

    fn has_file(&self, path: &str) -> bool {
        self.files.contains_key(&PathBuf::from(path))
    }
}

#[cfg(test)]
impl FileSystem for MockFileSystem {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.files.get(path).cloned().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "File not found")
        })
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self.files.keys()
            .filter(|p| p.starts_with(path))
            .cloned()
            .collect())
    }

    fn is_file(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }
}

// Usage
let mut fs = MockFileSystem::new();
fs.add_file("test.sql", "-- name: test\nCREATE TABLE test();");
assert!(fs.has_file("test.sql"));
```

## Setup and Teardown Patterns

### Test Fixture

For repeated test setup.

```rust
struct TestFixture {
    dir: TempDir,
    graph: TCGraph,
}

impl TestFixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        create_test_file(&dir, "a.sql", "-- name: a\n-- requires: b");
        create_test_file(&dir, "b.sql", "-- name: b");

        let graph = build_test_graph(&dir);

        Self { dir, graph }
    }

    fn add_file(&self, path: &str, content: &str) {
        create_test_file(&self.dir, path, content);
    }

    fn rebuild(&mut self) {
        self.graph = build_test_graph(&self.dir);
    }
}

// Usage
#[test]
fn test_with_fixture() {
    let mut fixture = TestFixture::new();
    assert_eq!(fixture.graph.node_count(), 2);

    fixture.add_file("c.sql", "-- name: c");
    fixture.rebuild();
    assert_eq!(fixture.graph.node_count(), 3);
}
```

## Best Practices

### DO

✓ Reuse standard helpers (create_test_file, build_test_graph)
✓ Create custom helpers for repeated patterns
✓ Use descriptive helper names
✓ Add comments explaining complex helpers
✓ Use unwrap in test helpers (let tests panic)
✓ Keep helpers in test modules

### DON'T

✗ Duplicate helper code across test files
✗ Create overly generic helpers (keep focused)
✗ Mix production and test code
✗ Make helpers too complex
✗ Return Result from test helpers (use unwrap)
