# Topcat

**Top**ological con**cat**enation of files with comprehensive dependency analysis.

## Overview

Topcat is a Rust CLI tool that reads files with dependency metadata, builds a directed acyclic graph (DAG), performs topological sorting with layer constraints, and provides powerful analysis, cleanup, and export capabilities.

**Primary use case:** Managing SQL migration files where execution order matters based on dependencies. Also analyzes dependency health, detects dead code, and safely cleans up unused files.

## Features

- 🔗 **Topological Concatenation** - Order files correctly based on dependencies
- 🔍 **Dependency Analysis** - Find dead code, cycles, orphans, and missing dependencies
- 🧹 **Safe Cleanup** - Remove unused files with multi-stage verification
- 📊 **Schema Management** - Analyze and filter by schema boundaries
- 📤 **Graph Export** - Export to JSON, GraphViz, GraphML, and Mermaid
- 🤖 **SQL Auto-Discovery** - Automatically extract dependencies from SQL code
- 🛡️ **Protection Patterns** - Safeguard entry points from accidental deletion
- 📐 **Layer System** - Enforce high-level ordering constraints
- ⚙️ **Configuration** - Project-specific settings via `topcat.toml`

## Installation

### From Crates.io

```bash
cargo install topcat
```

### From Source

```bash
git clone https://github.com/joshainglis/topcat.git
cd topcat
cargo build --release
./target/release/topcat --help
```

### Via Nix

```bash
nix develop  # Enter development environment with all dependencies
```

## Quick Start

```bash
# Concatenate SQL files in dependency order
topcat concat -i sql/ -o migrations.sql

# Find dead code that can be safely removed
topcat analyze -i sql/ -e sql dead-branches

# Preview deletion of orphaned files
topcat clean -i sql/ -e sql orphans

# List all schemas with statistics
topcat schema -i sql/ -e sql list

# Export dependency graph for visualization
topcat export -i sql/ -e sql -o graph.json json
```

## Commands

### `concat` - Concatenate Files

Concatenate files in topological order respecting dependencies and layer constraints.

```bash
topcat concat -i sql/ -o output.sql
topcat concat -i sql/ -o output.sql --enable-sql-discovery
topcat concat -i dir1/ -i dir2/ -o output.sql --layers prepend,normal,append
```

**Basic Options:**
- `-i, --input-dirs <DIR>...` - Input directories (multiple allowed)
- `-o, --output-file <FILE>` - Output file path
- `-e, --include-exts <EXT>...` - File extensions to include (e.g., `sql`)
- `-E, --exclude-exts <EXT>...` - File extensions to exclude
- `-g, --include-glob <PATTERN>...` - Include files matching glob
- `-G, --exclude-glob <PATTERN>...` - Exclude files matching glob
- `-d, --dry-run` - Preview output without writing
- `-v, --verbose` - Show debug information

**Layer Options:**
- `--layers <LAYERS>` - Custom layer ordering (comma-separated, default: `prepend,normal,append`)
- `--fallback-layer <LAYER>` - Default layer for files without declaration (default: `normal`)

**Filtering Options:**
- `--include-prefix <PREFIX>...` - Only include nodes with these name prefixes
- `--exclude-prefix <PREFIX>...` - Exclude nodes with these name prefixes
- `--subdir-filter <PATH>` - Include only files from subdirectory and their dependencies

**SQL Discovery Options:**
- `--enable-sql-discovery` - Extract dependencies from SQL code
- `--schema-pattern <REGEX>` - Pattern for schema names (e.g., `"myapp_\\w+"`)
- `--merge-strategy <STRATEGY>` - How to merge manual vs discovered deps:
  - `header-only` - Use only manual headers
  - `discovery-only` - Use only discovered (default)
  - `union` - Combine both
  - `header-with-fallback` - Manual if present, else discovered
  - `validate` - Check for discrepancies (fails on mismatch)

