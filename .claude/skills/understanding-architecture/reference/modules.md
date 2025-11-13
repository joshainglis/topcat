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

**Responsibility**: Error handling with enhanced context

Key features:
- `TopCatError` enum with variants for different error types
- `ErrorContext` trait for composable error messages:
  - `with_context()`: Add contextual information
  - `with_file_path()`: Attach file path to errors
  - `with_node_name()`: Attach node name to errors
- Convenience constructors:
  - `config_error()`: Configuration errors
  - `graph_build_error()`: Graph construction errors
  - `validation_error()`: Validation failures

## Utility Modules

### commands/common.rs

**Responsibility**: Shared command utilities and graph building

Key structures:
- `GraphBuilder`: Builder pattern for consistent graph construction across commands
- `load_sql_discovery_config()`: SQL discovery configuration
- `build_root_matcher()`: Root node protection patterns
- Platform utilities delegation

Eliminates ~400 lines of duplicated code across command files.

### schema_utils.rs

**Responsibility**: Schema filtering and matching operations

Key structures:
```rust
pub struct SchemaFilter {
    schemas: Vec<String>,
}
```

Functions:
- `to_node_prefixes()`: Convert schemas to node name prefixes
- `matches_node()`: Check if node belongs to schema
- `filter_nodes()`: Filter node collections by schema

### display_utils.rs

**Responsibility**: Table creation and output formatting

Functions:
- `create_standard_table()`: Standard table with borders
- `create_node_table()`: Tables for node displays
- `format_path_relative()`: Relative path formatting
- `format_file_list()`: Format file lists for output
- `print_separator()`: Section separators

### graph_utils.rs

**Responsibility**: Graph analysis and node mapping utilities

Functions:
- `build_name_to_node_map()`: Map node names to FileNode references
- `build_name_to_path_map()`: Map node names to file paths
- `build_path_to_node_map()`: Map file paths to nodes
- `count_nodes_by_layer()`: Layer statistics
- `filter_nodes_by_layer()`: Filter nodes by layer
- `group_nodes_by_schema()`: Group nodes by schema

### platform.rs

**Responsibility**: Platform-specific utilities

Functions:
- `null_device()`: Platform-specific null device path (`/dev/null` or `NUL`)
- `temp_dir()`: Temporary directory location
- `temp_file()`: Create temporary files with prefix

## Modularized Commands

### commands/analyze/

Broken down from 1,329 lines into 10 focused modules:

- `mod.rs` (437 lines): CLI routing, argument parsing, command dispatch
- `common.rs` (181 lines): `AnalysisLogger`, `AnalysisDisplayConfig`, generic display utilities
- `cycles.rs` (153 lines): Circular dependency detection
- `dead_branches.rs` (121 lines): Dead subtree analysis
- `file.rs` (148 lines): Single file detailed analysis
- `missing.rs` (144 lines): Missing dependency detection
- `leaf_nodes.rs` (61 lines): Leaf node identification
- `orphans.rs` (53 lines): Orphan file detection
- `root_nodes.rs` (47 lines): Root node identification
- `unrequired.rs` (61 lines): Unrequired file detection

Generic `analyze_and_display()` function eliminates 70% code duplication.

### commands/clean/

Broken down from 772 lines into 6 focused modules:

- `mod.rs` (394 lines): CLI routing, argument parsing, command dispatch
- `common.rs` (188 lines): `DeletionContext`, confirmation logic, deletion utilities
- `dead_branches.rs` (69 lines): Dead subtree cleanup
- `orphans.rs` (72 lines): Orphan file cleanup
- `targets.rs` (112 lines): Target-specific cleanup with dependency validation
- `unrequired.rs` (72 lines): Unrequired file cleanup

Shared deletion utilities prevent scattered logic.

### commands/export/

Modularized export functionality:

- `mod.rs`: CLI routing and dispatch
- Format-specific modules for JSON, DOT, GraphML, Mermaid
- Export mode handling (full, deps, dependents, direct)