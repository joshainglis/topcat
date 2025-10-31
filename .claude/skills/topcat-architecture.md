# Topcat Architecture Deep Dive

This skill provides in-depth architectural details about Topcat's implementation.

## Module Responsibilities

### Core Modules

#### main.rs
- CLI argument parsing using `structopt`
- Workflow orchestration and command dispatch
- Top-level error handling and user feedback
- Integration of all components

#### file_node.rs
- `FileNode` struct representing individual files
- Metadata fields: name, requires, dropped_by, exists, layer
- Header comment parsing logic
- Dependency merging with discovery results
- Node weight calculation for stable sorting

#### file_dag.rs
- `TCGraph` structure manages the dependency graph
- Layer-based organization using `HashMap<String, DiGraph>`
- Dependency validation across layers
- Cycle detection using `graph-cycles` crate
- Graph building and node index management

#### stable_topo.rs
- Custom DFS-based topological sort implementation
- Deterministic output through node weight ordering
- Respects both explicit dependencies and layer constraints
- FileNode Ord trait implementation for consistent sorting

### Supporting Modules

#### config.rs
- Configuration struct derived from CLI arguments
- Default values and validation
- Integration with SQL discovery configuration

#### output.rs
- File writing with atomic operations
- Stdout output handling
- File separator and suffix management
- Dry-run mode support

#### io_utils.rs
- File system traversal algorithms
- Glob pattern matching
- Hidden file detection and filtering
- Extension-based filtering

#### exceptions.rs
- Custom error types: `TopCatError`, `FileNodeError`
- Error context and chaining
- User-friendly error messages

## Layer System Implementation

### Concept

Layers enforce strict ordering constraints between groups of files:

```
Layer 0: prepend  ─┐
                   ├─> All files in Layer 0 before Layer 1
Layer 1: normal   ─┤
                   ├─> All files in Layer 1 before Layer 2
Layer 2: append   ─┘
```

### Data Structure

```rust
pub struct TCGraph {
    layers: Vec<String>,                           // Ordered layer names
    graphs: HashMap<String, DiGraph<NodeIndex>>,   // Per-layer DAGs
    node_map: HashMap<String, (String, NodeIndex)> // name -> (layer, index)
}
```

### Constraints

1. **Intra-layer**: Files within a layer follow dependency order
2. **Inter-layer**: All files in layer N before layer N+1
3. **Cross-layer deps**: File in layer N cannot depend on layer M where M > N
4. **Validation**: Checked during graph construction

### Configuration

- Default layers: `["prepend", "normal", "append"]`
- Custom via: `--layers first,second,third`
- Fallback layer: `--fallback-layer normal`
- Backward compatibility:
  - `is_initial` → `prepend`
  - `is_final` → `append`

## Dependency Types

### Hard Dependencies

#### requires
- Enforces topological ordering
- File A requires B means B comes before A
- Validated for existence and cycles

#### dropped_by
- Reverse dependency relationship
- File A dropped_by B means A comes before B
- Useful for cleanup/rollback scripts

### Soft Dependencies

#### exists
- Ensures file inclusion without ordering
- File A exists B means B must be in output
- No ordering relationship imposed

### Layer Dependencies

- Implicit between all files in different layers
- Cannot be overridden by explicit dependencies
- Enforced during topological sort

## Stable Topological Sort Algorithm

### Problem

Multiple valid topological orderings exist for a DAG. We need deterministic output across runs.

### Solution

1. **Node Weighting**: Each FileNode has a weight based on:
   - File path (for consistency)
   - Name (primary identifier)
   - Content hash (if needed)

2. **Ord Implementation**:
   ```rust
   impl Ord for FileNode {
       fn cmp(&self, other: &Self) -> Ordering {
           self.name.cmp(&other.name)
               .then_with(|| self.path.cmp(&other.path))
       }
   }
   ```

3. **DFS Traversal**:
   - Sort nodes by weight before traversal
   - Visit neighbors in sorted order
   - Guarantees consistent output

### Algorithm Steps

1. Build adjacency list from DiGraph
2. Sort all nodes by weight
3. Initialize visited set and result vector
4. For each unvisited node (in sorted order):
   - Perform DFS
   - Add to result in post-order