**Header Management:**
- `--update-headers` - Update source files with discovered dependencies
- `--generate-headers <DIR>` - Write files with updated headers to directory

**Formatting Options:**
- `-c, --comment-prefix <STR>` - Comment string (default: `--`)
- `-s, --file-separator <STR>` - Separator between concatenated files
- `-a, --file-suffix <STR>` - Ensure files end with this (default: `;`)

### `analyze` - Dependency Analysis

Analyze dependency structure and health with multiple analysis types.

```bash
topcat analyze -i sql/ -e sql dead-branches
topcat analyze -i sql/ -e sql --schema auth orphans
topcat analyze -i sql/ -e sql --quiet cycles  # CI/CD mode
```

**Analysis Types:**

| Type | Description | Exit Code on Issue |
|------|-------------|-------------------|
| `dead-branches` | Complete dead subtrees (transitive) | 0 (informational) |
| `orphans` | Files with no dependencies AND no dependents | 0 (informational) |
| `unrequired` | Files not required by any other files | 0 (informational) |
| `leaf-nodes` | Files with dependencies but no dependents | 0 (informational) |
| `root-nodes` | Files with dependents but no dependencies | 0 (informational) |
| `cycles` | Circular dependency detection | **1 (error)** |
| `missing` | Referenced but non-existent dependencies | **1 (error)** |
| `file <path>` | Detailed analysis of specific file | 0 |

**Protection Options:**
- `--root-nodes <NODE>...` - Specific nodes to protect (e.g., `api_main`)
- `--root-pattern <GLOB>...` - Glob patterns for files (e.g., `**/api/*.sql`)
- `--root-regex <REGEX>...` - Regex for node names (e.g., `^api_.*`)
- `--root-dir <DIR>...` - Directories to protect (e.g., `api/`)

**External Usage Checking:**
- `--external-check-dir <DIR>...` - Check these directories for usage
- `--external-check-pattern <PATTERN>...` - File patterns to check (e.g., `*.py`, `*.rs`)

**Filtering:**
- `--schema <SCHEMA>...` - Filter to specific schemas

**Output:**
- `-v, --verbose` - Show debug information
- `-q, --quiet` - Suppress output (CI/CD mode, only exit codes)

### `clean` - Safe File Deletion

Remove files based on analysis with safety checks and dry-run default.

```bash
topcat clean -i sql/ -e sql dead-branches              # Preview (dry-run)
topcat clean -i sql/ -e sql orphans --no-dry-run       # Execute with confirmation
topcat clean -i sql/ -e sql orphans --no-dry-run -f    # Force (no confirmation)
```

**Clean Types:**
- `dead-branches` - Remove complete dead subtrees
- `orphans` - Remove isolated files
- `unrequired` - Remove unrequired files
- `targets <files>...` - Remove specific targets (with dependency check)

**Safety Features:**
- `--dry-run` - Preview deletion (**DEFAULT** - always safe by default)
- `--no-dry-run` - Actually perform deletion
- `-f, --force` - Skip interactive confirmation (for automation)
- Protection checks prevent deleting files with dependents
- All protection and filtering options from `analyze` available

### `schema` - Schema Management

Analyze and manage schema boundaries in multi-schema projects.

```bash
topcat schema -i sql/ -e sql list                    # All schemas with stats
topcat schema -i sql/ -e sql analyze my_schema       # Detailed schema view
topcat schema -i sql/ -e sql dependencies            # Cross-schema deps
```

**Schema Operations:**
- `list` - List all schemas with file counts and distribution
- `analyze <schema>` - Detailed analysis showing:
  - Files in schema
  - Internal dependencies
  - External dependencies (grouped by target schema)
  - Schemas that depend on this schema
- `dependencies` - Cross-schema dependency table

### `export` - Graph Export

Export dependency graphs in multiple formats for visualization and integration.

