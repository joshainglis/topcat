# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with Topcat.

## Project Overview

**Topcat** is a Rust CLI tool for **topological concatenation of files**. It reads files with dependency metadata in header comments, builds a directed acyclic graph (DAG), performs topological sorting respecting layer constraints, and outputs a single concatenated file.

Primary use case: Ordering SQL migration files where execution order matters based on dependencies.

## Quick Start

```bash
# Build and run
cargo build --release
cargo run -- -i input_dir/ -o output.sql

# Basic usage
topcat -i sql/ -o migrations.sql                    # Concatenate SQL files
topcat -i sql/ -o migrations.sql --dry              # Preview output
topcat -i sql/ -o migrations.sql -v                 # Verbose with DOT graph

# With SQL dependency discovery
topcat -i sql/ -o output.sql --enable-sql-discovery --schema-pattern "myschema_\\w+"
```

## Development Commands

```bash
# Development environment (Nix)
nix develop                      # Enter dev shell with all dependencies

# Build & Test
cargo build                      # Debug build
cargo build --release            # Release build
cargo test                       # Run all tests
cargo test test_name             # Run specific test
cargo clippy                     # Lint code
cargo fmt                        # Format code

# Run with test data
cargo run -- -i tests/input/sql -o /tmp/output.sql
```

## Architecture Summary

### Core Modules

| Module | Purpose |
|--------|---------|
| `main.rs` | CLI parsing, workflow orchestration |
| `file_node.rs` | File representation with metadata |
| `file_dag.rs` | DAG management and validation |
| `stable_topo.rs` | Deterministic topological sort |
| `config.rs` | Configuration management |
| `output.rs` | Output generation |
| `io_utils.rs` | File system operations |

### Key Concepts

#### File Metadata

Files include dependency metadata in header comments:

```sql
-- name: create_users_table
-- requires: create_schema
-- layer: normal
-- exists: extensions

CREATE TABLE users (...);
```

#### Layers

Layers enforce ordering between groups of files:
- Default: `prepend` → `normal` → `append`
- Custom: `--layers first,second,third`
- Files in earlier layers always precede later layers

#### Dependencies

- **Hard** (`requires`, `dropped_by`): Enforce ordering
- **Soft** (`exists`): Ensure inclusion without ordering
- **Override** (`!prefix`): Force dependency retention

## Available Skills

### Project-Specific Skills

#### 🔍 discovering-sql-dependencies
Comprehensive guide for automatic SQL dependency discovery:
- Pattern-based dependency extraction
- Configuration via CLI or TOML
- Merge strategies for manual/automatic dependencies
- Header generation and updates
- Migration from manual to automatic discovery

#### 🏗️ understanding-architecture
Deep dive into implementation details:
- Module responsibilities and interactions
- Layer system implementation
- Graph validation and cycle detection
- Stable topological sort algorithm
- Performance considerations
- Extension points for customization

#### 🧪 testing-topcat
Testing and debugging guidance:
- Test organization and best practices
- Running and writing tests
- Debugging techniques with verbose mode
- DOT graph visualization
- Performance testing and profiling

### Meta-Skills for Skill Management

#### ✍️ writing-skills
Creating new skills from scratch:
- Proper structure and metadata
- Progressive disclosure patterns
- Testing and iteration workflow
- Templates for common skill types

#### 🔄 updating-skills
Holistically refactoring existing skills:
- Refactoring patterns
- Migration strategies
- Testing updates
- Maintaining quality standards

#### 📝 updating-claude-md
Maintaining CLAUDE.md quality and conciseness:
- Refactor vs patch approach
- Content vs skill decisions
- Structure optimization
- Token cost awareness

## Common Tasks

### Basic Concatenation

```bash
# Simple concatenation with default settings
topcat -i sql/ -o output.sql

# With custom layers
topcat -i sql/ -o output.sql --layers "ddl,dml,indexes"

# Filter by extension
topcat -i migrations/ -o all.sql --include-exts sql,ddl
```

### SQL Dependency Discovery

