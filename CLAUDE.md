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

## SQL Dependency Discovery

**NEW**: Topcat now supports automatic dependency discovery from SQL content, eliminating the need for manual header maintenance.

### Module Structure

- **sql_config.rs** - Configuration for SQL discovery (patterns, mappings, merge strategies)
- **sql_parser.rs** - `SqlAnalyzer` that extracts dependencies from SQL content using regex patterns
- **header_generator.rs** - Generates/updates file headers with discovered dependencies

### Key Features

#### Automatic Dependency Extraction

The SQL analyzer can discover dependencies from:
- **DDL Statements**: Extracts object names from CREATE/ALTER/DROP statements
- **DML References**: Finds schema.object references in SELECT, JOIN, INSERT, UPDATE, DELETE
- **Function Calls**: Detects function/procedure dependencies
- **Type References**: Handles custom types and casts (e.g., `::TSTZRANGE`)
- **Model Generation**: Special patterns for code generation procedures

#### Configuration

Configure SQL discovery via:
1. **CLI arguments** for simple cases
2. **TOML config file** for complex patterns

Example TOML config (`topcat.toml`):
```toml
[sql_discovery]
enabled = true
schema_pattern = "(?:test|e|c|d[pio]|codegen|md)_\\w+"
merge_strategy = "discovery-only"

[[sql_discovery.type_mappings]]
from = "TSTZRANGE"
to = "c_tmf.t_time_period"

[[sql_discovery.extension_mappings]]
object = "digest"
extension = "pgcrypto"
```

#### Merge Strategies

Control how discovered dependencies combine with manual ones:

- **discovery-only** (default): Replace manual dependencies with discovered ones
- **header-only**: Ignore discovered dependencies, use only manual headers
- **union**: Combine both manual and discovered dependencies
- **header-with-fallback**: Use manual if present, otherwise use discovered
- **validate**: Check for mismatches between manual and discovered

#### Override Mechanism

Use `!` prefix to force a dependency to be kept even if not discovered:

```sql
-- name: my_table
-- requires: !special_dep, other_dep
```

The `!special_dep` will always be included even if not found in the SQL content.

#### Header Generation

Two modes for updating file headers:

1. **In-place update** (`--update-headers`): Modifies files directly
2. **Generate to directory** (`--generate-headers DIR`): Creates updated copies

### CLI Arguments

```bash
# Enable SQL discovery
--enable-sql-discovery

# Provide config file
--sql-config topcat.toml

# Override schema pattern
--schema-pattern "(?:schema1|schema2)_\\w+"

# Set merge strategy
--merge-strategy discovery-only

# Update headers in-place
--update-headers

# Generate updated headers to directory
--generate-headers /path/to/output
```

### Example Usage

```bash
# Basic usage with discovery
cargo run -- -i sql/ -o output.sql --enable-sql-discovery --schema-pattern "myschema_\\w+"

# With config file
cargo run -- -i sql/ -o output.sql --sql-config topcat.toml

# Update headers in-place
cargo run -- -i sql/ -o output.sql --enable-sql-discovery --update-headers

# Generate updated headers to new directory
cargo run -- -i sql/ -o output.sql --enable-sql-discovery --generate-headers /tmp/updated_sql
```

### Implementation Details

#### Discovery Workflow

1. `file_dag.rs:build_graph()` creates `SqlAnalyzer` if discovery enabled
2. For each file, `perform_sql_discovery()` reads content and calls `analyzer.analyze()`
3. Results stored in `FileNode.discovered_deps`
4. `FileNode.merge_dependencies()` merges based on strategy
5. Graph validation proceeds as normal

#### Pattern Matching

The analyzer uses configurable regex patterns:
- Schema pattern matches schema names (e.g., `c_\w+`, `test_\w+`)
- Dependency pattern matches `schema.object` references
- Model generation patterns for procedure calls
- Type cast patterns for `::TYPE` syntax

#### Transformations

Apply transformations to normalize object names:
- Strip suffixes (e.g., `_or_ref`)
- Map to extensions (e.g., `digest` → `pgcrypto`)
- Custom type mappings (e.g., `TSTZRANGE` → `c_tmf.t_time_period`)

### Testing

SQL discovery features have comprehensive test coverage:
- `sql_config.rs`: Configuration parsing and merging
- `sql_parser.rs`: Pattern matching, DDL extraction, dependency discovery
- `header_generator.rs`: Header generation and file updates
- `file_node.rs`: Dependency merging with override mechanism

All tests can be run with:
```bash
cargo test sql_  # Run SQL-related tests
cargo test       # Run all tests
```

### Migration Path

For existing projects:

1. **Start with validation mode** to compare manual vs discovered dependencies:
   ```bash
   topcat --sql-config config.toml --merge-strategy validate
   ```

2. **Review warnings** about mismatches

3. **Switch to discovery-only** once confident:
   ```bash
   topcat --sql-config config.toml --merge-strategy discovery-only
   ```

4. **Update headers** to remove manual dependencies:
   ```bash
   topcat --sql-config config.toml --update-headers
   ```

### Configuration Best Practices

1. **Start simple**: Use CLI args for basic patterns
2. **Graduate to TOML**: Move to config file as patterns become complex
3. **Use override prefix**: Mark critical dependencies with `!` if needed
4. **Test incremental**: Validate on subset before full codebase
5. **Version control**: Commit config file with project