```bash
topcat export -i sql/ -e sql -o graph.json json                    # Full graph
topcat export -i sql/ -e sql -o graph.dot dot                      # GraphViz
topcat export -i sql/ -e sql --mode deps --node my_node -o deps.json json
```

**Export Formats:**
- `json` - JSON with full metadata
- `dot` - GraphViz DOT format
- `graphml` - GraphML for Gephi/yEd
- `mermaid` - Mermaid diagram syntax

**Export Modes:**
- `--mode full` - Entire dependency graph (default)
- `--mode deps` - Node and all transitive dependencies
- `--mode dependents` - Node and all transitive dependents
- `--mode direct` - Node and immediate neighbors only

**Options:**
- `-o, --output <FILE>` - Output file path (required)
- `--node <NAME>` - Target node (required for deps/dependents/direct modes)
- `--schema <SCHEMA>...` - Filter by schemas

## File Metadata

Files specify dependencies and properties via header comments:

```sql
-- name: create_users_table
-- requires: create_schema, create_extensions
-- dropped_by: drop_schema
-- exists: audit_trigger
-- layer: normal

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email TEXT UNIQUE NOT NULL
);
```

### Metadata Headers

**Required:**
- `-- name: <unique_name>` - Unique node identifier (REQUIRED)

**Dependencies:**
- `-- requires: dep1, dep2` - Hard dependencies (enforces ordering)
- `-- dropped_by: dep` - Alias for `requires` (semantic clarity for DDL drops)
- `-- exists: dep` - Soft dependencies (ensures inclusion, no ordering)
- `-- !override_dep` - Prefix with `!` to force dependency retention

**Layer Declaration:**
- `-- layer: <layer_name>` - Explicit layer assignment
- `-- is_initial` - Legacy: maps to "prepend" layer
- `-- is_final` - Legacy: maps to "append" layer

**Schema Extraction:**
- Automatic from node names: `schema.table` → schema is "schema"
- Also supports: `schema::table` (PostgreSQL namespace style)

### Dependency Types

| Type | Syntax | Behavior |
|------|--------|----------|
| **Hard** | `requires:`, `dropped_by:` | Enforces execution order |
| **Soft** | `exists:` | Ensures file inclusion without ordering |
| **Override** | `!prefix` | Forces dependency retention despite patterns |

### Layer System

Layers enforce high-level ordering between groups of files. Files in earlier layers always execute before later layers.

**Default Layers:** `prepend` → `normal` → `append`

**Custom Layers:**
```bash
topcat concat -i sql/ -o output.sql --layers setup,functions,views,cleanup
```

**Use Cases:**
- `prepend`: Schema creation, extensions, types
- `normal`: Tables, functions, main logic
- `append`: Grants, post-deployment scripts

**Rules:**
- Files in earlier layers cannot depend on later layers
- Files in the same layer are ordered by dependencies
- Files without layer declaration use `--fallback-layer` (default: `normal`)

## Configuration

Use `topcat.toml` in your project root for persistent configuration:

```toml
[sql_discovery]
enabled = true
schema_pattern = "(?:app|test)_\\w+"
object_pattern = "\\w+"
merge_strategy = "discovery-only"

# Map SQL types to their defining objects
[sql_discovery.type_mappings]
TSTZRANGE = "c_tmf.t_time_period"
my_enum = "schema.enum_definition"

# Map extensions to their providers
[sql_discovery.extension_mappings]
digest = "pgcrypto"
uuid_generate_v4 = "uuid-ossp"

# Patterns to ignore during discovery
strip_suffixes = ["_or_ref", "_view"]
model_gen_patterns = ["codegen_tmf\\.proc_(?:make_model|combine_enums)"]

[analysis]
# Protect these nodes from dead branch detection
root_nodes = ["api_main", "public_entry"]
root_patterns = ["**/api/*.sql", "**/public/*.sql"]
root_regex = ["^api_.*", "^public_.*"]
root_dirs = ["api/", "migrations/"]

# Check for external usage
external_check_dirs = ["src/", "app/"]
external_check_patterns = ["*.py", "*.rs", "*.ts"]
```