```bash
# Enable discovery with pattern
topcat -i sql/ -o output.sql \
  --enable-sql-discovery \
  --schema-pattern "app_\\w+"

# Use config file
topcat -i sql/ -o output.sql --sql-config topcat.toml

# Update headers in-place
topcat -i sql/ -o output.sql \
  --enable-sql-discovery \
  --update-headers
```

### Filtering

```bash
# By glob pattern
topcat -i sql/ -o output.sql --include-glob "**/migrations/*.sql"

# By prefix
topcat -i sql/ -o output.sql --include-prefix "v2_"

# Subdirectory with dependency pulling
topcat -i sql/ -o output.sql --subdir-filter "customer/"
```

### Debugging

```bash
# Verbose output with graph
topcat -i sql/ -o output.sql -v

# Debug specific module
RUST_LOG=topcat::file_dag=debug cargo run -- -i sql/ -o output.sql

# Generate graph visualization
topcat -i sql/ -o output.sql -v 2>&1 | \
  grep "digraph" -A 1000 > graph.dot && \
  dot -Tpng graph.dot -o graph.png
```

## Configuration

### TOML Config Example

```toml
[sql_discovery]
enabled = true
schema_pattern = "(?:app|test)_\\w+"
merge_strategy = "discovery-only"

[[sql_discovery.type_mappings]]
from = "JSONB"
to = "pg_catalog.jsonb"

[[sql_discovery.extension_mappings]]
object = "uuid_generate_v4"
extension = "uuid-ossp"
```

## Error Resolution

| Error | Solution |
|-------|----------|
| Cycle detected | Check dependencies, use layers to break cycles |
| Missing dependency | Ensure file exists or use `exists` for soft deps |
| Cross-layer violation | Move file to appropriate layer |
| Duplicate names | Ensure unique `name` metadata across files |

## Best Practices

1. **Use meaningful names** in metadata that reflect the file's purpose
2. **Leverage layers** for high-level ordering (DDL before DML)
3. **Start with discovery** for SQL projects to avoid manual maintenance
4. **Test incrementally** on subsets before processing entire codebases
5. **Version control** your `topcat.toml` configuration
6. **Use verbose mode** for debugging dependency issues

## Skill Maintenance Workflow

**IMPORTANT**: Skills should evolve with the codebase to capture learnings and new capabilities.

### When to Check Skills

After completing any of the following, **proactively ask the user** if skills should be updated:

1. **Feature Implementation**: "I've completed the new feature. Should I check if any skills need updating to reflect this new capability?"

2. **Bug Fix**: "I've fixed the bug. Should I update the relevant skill to document this issue and its solution?"

3. **Better Approach Found**: "I discovered a better way to do this. Should I update the skill to reflect the improved approach?"

4. **Complex Task Completed**: "This was a complex process. Should I create a skill to capture this workflow for future use?"

5. **Repeated Questions**: "You've asked about this several times. Should I create a skill to document this pattern?"

### During Planning

When planning complex tasks, consider:
- "Are there existing skills that could help with this task?"
- "Will this work create new patterns worth capturing in a skill?"
- "Should I plan to update skills as part of this task?"

### Skill Update Guidelines

When updating skills:
- **Use the `updating-skills` skill** for proper refactoring approach
- **Don't just append** - refactor holistically to maintain quality
- **Test updates** with a fresh context to ensure effectiveness
- **Keep skills focused** - consider creating new skills rather than expanding scope

### Available Meta-Skills

- **writing-skills**: For creating new skills from scratch
- **updating-skills**: For holistically refactoring existing skills
- **updating-claude-md**: For maintaining CLAUDE.md conciseness and quality

## Need More Details?

Load the appropriate skill for in-depth information:
- **discovering-sql-dependencies**: Automatic dependency extraction
- **understanding-architecture**: Implementation details and internals
- **testing-topcat**: Comprehensive testing and debugging guide
- **writing-skills**: Creating new skills from scratch
- **updating-skills**: Refactoring existing skills
- **updating-claude-md**: Maintaining CLAUDE.md quality
