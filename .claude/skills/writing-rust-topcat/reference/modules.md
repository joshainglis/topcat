# Module Architecture

## Public Library Modules (exported in `src/lib.rs`)

### Core Modules
```rust
pub mod analysis         // Dependency analysis (orphans, dead branches, root nodes)
pub mod config           // Configuration management
pub mod exceptions       // Error types (TopCatError, FileNodeError)
pub mod file_dag         // DAG graph logic (TCGraph)
pub mod file_node        // File representation with metadata
pub mod fs               // FileSystem trait for testable I/O
pub mod header_generator // Generate file headers from metadata
pub mod output           // Output formatting and writing
pub mod sql_config       // SQL discovery configuration
```

### Command Modules
```rust
pub mod commands {
    pub mod concat       // Main concatenation command
    pub mod analyze      // Dependency analysis command
    pub mod clean        // File cleanup/deletion command
    pub mod schema       // Schema analysis command
    pub mod export       // Graph export (JSON, DOT, GraphML, Mermaid)
}
```

## Private Implementation Modules

```rust
mod io_utils            // File I/O utilities
mod sql_parser          // SQL dependency extraction
mod stable_topo         // Deterministic topological sort
```

## Module Organization Guidelines

### When to Make a Module Public

Make modules public when:
- They provide core domain types (FileNode, TCGraph)
- They define configuration structures (Config, SqlDiscoveryConfig)
- They implement user-facing commands (commands/*)
- They define error types (exceptions)
- They provide extensibility points (FileSystem trait)

### When to Keep a Module Private

Keep modules private when:
- They're implementation details (stable_topo sorting algorithm)
- They're utility functions (io_utils)
- They're parser internals (sql_parser)
- They're unlikely to be used by library consumers

### File Organization Pattern

**Single-file modules**: Most modules in Topcat are single files
```
src/
├── file_node.rs          # Complete module in one file
├── file_dag.rs           # Large module (~1000 lines, still one file)
└── config.rs             # Configuration module
```

**Multi-file modules**: Only for commands and analysis
```
src/
├── commands/
│   ├── mod.rs            # Re-exports subcommands
│   ├── concat.rs
│   ├── analyze.rs
│   └── export.rs
└── analysis/
    ├── mod.rs            # Trait definition + re-exports
    ├── orphans.rs
    ├── dead_branches.rs
    └── root_nodes.rs
```

## Module Size Guidelines

- **Small**: < 200 lines (config.rs, exceptions.rs)
- **Medium**: 200-500 lines (file_node.rs, output.rs)
- **Large**: 500-1000+ lines (file_dag.rs - core graph logic)

**When to split**: Consider splitting when:
- A module exceeds 1500 lines
- Multiple distinct concerns exist
- Submodules would improve clarity

## Dependency Guidelines

### Allowed Dependencies
- **Core modules** can depend on exceptions, config
- **Command modules** can depend on core modules
- **Analysis modules** can depend on file_dag, file_node
- **Private modules** can depend on anything (they're internal)

### Circular Dependencies
Avoid circular dependencies by:
- Extracting shared types to a common module
- Using trait objects for abstraction
- Keeping dependency graph acyclic

Example:
```rust
// Good: file_dag depends on file_node
use crate::file_node::FileNode;

// Bad: file_node depending on file_dag (circular)
// Instead: extract shared traits or types
```

## Adding a New Module

Checklist:
1. Create file in `src/` (or subdirectory for multi-file)
2. Add `pub mod module_name;` in appropriate parent (lib.rs or mod.rs)
3. Add module documentation at top of file
4. Define public API carefully (minimize public surface)
5. Add unit tests in `#[cfg(test)]` module
6. Document with rustdoc comments

Example:
```rust
//! Graph export functionality.
//!
//! Provides multiple export formats for the dependency graph:
//! - JSON (structured data)
//! - DOT (Graphviz visualization)
//! - GraphML (graph exchange format)
//! - Mermaid (documentation diagrams)

use crate::file_dag::TCGraph;
use crate::exceptions::TopCatError;

pub struct GraphExporter {
    // ...
}

#[cfg(test)]
mod tests {
    use super::*;
    // ...
}
```
