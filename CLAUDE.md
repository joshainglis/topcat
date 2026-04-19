# Topcat development guide

## Project Overview

**Topcat** is a Rust CLI tool for **topological concatenation of files** with comprehensive dependency analysis. It reads files with dependency metadata, builds a directed acyclic graph (DAG), performs topological sorting respecting layer constraints, and provides analysis, cleanup, schema management, and export capabilities.

Primary use case: ordering SQL migration files where execution order matters based on dependencies. Also analyzes dependency health, detects dead code, and safely cleans up unused files.

## Development Commands

```bash
nix develop                      # Enter dev shell with all dependencies

cargo build                      # Debug build
cargo build --release            # Release build
cargo test                       # Run all tests
cargo test test_name             # Run specific test
cargo clippy                     # Lint code
cargo fmt                        # Format code

cargo run -- concat -i tests/input/sql /tmp/output.sql
```

## Architecture Summary

### Core Modules

| Module | Purpose |
|--------|---------|
| `main.rs` | CLI parsing, command routing |
| `cli/` | Composable argument groups (7 modules: global, input, filter, analysis, sql_discovery, formatting, execution) |
| `settings/` | Unified configuration system (5 modules: loading, validation, configs, tests) |
| `file_node/` | File representation with metadata (5 modules: parsing, soft_deps, filename, tests) |
| `file_dag/` | DAG management and validation (6 modules: core, builder, validation, schema, filters) |
| `commands/` | Command implementations (concat, update, config, schema, import + analyze/, clean/, export/) |
| `analysis/` | GraphAnalyzer trait, root matching, external usage checking |

### Utility Modules

| Module | Purpose |
|--------|---------|
| `display_utils/` | Table creation, tree rendering, output formatting (4 modules) |
| `header_generator/` | Header generation and in-place updates (4 modules) |
| `sql_parser/` | SQL dependency extraction (3 modules: analyzer, tests) |
| `schema_utils.rs` | SchemaFilter with matching/filtering operations |
| `graph_utils.rs` | Node mapping, graph analysis helpers |
| `stable_topo.rs` | Deterministic topological sort |
| `logging.rs` | Logging with verbose/quiet modes |

### Modularized Commands

**Analyze Command** (`commands/analyze/`): 10 focused modules
- `mod.rs` - CLI routing and dispatch
- `common.rs` - Generic display utilities, shared analysis logic
- `cycles.rs`, `dead_branches.rs`, `file.rs`, `missing.rs` - Analysis implementations
- `leaf_nodes.rs`, `orphans.rs`, `root_nodes.rs`, `unrequired.rs` - Node categorization

**Clean Command** (`commands/clean/`): 6 focused modules
- `mod.rs` - CLI routing and dispatch
- `common.rs` - DeletionContext, confirmation logic
- `dead_branches.rs`, `orphans.rs`, `targets.rs`, `unrequired.rs` - Cleanup implementations

## Key Concepts

### File Metadata

Files include dependency metadata in header comments:

```sql
-- name: create_users_table
-- requires: create_schema
-- layer: normal
-- exists: extensions

CREATE TABLE users (...);
```

### Layers

Layers enforce ordering between groups of files:
- Default: `prepend` → `normal` → `append`
- Custom: `--layers first,second,third`
- Files in earlier layers always precede later layers

### Dependencies

- **Hard** (`requires`, `dropped_by`): Enforce ordering
- **Soft** (`exists`): Ensure inclusion without ordering
- **Override** (`!prefix`): Force dependency retention

## Common Commands

### Concatenation

```bash
topcat concat -i sql/ -e sql output.sql
# See 'discovering-sql-dependencies' skill for detailed workflows
```

### Update

Discovers dependencies from SQL content and updates file headers. Can also rename files based on discovered node names.

**Smart defaults:** With SQL extensions (`-e sql`, `-e pg`, etc.), SQL discovery, header updates, and file renaming are all enabled by default.

```bash
topcat update -i sql/ -e sql                           # Preview (dry-run default)
topcat update -i sql/ -e sql --mode execute            # Apply changes
topcat update -i sql/ -e sql --generate-headers ./updated/   # Emit to separate dir
topcat update -i sql/ -e sql --schema-pattern "myapp_\\w+"   # Custom schema pattern
topcat update -i sql/ -e sql --no-rename-files         # Disable rename
topcat update -i sql/ -e sql --no-sql-discovery        # Header-only parsing
```

