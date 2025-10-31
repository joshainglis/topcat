# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`topcat` is a Rust CLI tool for **topological concatenation of files**. It reads files with dependency metadata in header comments, builds a directed acyclic graph (DAG), performs topological sorting respecting layer constraints, and outputs a single concatenated file. The primary use case is SQL files where execution order matters.

## Build and Development Commands

### Basic Commands

```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Linting
cargo clippy

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check
```

### Development Environment

This project uses Nix flakes for reproducible development:

```bash
# Enter development shell with all dependencies
nix develop
```

The Nix environment includes:
- Rust 1.88.0 stable with clippy, rustfmt, rust-src
- rust-analyzer
- Rust-Rover IDE support

### Testing the CLI

```bash
# Basic test
cargo run -- -i tests/input/sql -o /tmp/output.sql

# Dry run (print to stdout)
cargo run -- -i tests/input/sql -o /tmp/output.sql --dry

# Verbose mode (includes DOT graph output)
cargo run -- -i tests/input/sql -o /tmp/output.sql -v
```

## Architecture Overview

### Module Structure

- **main.rs** - CLI argument parsing (structopt), workflow orchestration, error handling
- **file_node.rs** - `FileNode` struct representing individual files with dependency metadata (name, requires, dropped_by, exists, layer)
- **file_dag.rs** - `TCGraph` manages the dependency graph, organized by layers (HashMap<String, DiGraph>), validates dependencies, detects cycles
- **stable_topo.rs** - Custom DFS-based topological sort that produces deterministic output by respecting node weights (FileNode implements Ord)
- **config.rs** - Configuration struct derived from CLI arguments
- **output.rs** - Handles file writing and stdout output with separators/suffixes
- **io_utils.rs** - File system traversal, glob matching, hidden file handling
- **exceptions.rs** - Error types (TopCatError, FileNodeError)

### Key Architectural Concepts

#### Layer System

Files can be assigned to layers that enforce strict ordering constraints:

```sql
-- layer: prepend
-- layer: normal
-- layer: append
```

- Default layers: `prepend` → `normal` → `append`
- Custom layers via `--layers first,second,third`
- Files in lower-index layers **cannot** depend on files in higher-index layers
- `--fallback-layer` specifies where files without explicit layer go (default: "normal")
- Each layer has its own independent DAG in `TCGraph`

Implementation: `file_dag.rs:TCGraph` maintains `HashMap<String, DiGraph<NodeIndex, ()>>` where keys are layer names.

#### Dependency Types

- **Hard dependencies** (`requires`, `dropped_by`): Enforce topological ordering
- **Soft dependencies** (`exists`): Ensure file inclusion without ordering constraints
- **Layer constraints**: Implicit ordering between all files in different layers

#### Stable Topological Sort

`stable_topo.rs` implements a custom DFS-based topological sort:

1. Nodes are sorted by weight before DFS traversal (FileNode implements Ord)
2. Ensures deterministic output even with equivalent topological orderings
3. Respects both explicit dependencies and layer constraints

### Metadata Parsing

Files must include header comments (format: `{comment_prefix} key: value1, value2`):

```sql
-- name: unique_identifier
-- requires: dependency1, dependency2
-- dropped_by: cleanup_task
-- layer: prepend
-- exists: soft_dependency
```

Backward compatibility:
- `-- is_initial` → maps to "prepend" layer
- `-- is_final` → maps to "append" layer

Parsing implementation: `file_node.rs:FileNode::from_file()`

## Testing

Test files are organized by module:

- `file_node.rs` - Header parsing, layer handling, dependency extraction
- `io_utils.rs` - Directory walking, glob matching
- `stable_topo.rs` - Topological sort correctness and stability
- `output.rs` - File suffix handling

When adding new features:

1. Add unit tests in the relevant module
2. Add integration test input files to `tests/input/`
3. Use `tempfile` crate for temporary file testing

## Important Implementation Notes

### Cycle Detection

The graph is validated for cycles using `graph-cycles` crate. Errors include the full cycle path for debugging.

### Name Uniqueness

All file nodes must have unique names across all layers. Duplicates cause validation errors.

### Cross-Layer Dependencies

If a file in layer N depends on a file in layer M where M > N, validation fails. Layers enforce a strict partial ordering.

### Node Filtering

Multiple filtering mechanisms:
- Extension-based: `--include-exts`, `--exclude-exts`
- Glob patterns: `--include-glob`, `--exclude-glob`
- Name prefixes: `--include-prefix`, `--exclude-prefix`
- Subdirectory filtering with dependency pulling: `--subdir-filter`

When using `--subdir-filter`, dependencies outside the subdirectory are automatically included to maintain graph integrity.

## Debugging

Use `-v` flag to output:
1. Debug logs via `env_logger`
2. DOT format graph visualization (can be rendered with Graphviz)

Example:
```bash
cargo run -- -i input/ -o output.sql -v > debug.log 2>&1
```