5. Reverse result for topological order

## Metadata Parsing

### Header Format

```sql
-- name: unique_identifier
-- requires: dep1, dep2, dep3
-- dropped_by: cleanup_script
-- exists: soft_dep1, soft_dep2
-- layer: prepend
```

### Parsing Rules

1. **Comment Prefix**: Configurable (`--`, `#`, `//`)
2. **Key-Value**: `key: value1, value2`
3. **Comma Separation**: Whitespace trimmed
4. **Case Sensitive**: Names must match exactly
5. **Override Prefix**: `!` forces dependency retention

### Implementation

```rust
// In file_node.rs
impl FileNode {
    pub fn from_file(path: &Path, comment_prefix: &str) -> Result<Self> {
        // Read first N lines
        // Parse headers with regex
        // Extract metadata
        // Validate and return
    }
}
```

## Node Filtering System

### Filter Types

#### Extension Filters
- `--include-exts sql,ddl`
- `--exclude-exts bak,tmp`
- Applied during file discovery

#### Glob Patterns
- `--include-glob "*/migrations/*.sql"`
- `--exclude-glob "**/test_*.sql"`
- Uses `glob` crate for matching

#### Name Prefixes
- `--include-prefix "schema_"`
- `--exclude-prefix "temp_"`
- Simple string prefix matching

#### Subdirectory Filter
- `--subdir-filter specific/path`
- Includes files from subdirectory
- **Pulls dependencies** from outside if needed

### Filter Priority

1. Exclude patterns checked first
2. Include patterns checked second
3. Default behavior if no match
4. Dependency pulling overrides filters

## Graph Validation

### Cycle Detection

Uses `graph-cycles` crate:

```rust
pub fn validate(&self) -> Result<()> {
    for (layer, graph) in &self.graphs {
        if is_cyclic_directed(graph) {
            let cycle = find_cycle(graph);
            return Err(TopCatError::CycleDetected {
                cycle,
                layer
            });
        }
    }
    Ok(())
}
```

### Name Uniqueness

- Enforced across all layers
- Checked during node addition
- Error includes duplicate name and files

### Cross-Layer Validation

```rust
fn validate_cross_layer_deps(&self) -> Result<()> {
    for (name, deps) in dependencies {
        let source_layer_idx = self.get_layer_index(name);
        for dep in deps {
            let dep_layer_idx = self.get_layer_index(dep);
            if dep_layer_idx > source_layer_idx {
                return Err(TopCatError::InvalidCrossLayerDep {
                    source: name,
                    target: dep,
                    source_layer,
                    target_layer
                });
            }
        }
    }
}
```

## Performance Considerations

### Graph Construction
- O(V + E) where V = files, E = dependencies
- HashMap lookups for name resolution
- Lazy dependency resolution

### Topological Sort
- O(V + E) DFS traversal
- Additional O(V log V) for sorting
- Memory: O(V) for visited set

### File I/O
- Buffered reading for header parsing
- Parallel file discovery (when possible)
- Lazy content reading (only when needed)

## Error Handling

### Error Types

```rust
pub enum TopCatError {
    CycleDetected { cycle: Vec<String>, layer: String },
    DuplicateName { name: String, files: Vec<PathBuf> },
    MissingDependency { file: String, dep: String },
    InvalidCrossLayerDep { /* fields */ },
    IoError(io::Error),
    ParseError { file: PathBuf, line: usize, msg: String }
}
```

### Error Context

- File paths included in errors
- Line numbers for parse errors
- Full cycle path for debugging
- Suggestions for fixes

## Extension Points

### Adding New Metadata Fields

1. Update `FileNode` struct in `file_node.rs`
2. Add parsing logic in `from_file()`
3. Update merge logic if needed
4. Add validation if required

### Custom Sorting Strategies

1. Modify `Ord` implementation in `FileNode`
2. Add weight calculation method
3. Update stable_topo algorithm if needed

### New Filter Types

1. Add CLI argument in `config.rs`
2. Implement filter logic in `io_utils.rs`
3. Integrate with file discovery
4. Handle dependency pulling if needed