### SQL Discovery

Automatically extract dependencies from SQL code, eliminating manual header maintenance:

```bash
topcat concat -i sql/ -o output.sql --enable-sql-discovery --schema-pattern "myapp_\\w+"
```

**Discovery Features:**
- Extracts table, view, function, type, and extension dependencies
- Configurable schema and object patterns
- Type and extension mappings for system objects
- Multiple merge strategies for combining with manual headers

See configuration section for detailed `sql_discovery` options.

## Examples

### Basic SQL Project

**Directory Structure:**
```
sql/
├── schema.sql
├── functions/
│   ├── user_auth.sql
│   └── user_profile.sql
└── views/
    └── active_users.sql
```

**File: sql/schema.sql**
```sql
-- name: myapp_schema
-- layer: prepend

DROP SCHEMA IF EXISTS myapp CASCADE;
CREATE SCHEMA myapp;
```

**File: sql/functions/user_auth.sql**
```sql
-- name: myapp.user_auth
-- dropped_by: myapp_schema
-- requires: myapp_schema

CREATE FUNCTION myapp.user_auth(email TEXT) RETURNS BOOLEAN AS $$
    SELECT EXISTS(SELECT 1 FROM myapp.users WHERE email = $1);
$$ LANGUAGE SQL;
```

**File: sql/views/active_users.sql**
```sql
-- name: myapp.active_users
-- dropped_by: myapp_schema
-- exists: myapp.user_auth

CREATE VIEW myapp.active_users AS
    SELECT * FROM myapp.users WHERE last_login > NOW() - INTERVAL '30 days';
```

**Concatenate:**
```bash
topcat concat -i sql/ -o migrations/deploy.sql -e sql
```

**Result:** Files ordered as `schema.sql` → `user_auth.sql` → `user_profile.sql` → `active_users.sql`

### Dead Code Cleanup Workflow

```bash
# Step 1: Analyze and find dead branches
topcat analyze -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

# Step 2: Preview deletion (dry-run is default)
topcat clean -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

# Step 3: Execute deletion with confirmation
topcat clean -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches --no-dry-run

# Step 4: Verify no cycles or missing deps remain
topcat analyze -i sql/ -e sql cycles
topcat analyze -i sql/ -e sql missing
```

### Multi-Schema Project

```bash
# List all schemas
topcat schema -i sql/ -e sql list

# Analyze specific schema
topcat schema -i sql/ -e sql analyze auth

# Show cross-schema dependencies
topcat schema -i sql/ -e sql dependencies

# Concatenate only one schema
topcat concat -i sql/ -e sql -o auth.sql --include-prefix auth.
```

### CI/CD Integration

**GitHub Actions Example:**
```yaml
name: Check SQL Dependencies

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo install topcat

      # Fail on cycles
      - run: topcat analyze -i sql/ -e sql --quiet cycles

      # Fail on missing dependencies
      - run: topcat analyze -i sql/ -e sql --quiet missing

      # Report dead branches (informational)
      - run: topcat analyze -i sql/ -e sql dead-branches
```

**Pre-commit Hook:**
```bash
#!/bin/bash
# .git/hooks/pre-commit

# Check for cycles
if ! topcat analyze -i sql/ -e sql --quiet cycles; then
    echo "Error: Circular dependencies detected!"
    exit 1
fi

# Check for missing dependencies
if ! topcat analyze -i sql/ -e sql --quiet missing; then
    echo "Error: Missing dependencies detected!"
    exit 1
fi
```

### Graph Visualization

```bash
# Export to GraphViz and render
topcat export -i sql/ -e sql -o graph.dot dot
dot -Tpng graph.dot -o graph.png

# Export to Mermaid for documentation
topcat export -i sql/ -e sql -o graph.md mermaid

# Export dependencies of specific node
topcat export -i sql/ -e sql --mode deps --node api_main -o api_deps.json json
```

