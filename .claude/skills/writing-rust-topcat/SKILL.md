---
name: writing-rust-topcat
description: Guides writing Rust code in Topcat following project conventions for modules, error handling, documentation, and idioms. Use when implementing new features, adding modules, or refactoring existing code.
---

# Writing Rust in Topcat

## Quick Reference

### Module Visibility
```rust
// Public API (exported via lib.rs)
pub mod analysis;
pub mod file_dag;

// Private implementation
mod io_utils;
mod stable_topo;

// Public functions in modules
pub fn build_graph(&mut self) -> Result<(), TopCatError> { }

// Private helpers
fn get_file_headers(path: &PathBuf, comment_str: &str) -> Vec<String> { }
```

### Error Handling Pattern
```rust
// Return Result with TopCatError
pub fn validate(&self) -> Result<(), TopCatError> {
    if self.name.is_empty() {
        return Err(TopCatError::InvalidFileHeader(
            self.path.clone(),
            "Name cannot be empty".to_string(),
        ));
    }
    Ok(())
}

// Use match for error conversion
match result {
    Ok(node) => nodes.push(node),
    Err(FileNodeError::NoNameDefined(p)) => {
        info!("Ignoring {p:?}: No name defined");
        continue;
    }
    Err(e) => return Err(e.into()),
}
```

### Struct Pattern
```rust
#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    deps: HashSet<String>,
}

impl FileNode {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            deps: HashSet::new(),
        }
    }
}
```

### Documentation Standard
```rust
/// Build the dependency graph from input files.
///
/// Scans input directories, parses file headers, extracts dependencies,
/// and constructs the directed acyclic graph (DAG).
///
/// # Returns
///
/// `Ok(())` on success, or `TopCatError` if:
/// - Cyclic dependencies are detected
/// - Required files are missing
/// - File headers are invalid
///
/// # Example
///
/// ```
/// let mut graph = TCGraph::new(&config);
/// graph.build_graph()?;
/// ```
pub fn build_graph(&mut self) -> Result<(), TopCatError> {
    // implementation
}
```

### Logging Levels
```rust
use log::{debug, info, trace, error};

info!("Building dependency graph from {} files", count);  // User-facing
debug!("Processing file: {:?}", path);                     // Operational
trace!("Dependency edge: {} -> {}", from, to);            // Detailed
error!("Failed to read file: {}", err);                   // Errors
```

## Common Patterns

### Trait Implementation Order
```rust
impl PartialEq for FileNode {
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
}
impl Eq for FileNode {}
impl PartialOrd for FileNode { /* ... */ }
impl Ord for FileNode { /* ... */ }
impl Hash for FileNode { /* ... */ }
impl Display for FileNode { /* ... */ }
```

### Iterator Chains
```rust
// Collect before sorting if order matters
let mut deps: Vec<_> = file_node.deps.iter().collect();
deps.sort();
for dep in deps {
    // process in sorted order
}

// Filter and map pattern
nodes
    .iter()
    .filter(|n| n.layer == "normal")
    .map(|n| n.name.clone())
    .collect()
```

### Collections Usage
- `Vec<T>` - ordered sequences
- `HashMap<K, V>` - key-value lookups
- `HashSet<T>` - unique membership
- `VecDeque<T>` - queue operations

## Checklist for New Code

```
- [ ] Add rustdoc comments for public items
- [ ] Use Result<T, TopCatError> for fallible operations
- [ ] Derive Debug, Clone for structs (add others as needed)
- [ ] Add logging at appropriate levels
- [ ] Handle errors explicitly (no unwrap in production code)
- [ ] Use meaningful variable names
- [ ] Sort collections when deterministic order required
- [ ] Add unit tests for complex logic
- [ ] Validate inputs early (fail fast)
```

## Reference Documentation

**Module Architecture**: See [reference/modules.md](reference/modules.md)
**Error Handling**: See [reference/error-handling.md](reference/error-handling.md)
**Trait Patterns**: See [reference/traits.md](reference/traits.md)
**Common Idioms**: See [reference/idioms.md](reference/idioms.md)

## Quick Tips

1. **Config structs use lifetimes** to borrow instead of clone
2. **FileSystem trait** for testable I/O code
3. **Match expressions** preferred over if-let chains
4. **String handling**: Use `trim()`, `to_string()`, `to_lowercase()` consistently
5. **Path handling**: Use `PathBuf` for owned, `&Path` for borrowed
6. **Builder pattern**: Implement `new()` constructors with sensible defaults