### Analysis

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
topcat analyze -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches
topcat analyze -i sql/ -e sql --external-check-dir src/ --external-check-pattern "*.py" dead-branches
topcat analyze -i sql/ -e sql --quiet cycles           # CI mode — exit code 0/1
```

### Cleanup (dry-run by default)

| Command | Purpose |
|---------|---------|
| `clean dead-branches` | Remove dead subtrees |
| `clean orphans` | Remove isolated files |
| `clean unrequired` | Remove unrequired files |

```bash
topcat clean -i sql/ -e sql dead-branches                   # Preview
topcat clean -i sql/ -e sql dead-branches --mode execute    # Execute with confirmation
topcat clean -i sql/ -e sql orphans --mode execute --force  # No confirmation
```

### Schema

```bash
topcat schema -i sql/ -e sql list                 # All schemas with stats
topcat schema -i sql/ -e sql analyze my_schema    # Detailed schema view
topcat schema -i sql/ -e sql dependencies         # Cross-schema dependencies
topcat analyze -i sql/ -e sql --schema auth dead-branches   # Filter analysis by schema
```

### Export

Formats: `json` (API/programmatic), `dot` (GraphViz), `graphml` (Gephi/yEd), `mermaid` (Markdown).

```bash
topcat export -i sql/ -e sql graph.json json
topcat export -i sql/ -e sql deps.json json --mode deps --node my_node
topcat export -i sql/ -e sql auth.dot dot --schema auth
```

### Import

Split a pg_dump into per-object SQL files with topcat headers. See the README for the full list of supported PostgreSQL object types and output directory structure.

```bash
topcat import pg-dump database.sql ./output/              # Split dump
topcat import pg-dump database.sql ./output/ --dry-run    # Preview
topcat import pg-dump database.sql ./output/ --schema-pattern "app_\\w+"

topcat import pg-dump database.sql ./output/ --generate-layers false  # No layer headers
topcat import pg-dump database.sql ./output/ --generate-deps false    # No requires headers
topcat import pg-dump database.sql ./output/ --include-acl false      # Skip ACL
topcat import pg-dump database.sql ./output/ --include-owner false    # Skip OWNER
```

### Config

```bash
topcat config show                 # Effective configuration
topcat config validate             # Validate config file
topcat config generate             # Generate example config
```

## Configuration

Precedence (highest → lowest): CLI args → `TOPCAT_*` env vars → `./topcat.toml` → `~/.config/topcat/config.toml` → `/etc/topcat/config.toml` → defaults.

Example `topcat.toml`:

```toml
[sql_discovery]
enabled = true
schema_pattern = "(?:app|test)_\\w+"
extension_mappings = { "nlevel" = "ltree", "digest" = "pgcrypto" }
type_mappings = { "TSTZRANGE" = "c_tmf.t_time_period" }

[analysis]
root_patterns = ["**/api/*.sql", "**/migrations/*.sql"]
external_check_dirs = ["src/", "app/"]
external_check_patterns = ["*.py", "*.ts"]

[layers]
names = ["prepend", "normal", "append"]
fallback = "normal"
```

Environment variables use `TOPCAT_*`, double-underscore for nesting (`TOPCAT_SQL_DISCOVERY__ENABLED`), comma-separated for arrays. Full reference in README.

## Error Resolution

| Error | Solution |
|-------|----------|
| Cycle detected | `analyze cycles` to identify; break with layers or soft deps |
| Missing dependency | `analyze missing` to find; fix or add files |
| Cross-layer violation | Move file to appropriate layer |
| Duplicate names | Ensure unique `name` metadata across files |
| False positives in dead branches | Add `--root-pattern` or `--external-check-dir` |

## Project-specific gotchas

- **Cleanup defaults to dry-run.** `--mode execute` is required to actually delete; `--force` skips confirmation.
- **`--root-pattern` protects entry points** from being flagged as dead — use for API/migration roots before running `clean`.
- **Run `analyze cycles` and `analyze missing` in CI** (both support `--quiet` with exit 0/1).

## Skills

After completing features, bug fixes, or discovering better approaches, consider whether the relevant skill needs updating. Use `updating-skills` for holistic refactoring (not just appending). Use `writing-skills` to create new skills.

Load a skill for deeper info:
- **analyzing-dependencies**: Analysis commands, safe cleanup, CI/CD integration
- **managing-schemas**: Schema operations, filtering, cross-schema dependencies
- **exporting-graphs**: Graph export formats, visualization workflows
- **discovering-sql-dependencies**: Automatic dependency extraction
- **understanding-architecture**: Implementation details and internals
- **testing-topcat**, **creating-tests-topcat**: Testing and debugging
- **writing-rust-topcat**: Rust conventions for Topcat
- **clippy-fixing**: Systematic linting workflow
- **writing-skills**, **updating-skills**, **updating-claude-md**: Meta-skills
