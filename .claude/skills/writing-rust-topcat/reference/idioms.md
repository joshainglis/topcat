# Common Rust Idioms in Topcat

## String Processing

### Splitting and Filtering

```rust
fn split_dependencies(line: &str) -> Vec<String> {
    line.split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|x| {
            let x = x.trim().to_string();
            if !x.is_empty() { Some(x) } else { None }
        })
        .collect()
}
```

**Pattern**: Split → trim → filter empty → collect

### String Ownership

```rust
// Borrow for reading
fn process_name(name: &str) -> bool {
    name.starts_with("schema_")
}

// Own for modification
fn append_suffix(mut name: String, suffix: &str) -> String {
    name.push_str(suffix);
    name
}

// Convert when needed
let owned: String = borrowed_str.to_string();
let borrowed: &str = &owned_string;
```

### Case Conversion

```rust
let ext = path
    .extension()
    .map(|e| e.to_string_lossy().to_lowercase())
    .unwrap_or_default();
```

## Path Handling

### Path vs PathBuf

```rust
// PathBuf for owned paths
pub struct FileNode {
    pub path: PathBuf,  // Owns the path
}

// &Path for borrowed paths
fn read_file(path: &Path) -> io::Result<String> {
    std::fs::read_to_string(path)
}
```

### Extension Checking

```rust
fn has_sql_extension(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => ext.to_string_lossy().to_lowercase() == "sql",
        None => false,
    }
}
```

### Path Construction

```rust
// Join paths
let file_path = dir.join("subdir").join("file.sql");

// Get components
if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
}

let file_name = path
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("unknown");
```

## Iterator Patterns

### Collect and Sort

```rust
// Always collect before sorting
let mut deps: Vec<_> = node.deps.iter().collect();
deps.sort();

for dep in deps {
    println!("{}", dep);
}
```

**Why**: HashSet/HashMap have undefined iteration order. Sorting ensures deterministic output.

### Filter-Map Chain

```rust
let valid_nodes: Vec<_> = all_nodes
    .iter()
    .filter(|n| n.layer == "normal")
    .filter(|n| !n.deps.is_empty())
    .map(|n| n.name.clone())
    .collect();
```

### Find First Match

```rust
let node = nodes
    .iter()
    .find(|n| n.name == target_name)
    .ok_or_else(|| TopCatError::MissingDependency(
        "graph".to_string(),
        target_name.to_string(),
    ))?;
```

### Partition

```rust
let (valid, invalid): (Vec<_>, Vec<_>) = results
    .into_iter()
    .partition(Result::is_ok);
```

## Option Handling

### Match Pattern

```rust
let config = match sql_config {
    Some(cfg) => cfg,
    None => return Ok(Vec::new()),
};
```

### Unwrap with Default

```rust
let layer = node.layer.as_deref().unwrap_or("normal");
let count = cache.get(&key).copied().unwrap_or(0);
```

### Option Chaining

```rust
let name = path
    .file_name()
    .and_then(|n| n.to_str())
    .map(|s| s.to_lowercase())
    .unwrap_or_else(|| "default".to_string());
```

### Filter Option

```rust
let nodes: Vec<_> = items
    .iter()
    .filter_map(|item| {
        match parse_node(item) {
            Ok(node) => Some(node),
            Err(_) => None,
        }
    })
    .collect();
```

## Result Handling

### Early Return with ?

```rust
pub fn process(&self) -> Result<(), TopCatError> {
    let config = self.load_config()?;
    let nodes = self.parse_files(&config)?;
    self.validate_nodes(&nodes)?;
    Ok(())
}
```

### Map Error

```rust
std::fs::write(&path, content)
    .map_err(|e| TopCatError::Io(e))?;

// Or with From implementation:
std::fs::write(&path, content)?;  // Auto-converts via From<io::Error>
```

### Collecting Results

```rust
// Collect Vec<Result<T, E>> into Result<Vec<T>, E>
let nodes: Result<Vec<_>, TopCatError> = paths
    .iter()
    .map(|p| parse_file(p))
    .collect();

let nodes = nodes?;  // Propagate first error
```

### Match for Control Flow

```rust
match result {
    Ok(node) => {
        self.add_node(node);
        Ok(())
    }
    Err(FileNodeError::NoNameDefined(_)) => {
        info!("Skipping file without name");
        Ok(())  // Graceful handling
    }
    Err(e) => Err(e.into()),  // Propagate other errors
}
```

## Collection Usage

### HashMap Patterns

```rust
use std::collections::HashMap;

// Insert and get
let mut map = HashMap::new();
map.insert(key, value);

// Get with default
let count = *map.get(&key).unwrap_or(&0);

// Entry API for upsert
map.entry(key)
    .and_modify(|v| *v += 1)
    .or_insert(1);
```

### HashSet Patterns

```rust
use std::collections::HashSet;

// Check membership
if deps.contains(&dependency) {
    // ...
}

// Union
let all_deps: HashSet<_> = set1.union(&set2).cloned().collect();

// Difference
let missing: HashSet<_> = required.difference(&available).cloned().collect();
```

### VecDeque for Queue

