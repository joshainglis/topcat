# Error Handling

## Error Types

### TopCatError (Main Error Type)

Defined in `src/exceptions.rs`:

```rust
#[derive(Debug)]
pub enum TopCatError {
    Io(io::Error),
    NameClash(PathBuf, PathBuf, String),
    MissingDependency(String, String),
    InvalidDependency(String, String, String),
    CyclicDependency(Vec<String>),
    ConfigError(String),
    InvalidFileHeader(PathBuf, String),
}
```

**When to use each variant**:

- **Io**: File system operations (read, write, create directories)
- **NameClash**: Two files declare same name
- **MissingDependency**: Required dependency not found
- **InvalidDependency**: Dependency format error
- **CyclicDependency**: Circular dependencies detected
- **ConfigError**: Configuration validation failures
- **InvalidFileHeader**: Header parsing errors

### FileNodeError (File-specific Errors)

```rust
#[derive(Debug)]
pub enum FileNodeError {
    NoNameDefined(PathBuf),
    TooManyNames(PathBuf, Vec<String>),
    InvalidLayer(PathBuf, String),
}
```

Used during file parsing, then converted to TopCatError or handled gracefully.

## Error Handling Patterns

### Pattern 1: Direct Return

```rust
pub fn validate_name(&self) -> Result<(), TopCatError> {
    if self.name.is_empty() {
        return Err(TopCatError::InvalidFileHeader(
            self.path.clone(),
            "Name cannot be empty".to_string(),
        ));
    }
    Ok(())
}
```

### Pattern 2: Match and Convert

```rust
fn handle_file_node_error(e: FileNodeError) -> Result<(), TopCatError> {
    match e {
        FileNodeError::NoNameDefined(p) => {
            info!("Ignoring {p:?}: No name defined in file header");
            Ok(())  // Gracefully skip
        }
        FileNodeError::TooManyNames(p, s) => Err(TopCatError::InvalidFileHeader(
            p,
            format!("Too many names declared: {}", s.join(", ")),
        )),
        FileNodeError::InvalidLayer(p, layer) => Err(TopCatError::InvalidFileHeader(
            p,
            format!("Invalid layer '{layer}' declared"),
        )),
    }
}
```

### Pattern 3: Context with map_err

```rust
fs::read_to_string(&path)
    .map_err(|e| TopCatError::Io(e))?;
```

### Pattern 4: Early Exit with ?

```rust
pub fn process_file(&self, path: &Path) -> Result<FileNode, TopCatError> {
    let content = fs::read_to_string(path)?;  // Propagate IO errors
    let node = FileNode::parse(&content)?;     // Propagate parsing errors
    self.validate_node(&node)?;                // Propagate validation errors
    Ok(node)
}
```

### Pattern 5: Collecting Results

```rust
let nodes: Result<Vec<_>, TopCatError> = paths
    .iter()
    .map(|p| self.parse_file(p))
    .collect();

let nodes = nodes?;  // Fail on first error
```

## Error Implementation

### Implementing Error Trait

```rust
impl std::fmt::Display for TopCatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopCatError::Io(e) => write!(f, "IO error: {}", e),
            TopCatError::NameClash(p1, p2, name) => write!(
                f,
                "Name clash: '{}' defined in both {:?} and {:?}",
                name, p1, p2
            ),
            TopCatError::CyclicDependency(cycle) => write!(
                f,
                "Cyclic dependency detected: {}",
                cycle.join(" -> ")
            ),
            // ... other variants
        }
    }
}

impl std::error::Error for TopCatError {}
```

### From Conversions

```rust
impl From<io::Error> for TopCatError {
    fn from(err: io::Error) -> Self {
        TopCatError::Io(err)
    }
}

impl From<FileNodeError> for TopCatError {
    fn from(err: FileNodeError) -> Self {
        match err {
            FileNodeError::TooManyNames(p, s) => TopCatError::InvalidFileHeader(
                p,
                format!("Too many names: {}", s.join(", ")),
            ),
            // ... other conversions
        }
    }
}
```

## Error Handling in Commands

Commands use `Result<(), Box<dyn std::error::Error>>` for flexibility:

```rust
pub fn run(args: &AnalyzeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args(args)?;
    let mut graph = TCGraph::new(&config);

    graph.build_graph().map_err(|e| {
        error!("Failed to build graph: {}", e);
        e
    })?;

    let analyzer = OrphanAnalyzer::new(&graph);
    let orphans = analyzer.find_orphans()?;

    println!("Found {} orphaned files", orphans.len());
    Ok(())
}
```

## Best Practices

### DO

✓ Use `Result<T, TopCatError>` for library functions
✓ Convert errors at module boundaries
✓ Log errors before propagating: `error!("Failed: {}", e)`
✓ Provide context in error messages
✓ Use `?` operator for propagation
✓ Match on errors when graceful handling needed

### DON'T

✗ Use `unwrap()` or `expect()` in production code (ok in tests)
✗ Swallow errors silently
✗ Create generic string errors - use proper variants
✗ Return bare `String` as error type
✗ Panic on recoverable errors

## Testing Error Cases

```rust
#[test]
fn test_missing_dependency_error() {
    let node = FileNode::new("a".to_string(), PathBuf::from("a.sql"));
    node.add_dependency("missing".to_string());

    let mut graph = TCGraph::new(&config);
    graph.add_node(node);

    let result = graph.validate();
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

## Error Messages

### Good Error Messages

Include context:
```rust
Err(TopCatError::InvalidFileHeader(
    path.clone(),
    format!("Invalid layer '{}'. Valid layers: {}", layer, valid_layers.join(", ")),
))
```

### Bad Error Messages

Too generic:
```rust
Err(TopCatError::ConfigError("Invalid config".to_string()))  // Not helpful
```

Better:
```rust
Err(TopCatError::ConfigError(
    format!("Invalid schema pattern '{}': {}", pattern, regex_err)
))
```

## Cycle Detection Example

Special error formatting for cycles:

```rust
pub fn detect_cycles(&self) -> Result<(), TopCatError> {
    if let Some(cycle) = find_cycle(&self.graph) {
        let cycle_str: Vec<String> = cycle
            .iter()
            .map(|n| self.get_node(n).unwrap().name.clone())
            .collect();

        return Err(TopCatError::CyclicDependency(cycle_str));
    }
    Ok(())
}
```

Error output:
```
Error: Cyclic dependency detected: table_a -> table_b -> table_c -> table_a
```
