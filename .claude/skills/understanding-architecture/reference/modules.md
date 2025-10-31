# Module Reference

## Core Modules

### main.rs

**Responsibility**: Entry point and workflow orchestration

- Parses CLI arguments using `structopt`
- Coordinates file discovery, graph building, and output
- Handles top-level error reporting

### file_node.rs

**Responsibility**: File representation and metadata

Key structures:

```rust
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub requires: Vec<String>,
    pub dropped_by: Vec<String>,
    pub exists: Vec<String>,
    pub layer: String,
    pub discovered_deps: Option<HashSet<String>>,
}
```

Functions:

- `from_file()`: Parse file headers and create node
- `merge_dependencies()`: Combine manual and discovered deps
- `Ord` implementation: Define sort order

### file_dag.rs

**Responsibility**: DAG construction and validation

Key structures:

```rust
pub struct TCGraph {
    layers: Vec<String>,
    graphs: HashMap<String, DiGraph<NodeIndex>>,
    node_map: HashMap<String, (String, NodeIndex)>,
}
```

Functions:

- `build_graph()`: Construct DAG from files
- `validate()`: Check cycles and constraints
- `add_edge()`: Add dependency relationships

## Supporting Modules

### stable_topo.rs

**Responsibility**: Deterministic topological sorting

- Custom DFS implementation
- Node weight-based ordering
- Layer constraint enforcement

### config.rs

**Responsibility**: Configuration management

- CLI argument definitions
- Default values
- SQL discovery configuration integration

### output.rs

**Responsibility**: File generation

- Atomic file writing
- Stdout handling
- Separator and suffix management

### io_utils.rs

**Responsibility**: File system operations

- Directory traversal
- Pattern matching (glob, prefix, extension)
- Hidden file filtering

### exceptions.rs

**Responsibility**: Error handling

- Custom error types
- Error context and chaining
- User-friendly messages