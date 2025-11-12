# Trait Patterns

## Standard Trait Implementation

### Implementation Order Convention

Implement traits in this order for consistency:
1. PartialEq
2. Eq
3. PartialOrd
4. Ord
5. Hash
6. Display

### Complete Example: FileNode

```rust
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    // ... other fields
}

impl PartialEq for FileNode {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for FileNode {}

impl PartialOrd for FileNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl Hash for FileNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Display for FileNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
```

**Key points**:
- Equality/ordering based on `name` field (identity)
- Hash uses same field as equality (consistency requirement)
- Display shows user-friendly representation

## Custom Traits

### FileSystem Trait (Testability)

Defined in `src/fs.rs` for testable file operations:

```rust
pub trait FileSystem {
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn is_file(&self, path: &Path) -> bool;
    fn exists(&self, path: &Path) -> bool;
}
```

**Implementation for production**:
```rust
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        std::fs::read_dir(path)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect()
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}
```

**Mock for testing**:
```rust
#[cfg(test)]
struct MockFileSystem {
    files: HashMap<PathBuf, String>,
}

#[cfg(test)]
impl FileSystem for MockFileSystem {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "File not found"))
    }
    // ... other methods
}
```

### OutputDestination Trait

For flexible output handling:

```rust
pub trait OutputDestination {
    fn write(&mut self, content: &str) -> Result<(), TopCatError>;
}

impl OutputDestination for File {
    fn write(&mut self, content: &str) -> Result<(), TopCatError> {
        self.write_all(content.as_bytes())
            .map_err(TopCatError::Io)
    }
}

impl OutputDestination for String {
    fn write(&mut self, content: &str) -> Result<(), TopCatError> {
        self.push_str(content);
        Ok(())
    }
}
```

### GraphAnalyzer Trait

For analysis plugins (in `src/analysis/mod.rs`):

```rust
pub trait GraphAnalyzer {
    fn analyze(&self) -> Result<AnalysisResult, Box<dyn std::error::Error>>;
}

pub struct OrphanAnalyzer<'a> {
    graph: &'a TCGraph,
}

impl<'a> GraphAnalyzer for OrphanAnalyzer<'a> {
    fn analyze(&self) -> Result<AnalysisResult, Box<dyn std::error::Error>> {
        let orphans = self.find_orphans()?;
        Ok(AnalysisResult::Orphans(orphans))
    }
}
```

## Derive Macros

### Common Derive Combinations

```rust
// Basic structs
#[derive(Debug, Clone)]
pub struct Config { }

// Small enums (can copy)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Prepend,
    Normal,
    Append,
}

// Configuration types (serialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlDiscoveryConfig {
    pub enabled: bool,
    pub schema_pattern: Option<String>,
}

// CLI arguments
#[derive(Debug, Args)]
pub struct ConcatArgs {
    #[arg(short, long)]
    input_dirs: Vec<PathBuf>,
}
```

### When NOT to Derive

Don't derive when custom logic needed:

```rust
// FileNode: Custom PartialEq based on name only
// (can't derive because would compare all fields)
impl PartialEq for FileNode {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name  // Only compare name
    }
}

// Custom Display for user-friendly output
impl Display for TopCatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Custom formatting logic
    }
}
```

## Trait Bounds

### Function Trait Bounds

```rust
pub fn process_nodes<T>(nodes: Vec<T>) -> Vec<String>
where
    T: Display + Ord,
{
    let mut sorted = nodes;
    sorted.sort();
    sorted.iter().map(|n| n.to_string()).collect()
}
```

### Impl Trait

```rust
pub fn get_analyzers() -> impl Iterator<Item = Box<dyn GraphAnalyzer>> {
    vec![
        Box::new(OrphanAnalyzer::new(&graph)),
        Box::new(DeadBranchAnalyzer::new(&graph)),
    ]
    .into_iter()
}
```

### Trait Objects

```rust
pub fn run_analysis(analyzer: &dyn GraphAnalyzer) -> Result<(), Box<dyn std::error::Error>> {
    let result = analyzer.analyze()?;
    println!("{:?}", result);
    Ok(())
}
```

## Iterator Trait

Implementing Iterator for custom types:

```rust
pub struct NodeIterator<'a> {
    nodes: &'a [FileNode],
    index: usize,
}

impl<'a> Iterator for NodeIterator<'a> {
    type Item = &'a FileNode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.nodes.len() {
            let node = &self.nodes[self.index];
            self.index += 1;
            Some(node)
        } else {
            None
        }
    }
}
```

## Lifetime Parameters

### Borrowing in Traits

```rust
pub trait NodeVisitor<'a> {
    fn visit(&mut self, node: &'a FileNode);
}

pub struct NameCollector<'a> {
    names: Vec<&'a str>,
}

impl<'a> NodeVisitor<'a> for NameCollector<'a> {
    fn visit(&mut self, node: &'a FileNode) {
        self.names.push(&node.name);
    }
}
```

## Best Practices

### DO

✓ Implement standard traits in consistent order
✓ Use traits for abstraction (FileSystem, OutputDestination)
✓ Keep trait methods focused and composable
✓ Document trait contracts clearly
✓ Use derive macros when default implementations suffice
✓ Implement Hash + Eq consistently (same fields)

### DON'T

✗ Mix identity fields in PartialEq (use single identifier)
✗ Derive PartialEq when custom logic needed
✗ Create trait hierarchies without clear need
✗ Use trait objects when static dispatch sufficient
✗ Forget to implement Error trait for error types

## Testing Traits

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_node_equality() {
        let node1 = FileNode::new("a".to_string(), PathBuf::from("path1"));
        let node2 = FileNode::new("a".to_string(), PathBuf::from("path2"));

        assert_eq!(node1, node2);  // Equal by name despite different paths
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

    #[test]
    fn test_mock_filesystem() {
        let mut mock = MockFileSystem::new();
        mock.add_file("test.sql", "-- name: test");

        let content = mock.read_to_string(Path::new("test.sql")).unwrap();
        assert_eq!(content, "-- name: test");
    }
}
```