```rust
use std::collections::VecDeque;

let mut queue = VecDeque::new();
queue.push_back(start_node);

while let Some(node) = queue.pop_front() {
    for neighbor in node.neighbors() {
        queue.push_back(neighbor);
    }
}
```

## Borrowing and Lifetimes

### Config Pattern with Lifetimes

```rust
pub struct Config<'a> {
    pub input_dirs: Vec<PathBuf>,
    pub include_globs: Option<&'a [String]>,  // Borrow from caller
    pub include_extensions: Option<&'a [String]>,
}

// Usage
let extensions = vec!["sql".to_string(), "ddl".to_string()];
let config = Config {
    input_dirs: vec![PathBuf::from("sql/")],
    include_extensions: Some(&extensions),  // Borrow
    include_globs: None,
};
```

**Why**: Avoid cloning large vectors when Config is short-lived.

### Return Borrowed Data

```rust
impl FileNode {
    pub fn name(&self) -> &str {
        &self.name  // Return borrowed, avoid clone
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
```

## Clone Patterns

### Clone When Needed

```rust
// Clone for ownership transfer
let name = node.name.clone();
self.names.push(name);

// Clone for multiple ownership
let node1 = node.clone();
let node2 = node.clone();
```

### Avoid Unnecessary Clones

```rust
// Bad: Cloning in loop
for name in &names {
    process(name.clone());  // Unnecessary if process takes &str
}

// Good: Borrow in loop
for name in &names {
    process(name);  // Just borrow
}
```

### Clone for Collections

```rust
// Clone when collecting
let names: Vec<String> = nodes
    .iter()
    .map(|n| n.name.clone())
    .collect();

// Avoid clone with into_iter
let names: Vec<String> = nodes
    .into_iter()
    .map(|n| n.name)
    .collect();
```

## Logging Patterns

### Structured Logging

```rust
use log::{debug, info, trace, error};

info!("Processing {} files from {:?}", file_count, input_dir);
debug!("Node {} has {} dependencies", node.name, node.deps.len());
trace!("Adding edge: {} -> {}", from, to);
error!("Failed to parse file {:?}: {}", path, err);
```

### Log Before Operations

```rust
pub fn build_graph(&mut self) -> Result<(), TopCatError> {
    info!("Building dependency graph...");

    let files = self.discover_files()?;
    debug!("Discovered {} files", files.len());

    for file in files {
        trace!("Processing {:?}", file);
        self.process_file(&file)?;
    }

    info!("Graph built successfully with {} nodes", self.node_count());
    Ok(())
}
```

## Performance Patterns

### Pre-allocate Collections

```rust
let mut nodes = Vec::with_capacity(100);  // Avoid reallocation
let mut map = HashMap::with_capacity(50);
```

### Use References in Loops

```rust
// Good: Iterate by reference
for node in &nodes {
    process(node);  // Borrow, no move
}

// Bad: Unnecessary moves
for node in nodes.clone() {
    process(node);
}
```

### Parallel Processing (Rayon)

```rust
use rayon::prelude::*;

let results: Vec<_> = paths
    .par_iter()  // Parallel iterator
    .map(|p| parse_file(p))
    .collect();
```

## Testing Patterns

### Assert Patterns

```rust
assert_eq!(actual, expected);
assert_ne!(actual, unexpected);
assert!(condition);
assert!(result.is_ok());
assert!(result.is_err());
```

### Match Assertions

```rust
match result {
    Ok(node) => assert_eq!(node.name, "expected"),
    Err(e) => panic!("Expected Ok, got Err: {:?}", e),
}

// Or with matches! macro
assert!(matches!(result, Ok(node) if node.name == "expected"));
```

### Error Matching

```rust
let result = validate_cycle(&graph);
assert!(result.is_err());

match result {
    Err(TopCatError::CyclicDependency(cycle)) => {
        assert!(cycle.contains(&"node_a".to_string()));
    }
    _ => panic!("Expected CyclicDependency error"),
}
```

## Documentation Patterns

### Function Documentation

```rust
/// Parse file header and extract metadata.
///
/// Reads comment lines from the file header and extracts name,
/// dependencies, and layer information.
///
/// # Arguments
///
/// * `path` - Path to the file to parse
/// * `comment_str` - Comment prefix (e.g., "--" for SQL)
///
/// # Returns
///
/// `Ok(FileNode)` if successful, or `FileNodeError` if:
/// - No name is defined
/// - Multiple names are declared
/// - Invalid layer is specified
///
/// # Example
///
/// ```
/// let node = parse_file_header(Path::new("schema.sql"), "--")?;
/// assert_eq!(node.name, "create_schema");
/// ```
pub fn parse_file_header(
    path: &Path,
    comment_str: &str,
) -> Result<FileNode, FileNodeError> {
    // ...
}
```

### Module Documentation

```rust
//! File node representation and parsing.
//!
//! This module defines `FileNode`, which represents a file with its
//! dependency metadata extracted from header comments.
//!
//! # Example
//!
//! ```
//! let node = FileNode::new("table_a".to_string(), PathBuf::from("a.sql"));
//! node.add_dependency("table_b".to_string());
//! ```
```
