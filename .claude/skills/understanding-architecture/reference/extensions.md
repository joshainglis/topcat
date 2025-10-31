# Extension Guide

## Adding New Metadata Fields

### Step 1: Update FileNode Structure

```rust
// In file_node.rs
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub requires: Vec<String>,
    // Add new field
    pub priority: Option<u32>,  // New metadata field
    // ...
}
```

### Step 2: Add Parsing Logic

```rust
// In file_node.rs::from_file()
if line.starts_with("priority:") {
    node.priority = Some(
        line["priority:".len()..]
            .trim()
            .parse()
            .unwrap_or(0)
    );
}
```

### Step 3: Update Merge Logic

```rust
// In merge_dependencies() if needed
if let Some(priority) = discovered_priority {
    self.priority = Some(priority);
}
```

## Custom Sorting Strategies

### Modify Ord Implementation

```rust
impl Ord for FileNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Add priority-based sorting
        self.priority.cmp(&other.priority)
            .then_with(|| self.name.cmp(&other.name))
            .then_with(|| self.path.cmp(&other.path))
    }
}
```

### Add Weight Calculation

```rust
impl FileNode {
    pub fn weight(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        self.path.hash(&mut hasher);
        // Include priority in weight
        self.priority.hash(&mut hasher);
        hasher.finish()
    }
}
```

## New Filter Types

### Step 1: Add CLI Argument

```rust
// In config.rs
#[structopt(long = "include-author")]
pub include_author: Option<String>,
```

### Step 2: Implement Filter Logic

```rust
// In io_utils.rs
pub fn filter_by_author(
    files: Vec<PathBuf>,
    author: &str
) -> Vec<PathBuf> {
    files.into_iter()
        .filter(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            content.contains(&format!("-- author: {}", author))
        })
        .collect()
}
```

### Step 3: Integrate with Discovery

```rust
// In main.rs or file_dag.rs
if let Some(author) = &config.include_author {
    files = filter_by_author(files, author);
}
```

## Custom Validators

### Add Validation Function

```rust
// In file_dag.rs
pub fn validate_custom_rule(&self) -> Result<()> {
    for (name, node) in &self.nodes {
        // Custom validation logic
        if node.requires.len() > 10 {
            return Err(TopCatError::TooManyDependencies {
                file: name.clone(),
                count: node.requires.len(),
            });
        }
    }
    Ok(())
}
```

### Hook into Validation Pipeline

```rust
// In validate() method
pub fn validate(&self) -> Result<()> {
    self.validate_cycles()?;
    self.validate_cross_layer_deps()?;
    self.validate_custom_rule()?;  // Add custom validation
    Ok(())
}
```

## Output Format Extensions

### Custom Separators

```rust
// In output.rs
pub fn write_with_custom_format(
    files: Vec<FileNode>,
    format: OutputFormat
) -> Result<()> {
    match format {
        OutputFormat::Sql => write_sql_format(files),
        OutputFormat::Json => write_json_format(files),
        OutputFormat::Custom(fmt) => write_custom(files, fmt),
    }
}
```

## Testing Extensions

Always add tests for new functionality:

```rust
#[test]
fn test_priority_sorting() {
    let mut nodes = vec![
        FileNode { name: "a", priority: Some(2), ... },
        FileNode { name: "b", priority: Some(1), ... },
    ];

    nodes.sort();

    assert_eq!(nodes[0].name, "b");  // Lower priority first
    assert_eq!(nodes[1].name, "a");
}
```