## Best Practices

### 1. Use Meaningful Names
```sql
-- Good
-- name: auth.create_user_function
-- name: billing.monthly_invoice_view

-- Avoid
-- name: function1
-- name: temp
```

### 2. Leverage Layers for High-Level Organization
```sql
-- Schema setup (prepend layer)
-- name: schema_init
-- layer: prepend

-- Core logic (normal layer - default)
-- name: user_functions
-- layer: normal

-- Post-deployment (append layer)
-- name: grant_permissions
-- layer: append
```

### 3. Use Discovery for SQL Projects
```bash
# Enable discovery to avoid manual maintenance
topcat concat -i sql/ -o output.sql --enable-sql-discovery --schema-pattern "myapp_\\w+"

# Validate your manual headers match reality
topcat concat -i sql/ -o output.sql --enable-sql-discovery --merge-strategy validate
```

### 4. Protect Entry Points
```bash
# Prevent accidental deletion of API endpoints
topcat analyze -i sql/ -e sql \
    --root-pattern "**/api/*.sql" \
    --root-pattern "**/public/*.sql" \
    dead-branches
```

### 5. Check External Usage
```bash
# Verify SQL objects aren't used in application code
topcat analyze -i sql/ -e sql \
    --external-check-dir src/ \
    --external-check-pattern "*.py" \
    --external-check-pattern "*.ts" \
    dead-branches
```

### 6. Always Dry-Run First
```bash
# Default is safe (dry-run)
topcat clean -i sql/ -e sql dead-branches

# Only execute after reviewing
topcat clean -i sql/ -e sql dead-branches --no-dry-run
```

### 7. Use Configuration Files
```toml
# topcat.toml - commit to version control
[analysis]
root_patterns = ["**/api/*.sql"]
external_check_dirs = ["src/", "app/"]
external_check_patterns = ["*.py"]

[sql_discovery]
enabled = true
schema_pattern = "myapp_\\w+"
```

### 8. Integrate with CI/CD
```bash
# Fail builds on dependency issues
topcat analyze -i sql/ -e sql --quiet cycles || exit 1
topcat analyze -i sql/ -e sql --quiet missing || exit 1
```

## Troubleshooting

### Circular Dependencies
```bash
# Detect cycles
topcat analyze -i sql/ -e sql cycles

# Solutions:
# 1. Use soft dependencies (exists:) instead of hard (requires:)
# 2. Split files into different layers
# 3. Reorganize to break the cycle
```

### Missing Dependencies
```bash
# Find missing deps
topcat analyze -i sql/ -e sql missing

# Solutions:
# 1. Add the missing file
# 2. Fix the typo in the dependency name
# 3. Remove the dependency if no longer needed
```

### Cross-Layer Violations
```
Error: Node 'views.user_summary' in layer 'functions' depends on 'schema.init' in layer 'setup'
```

**Solution:** Assign correct layer to file or restructure layers:
```sql
-- Change layer assignment
-- layer: setup
```

### False Positives in Dead Branch Detection
```bash
# Protect known entry points
topcat analyze -i sql/ -e sql \
    --root-pattern "**/api/*.sql" \
    --external-check-dir app/ \
    dead-branches
```

### Duplicate Names
```
Error: Duplicate node name 'schema.users' found in:
  - sql/v1/users.sql
  - sql/v2/users.sql
```

**Solution:** Ensure each file has a unique name:
```sql
-- name: schema.users_v1
-- name: schema.users_v2
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

See [LICENSE](LICENSE) for details.

## Links

- **Repository:** https://github.com/joshainglis/topcat
- **Issues:** https://github.com/joshainglis/topcat/issues
- **Documentation:** See [CLAUDE.md](CLAUDE.md) for detailed project documentation
