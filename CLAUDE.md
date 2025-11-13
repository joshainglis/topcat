# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with Topcat.

## Project Overview

**Topcat** is a Rust CLI tool for **topological concatenation of files** with comprehensive dependency analysis. It reads files with dependency metadata, builds a directed acyclic graph (DAG), performs topological sorting respecting layer constraints, and provides analysis, cleanup, schema management, and export capabilities.

Primary use case: Ordering SQL migration files where execution order matters based on dependencies. Also analyzes dependency health, detects dead code, and safely cleans up unused files.

## Quick Start

```bash
# Build and run
cargo build --release
cargo run -- -i input_dir/ -o output.sql

# Concatenation
topcat concat -i sql/ -o migrations.sql             # Concatenate files
topcat concat -i sql/ -o output.sql --enable-sql-discovery --schema-pattern "myapp_\\w+"

# Analysis
topcat analyze -i sql/ -e sql dead-branches         # Find dead code
topcat analyze -i sql/ -e sql cycles                # Detect circular deps
topcat analyze -i sql/ -e sql orphans               # Find isolated files

# Cleanup (dry-run by default)
topcat clean -i sql/ -e sql dead-branches           # Preview deletion
topcat clean -i sql/ -e sql orphans --no-dry-run    # Actually delete

# Schema operations
topcat schema -i sql/ -e sql list                   # View all schemas
topcat schema -i sql/ -e sql analyze my_schema      # Detailed schema view

# Export
topcat export -i sql/ -e sql -o graph.json json     # Export to JSON
topcat export -i sql/ -e sql -o graph.dot dot       # Export to GraphViz
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
| `main.rs` | CLI parsing, command routing |
| `file_node.rs` | File representation with metadata, schema extraction |
| `file_dag.rs` | DAG management, validation, schema operations |
| `stable_topo.rs` | Deterministic topological sort |
| `commands/concat.rs` | File concatenation command |
| `commands/analyze.rs` | Dependency analysis commands |
| `commands/clean.rs` | Safe file deletion commands |
| `commands/schema.rs` | Schema operations |
| `commands/export.rs` | Graph export in multiple formats |
| `analysis/mod.rs` | GraphAnalyzer trait, analysis algorithms |
| `analysis/root_matcher.rs` | Root node protection patterns |
| `analysis/external_usage.rs` | External usage checking |
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

## Common Commands

### Concatenation

```bash
topcat concat -i sql/ -o output.sql                      # Basic concat
topcat concat -i sql/ -o output.sql --enable-sql-discovery --schema-pattern "myapp_\\w+"
# See 'discovering-sql-dependencies' skill for detailed workflows
```

### Analysis Commands

| Command | Purpose |
|---------|---------|
| `analyze dead-branches` | Find complete dead subtrees |
| `analyze orphans` | Find isolated files |
| `analyze cycles` | Detect circular dependencies |
| `analyze missing` | Find missing dependencies |
| `analyze leaf-nodes` | Files with no dependents |
| `analyze root-nodes` | Files with no dependencies |
| `analyze file <path>` | Deep analysis of single file |

```bash
# With protection patterns
topcat analyze -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

# With external usage checking
topcat analyze -i sql/ -e sql --external-check-dir src/ --external-check-pattern "*.py" dead-branches

# CI/CD quiet mode
topcat analyze -i sql/ -e sql --quiet cycles  # Exit code 0/1
```

### Cleanup Commands

| Command | Default | Purpose |
|---------|---------|---------|
| `clean dead-branches` | Dry-run | Remove dead subtrees |
| `clean orphans` | Dry-run | Remove isolated files |
| `clean unrequired` | Dry-run | Remove unrequired files |

```bash
topcat clean -i sql/ -e sql dead-branches                # Preview
topcat clean -i sql/ -e sql dead-branches --no-dry-run   # Execute with confirmation
topcat clean -i sql/ -e sql orphans --no-dry-run --force # Force mode (no confirmation)
```

### Schema Commands

```bash
topcat schema -i sql/ -e sql list                 # View all schemas with stats
topcat schema -i sql/ -e sql analyze my_schema    # Detailed schema view
topcat schema -i sql/ -e sql dependencies         # Cross-schema dependencies

# Schema filtering
topcat analyze -i sql/ -e sql --schema auth dead-branches
topcat clean -i sql/ -e sql --schema billing orphans --no-dry-run
```

### Export Commands

| Format | Use Case |
|--------|----------|
| `json` | API integration, programmatic access |
| `dot` | GraphViz visualization |
| `graphml` | Gephi/yEd import |
| `mermaid` | Markdown diagrams |

```bash
topcat export -i sql/ -e sql -o graph.json json           # Full graph
topcat export -i sql/ -e sql --mode deps --node my_node -o deps.json json  # Dependencies
topcat export -i sql/ -e sql --schema auth -o auth.dot dot  # Schema-filtered
```

## Configuration

Use `topcat.toml` for project-specific settings:

```toml
[sql_discovery]
enabled = true
schema_pattern = "(?:app|test)_\\w+"

[analysis]
root_patterns = ["**/api/*.sql", "**/migrations/*.sql"]
```

See `discovering-sql-dependencies` and `analyzing-dependencies` skills for detailed configuration.

## Error Resolution

| Error | Solution |
|-------|----------|
| Cycle detected | Use `analyze cycles` to identify, break with layers or soft deps |
| Missing dependency | Use `analyze missing` to find, then fix or add files |
| Cross-layer violation | Move file to appropriate layer |
| Duplicate names | Ensure unique `name` metadata across files |
| False positives in dead branches | Add `--root-pattern` or `--external-check-dir` |

## Best Practices

1. **Use meaningful names** in metadata that reflect the file's purpose
2. **Leverage layers** for high-level ordering (DDL before DML)
3. **Start with discovery** for SQL projects to avoid manual maintenance
4. **Analyze before cleaning** - use `analyze dead-branches` before `clean`
5. **Protect entry points** - use `--root-pattern` to prevent accidental deletion
6. **Use dry-run mode** - cleanup defaults to dry-run for safety
7. **Check health regularly** - run `analyze cycles` and `analyze missing` in CI
8. **Version control** your `topcat.toml` configuration

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
- **analyzing-dependencies**: Analysis commands, safe cleanup, CI/CD integration
- **managing-schemas**: Schema operations, filtering, cross-schema dependencies
- **exporting-graphs**: Graph export formats, visualization workflows
- **discovering-sql-dependencies**: Automatic dependency extraction
- **understanding-architecture**: Implementation details and internals
- **testing-topcat**: Comprehensive testing and debugging guide
- **writing-skills**: Creating new skills from scratch
- **updating-skills**: Refactoring existing skills
- **updating-claude-md**: Maintaining CLAUDE.md quality
- **writing-rust-topcat**: Rust conventions for Topcat
- **clippy-fixing**: Systematic linting workflow
- **creating-tests-topcat**: Writing tests following project patterns
