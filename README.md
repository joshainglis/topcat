# Topcat

**Top**ological con**cat**enation of files with comprehensive dependency analysis.

## Overview

Topcat is a Rust CLI tool that reads files with dependency metadata, builds a directed acyclic graph (DAG), performs topological sorting with layer constraints, and provides powerful analysis, cleanup, and export capabilities.

**Primary use case:** Managing SQL migration files where execution order matters based on dependencies. Also analyzes dependency health, detects dead code, and safely cleans up unused files.

## Features

- 🔗 **Topological Concatenation** - Order files correctly based on dependencies
- 📝 **Header Management** - Update file headers with discovered dependencies and rename files
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
# Update file headers with discovered SQL dependencies (smart defaults for .sql files)
topcat update -i sql/ -e sql --mode execute

# Concatenate SQL files in dependency order
topcat concat -i sql/ -e sql migrations.sql

# Find dead code that can be safely removed
topcat analyze -i sql/ -e sql dead-branches

# Preview deletion of orphaned files
topcat clean -i sql/ -e sql orphans

# List all schemas with statistics
topcat schema -i sql/ -e sql list

# Export dependency graph for visualization
topcat export -i sql/ -e sql graph.json json

# Import a pg_dump and split into per-object files
topcat import pg-dump database.sql ./output/
```

## Commands

### `concat` - Concatenate Files

Concatenate files in topological order respecting dependencies and layer constraints.

```bash
topcat concat -i sql/ -e sql output.sql
topcat concat -i dir1/ -i dir2/ -e sql output.sql --layers prepend,normal,append
```

**Arguments:**
- `<OUTPUT>` - Output file path (positional)

**Basic Options:**
- `-i, --input-dirs <DIR>...` - Input directories (multiple allowed)
- `-e, --include-exts <EXT>...` - File extensions to include (e.g., `sql`)
- `-E, --exclude-exts <EXT>...` - File extensions to exclude
- `-g, --include-glob <PATTERN>...` - Include files matching glob
- `-G, --exclude-glob <PATTERN>...` - Exclude files matching glob
- `-v, --verbose` - Show debug information

**Layer Options:**
- `--layers <LAYERS>` - Custom layer ordering (comma-separated, default: `prepend,normal,append`)
- `--fallback-layer <LAYER>` - Default layer for files without declaration (default: `normal`)

**Filtering Options:**
- `--include-prefix <PREFIX>...` - Only include nodes with these name prefixes
- `--exclude-prefix <PREFIX>...` - Exclude nodes with these name prefixes
- `--subdir-filter <PATH>` - Include only files from subdirectory and their dependencies
- `--schema <SCHEMA>...` - Filter to specific schemas

**Formatting Options:**
- `-c, --comment-prefix <STR>` - Comment string (default: `--`)
- `-s, --file-separator <STR>` - Separator between concatenated files
- `-a, --file-suffix <STR>` - Ensure files end with this (default: `;`)

### `update` - Update File Headers

Discover dependencies from SQL content and update file headers accordingly.

**Smart Defaults:** When using SQL extensions (`-e sql`, `-e pg`, etc.), SQL discovery, header updates, and file renaming are all enabled by default.

```bash
# Preview changes (dry-run is default)
topcat update -i sql/ -e sql

# Apply changes
topcat update -i sql/ -e sql --mode execute

# Generate updated files to a separate directory (instead of in-place)
topcat update -i sql/ -e sql --generate-headers ./updated/

# Disable specific defaults
topcat update -i sql/ -e sql --no-rename-files         # Keep original filenames
topcat update -i sql/ -e sql --no-sql-discovery        # Header-only parsing
```

**Options:**
- `--update-headers` / `--no-update-headers` - Update source files in-place (default: enabled for SQL)
- `--generate-headers <DIR>` - Write files with updated headers to a separate directory
- `--rename-files` / `--no-rename-files` - Rename files based on discovered node names (default: enabled for SQL)
- `--sql-discovery` / `--no-sql-discovery` - Extract dependencies from SQL code (default: enabled for SQL)
- `--schema-pattern <REGEX>` - Pattern for schema names (e.g., `"myapp_\\w+"`)
- `--merge-strategy <STRATEGY>` - How to merge manual vs discovered dependencies
- `--mode <MODE>` - `dry-run` (default) or `execute`

**Typical Workflow:**

```bash
# Step 1: Preview changes
topcat update -i sql/ -e sql

# Step 2: Apply changes
topcat update -i sql/ -e sql --mode execute

# Step 3: Concatenate the properly annotated files
topcat concat -i sql/ -e sql migrations.sql
```

**Note:** The `concat` command no longer supports header updates or SQL discovery. Use `topcat update` for header management instead.

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
topcat clean -i sql/ -e sql dead-branches              # Preview (dry-run default)
topcat clean -i sql/ -e sql orphans --mode execute     # Execute with confirmation
topcat clean -i sql/ -e sql orphans --mode execute -f  # Force (no confirmation)
```

**Clean Types:**
- `dead-branches` - Remove complete dead subtrees
- `orphans` - Remove isolated files
- `unrequired` - Remove unrequired files
- `targets <files>...` - Remove specific targets (with dependency check)

**Safety Features:**
- `--mode dry-run` - Preview deletion (**DEFAULT** - always safe by default)
- `--mode execute` - Actually perform deletion
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
topcat export -i sql/ -e sql graph.json json                       # Full graph
topcat export -i sql/ -e sql graph.dot dot                         # GraphViz
topcat export -i sql/ -e sql deps.json json --mode deps --node my_node
```

**Arguments:**
- `<OUTPUT>` - Output file path (positional)
- `<FORMAT>` - Export format subcommand: `json`, `dot`, `graphml`, `mermaid`

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
- `--node <NAME>` - Target node (required for deps/dependents/direct modes)
- `--schema <SCHEMA>...` - Filter by schemas

### `import` - Import External Sources

Import database dumps and other sources into organized file structures with topcat headers.

```bash
topcat import pg-dump database.sql ./output/                   # Split pg_dump file
topcat import pg-dump database.sql ./output/ --dry-run         # Preview changes
topcat import pg-dump database.sql ./output/ --schema-pattern "app_\\w+"
```

**Subcommands:**
- `pg-dump` - Import PostgreSQL pg_dump file and split into per-object files

**Arguments:**
- `<DUMP_FILE>` - Path to the pg_dump SQL file to import
- `<OUTPUT_DIR>` - Output directory for the split SQL files

**Options:**
- `--schema-pattern <PATTERN>` - Regex pattern for matching schema names (for CAST/OPERATOR parsing)
- `--dry-run` - Preview changes without writing files

**Output Structure:**

```
output_dir/
├── _global/                    # Database-level objects
│   ├── cast/                   # CAST definitions
│   └── operator/               # OPERATOR definitions
└── schema_name/
    ├── schema_name.sql         # SCHEMA definition
    ├── table/
    │   └── table_name.sql      # TABLE + constraints, indexes
    ├── functions/
    │   └── func_name.sql       # FUNCTION definitions
    └── type/
        ├── enum/
        │   └── status.sql      # ENUM types
        ├── composite/
        │   └── address.sql     # Composite types
        └── domain/
            └── email.sql       # DOMAIN types
```

Each file includes a topcat-compatible header (`-- name: schema.object_name`) for dependency tracking.

### `config` - Configuration Management

Manage and inspect Topcat configuration from multiple sources.

```bash
topcat config show                          # Display effective configuration
topcat config validate                      # Validate config file
topcat config generate                      # Generate example config
topcat config generate > topcat.toml        # Save example to file
```

**Configuration Operations:**
- `show` - Display the effective configuration from all sources (CLI, env vars, config files, defaults) with precedence information
- `validate` - Validate configuration file syntax and settings, with helpful error messages
- `generate` - Generate a comprehensive example `topcat.toml` with all available options and inline documentation

**Configuration Sources (Precedence Order):**
1. CLI arguments (highest priority)
2. Environment variables (`TOPCAT_*`)
3. Project config (`./topcat.toml`)
4. User config (`~/.config/topcat/config.toml`)
5. System config (`/etc/topcat/config.toml`)
6. Default values (lowest priority)

**Options:**
- `--config <PATH>` - Specify custom config file path (overrides default discovery)

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
topcat concat -i sql/ -e sql output.sql --layers setup,functions,views,cleanup
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

Topcat uses a unified configuration system that loads settings from multiple sources with clear precedence rules.

### Configuration Precedence

Settings are merged from multiple sources in this order (highest to lowest priority):

1. **CLI arguments** - Command-line flags always take precedence
2. **Environment variables** - `TOPCAT_*` variables
3. **Project config** - `./topcat.toml` or `./.topcat.toml`
4. **User config** - `~/.config/topcat/config.toml`
5. **System config** - `/etc/topcat/config.toml`
6. **Default values** - Built-in defaults

This allows you to set baseline configuration in files and override specific settings via environment variables or CLI flags as needed.

### Configuration File

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

# Convert hard deps to soft deps for specific patterns (config-only)
[sql_discovery.soft_deps_mappings]
"^codegen_tmf\\b" = "^c_tmf\\b"

[analysis]
# Protect these nodes from dead branch detection
root_nodes = ["api_main", "public_entry"]
root_patterns = ["**/api/*.sql", "**/public/*.sql"]
root_regex = ["^api_.*", "^public_.*"]
root_dirs = ["api/", "migrations/"]

# Check for external usage
external_check_dirs = ["src/", "app/"]
external_check_patterns = ["*.py", "*.rs", "*.ts"]

# Protect implicit nodes (CAST, OPERATOR) from cleanup
protect_implicit = true

[layers]
names = ["prepend", "normal", "append"]
fallback = "normal"

# Auto-assign layers based on node name patterns (config-only)
[layers.auto_mapping]
"^\\w+\\.grants$" = "append"
"^\\w+\\.schema$" = "prepend"

[filters]
include_extensions = ["sql"]
exclude_globs = ["**/*.backup.sql", "**/*.old.sql"]

[behavior]
verbose = false
quiet = false
dry_run = false
```

**Generate example config:**
```bash
topcat config generate > topcat.toml
```

### Environment Variables

All configuration options can be set via environment variables using the `TOPCAT_` prefix. This is particularly useful for CI/CD pipelines and containerized environments.

#### Variable Naming Convention

- Prefix: `TOPCAT_`
- Nested sections: Use double underscore `__` (e.g., `TOPCAT_SQL_DISCOVERY__ENABLED`)
- Arrays: Comma-separated values (e.g., `"value1,value2,value3"`)
- Booleans: `true` or `false` (case-insensitive)

#### Basic Settings

```bash
# Verbose output
export TOPCAT_VERBOSE=true

# Quiet mode (suppress non-error output)
export TOPCAT_QUIET=true

# Dry-run mode
export TOPCAT_BEHAVIOR__DRY_RUN=true

# Force mode (skip confirmations)
export TOPCAT_BEHAVIOR__FORCE=false

# Input/output paths
export TOPCAT_INPUT_DIRS="/path/to/sql,/path/to/more/sql"
export TOPCAT_OUTPUT="/path/to/output.sql"
```

#### File Filters

```bash
# Include specific file extensions
export TOPCAT_FILTERS__INCLUDE_EXTENSIONS="sql,ddl"

# Exclude specific extensions
export TOPCAT_FILTERS__EXCLUDE_EXTENSIONS="backup,tmp"

# Include glob patterns
export TOPCAT_FILTERS__INCLUDE_GLOBS="**/*.sql,**/*.ddl"

# Exclude glob patterns
export TOPCAT_FILTERS__EXCLUDE_GLOBS="**/*.backup.sql,**/temp/**"

# Include hidden files
export TOPCAT_FILTERS__INCLUDE_HIDDEN=true
```

#### Node Filtering

```bash
# Prefix filters for node names
export TOPCAT_NODE_FILTERING__INCLUDE_PREFIXES="api_,public_"
export TOPCAT_NODE_FILTERING__EXCLUDE_PREFIXES="test_,deprecated_"

# Subdirectory filter
export TOPCAT_NODE_FILTERING__SUBDIR_FILTER="sql/auth"
```

#### Layer Configuration

```bash
# Define layer ordering
export TOPCAT_LAYERS__NAMES="prepend,normal,append"

# Fallback layer for files without declaration
export TOPCAT_LAYERS__FALLBACK="normal"
```

#### SQL Discovery

```bash
# Enable SQL dependency discovery
export TOPCAT_SQL_DISCOVERY__ENABLED=true

# Schema name pattern (regex)
export TOPCAT_SQL_DISCOVERY__SCHEMA_PATTERN="myapp_\\w+"

# Object name pattern (regex)
export TOPCAT_SQL_DISCOVERY__OBJECT_PATTERN="\\w+"

# Merge strategy
export TOPCAT_SQL_DISCOVERY__MERGE_STRATEGY="discovery-only"

# Type mappings (JSON format)
export TOPCAT_SQL_DISCOVERY__TYPE_MAPPINGS='{"TSTZRANGE":"c_tmf.t_time_period"}'

# Extension mappings (JSON format)
export TOPCAT_SQL_DISCOVERY__EXTENSION_MAPPINGS='{"digest":"pgcrypto"}'

# Strip suffixes
export TOPCAT_SQL_DISCOVERY__STRIP_SUFFIXES="_or_ref,_view"

# Model generation patterns
export TOPCAT_SQL_DISCOVERY__MODEL_GEN_PATTERNS="codegen_tmf\\.proc_(?:make_model|combine_enums)"
```

#### Analysis Configuration

```bash
# Root node protection
export TOPCAT_ANALYSIS__ROOT_NODES="api_main,public_entry"
export TOPCAT_ANALYSIS__ROOT_PATTERNS="**/api/*.sql,**/migrations/*.sql"
export TOPCAT_ANALYSIS__ROOT_REGEX="^api_.*,^public_.*"
export TOPCAT_ANALYSIS__ROOT_DIRS="api/,migrations/"

# External usage checking
export TOPCAT_ANALYSIS__EXTERNAL_CHECK_DIRS="/app/src,/app/lib"
export TOPCAT_ANALYSIS__EXTERNAL_CHECK_PATTERNS="*.py,*.ts,*.rs"

# Protect implicit nodes (CAST, OPERATOR)
export TOPCAT_ANALYSIS__PROTECT_IMPLICIT=true
```

#### Formatting

```bash
# Comment prefix for headers
export TOPCAT_FORMATTING__COMMENT_STR="--"

# File separator in concatenated output
export TOPCAT_FORMATTING__FILE_SEPARATOR_STR="---"

# String appended at end of each file
export TOPCAT_FORMATTING__FILE_END_STR=";"
```

#### Schema Filtering

```bash
# Filter operations to specific schemas
export TOPCAT_SCHEMA_FILTERING__SCHEMAS="auth,billing,analytics"
```

#### Export Configuration

```bash
# Export mode (full, deps, dependents, direct)
export TOPCAT_EXPORT__MODE="full"

# Target node (required for deps/dependents/direct modes)
export TOPCAT_EXPORT__NODE="api_main"
```

### CI/CD Pipeline Example

Example GitHub Actions workflow using environment variables:

```yaml
name: SQL Dependency Check

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    env:
      TOPCAT_VERBOSE: false
      TOPCAT_QUIET: true
      TOPCAT_FILTERS__INCLUDE_EXTENSIONS: sql
      TOPCAT_ANALYSIS__ROOT_PATTERNS: "**/api/*.sql,**/migrations/*.sql"
      TOPCAT_ANALYSIS__EXTERNAL_CHECK_DIRS: "src/,app/"
      TOPCAT_ANALYSIS__EXTERNAL_CHECK_PATTERNS: "*.py,*.ts"

    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo install topcat

      # Fail on cycles
      - name: Check for circular dependencies
        run: topcat analyze -i sql/ -e sql cycles

      # Fail on missing dependencies
      - name: Check for missing dependencies
        run: topcat analyze -i sql/ -e sql missing

      # Report dead branches (informational)
      - name: Find dead code
        run: topcat analyze -i sql/ -e sql dead-branches
```

### Configuration Management Commands

```bash
# View effective configuration from all sources
topcat config show

# Validate configuration file
topcat config validate

# Generate example configuration file
topcat config generate > topcat.toml
```

### SQL Discovery

Automatically extract dependencies from SQL code, eliminating manual header maintenance. SQL discovery is enabled by default for SQL files in the `update` command:

```bash
# Smart defaults: discovery, header updates, and file renaming all enabled
topcat update -i sql/ -e sql --mode execute

# With custom schema pattern
topcat update -i sql/ -e sql --schema-pattern "myapp_\\w+" --mode execute
```

**Discovery Features:**
- Extracts table, view, function, type, and extension dependencies
- Configurable schema and object patterns
- Type and extension mappings for system objects
- Multiple merge strategies for combining with manual headers

**Extension Mapping Example:**

When SQL uses extension functions, map them to their providing extensions:

```toml
[sql_discovery]
extension_mappings = { "nlevel" = "ltree", "lca" = "ltree", "digest" = "pgcrypto" }
```

Now when your SQL contains `SELECT e_extensions.nlevel(path)`, Topcat automatically creates a dependency on `e_extensions.ltree` instead of the non-existent `e_extensions.nlevel`.

**File Renaming:**

File renaming is enabled by default for SQL files. Files are renamed based on discovered node names:

```bash
# Renaming enabled by default - just run update
topcat update -i sql/ -e sql --mode execute

# Disable renaming if you want to keep original filenames
topcat update -i sql/ -e sql --no-rename-files --mode execute

# Generate renamed files in a new directory
topcat update -i sql/ -e sql --generate-headers ./updated
```

For schema.object nodes, files are renamed to `object.ext` (e.g., `my_schema.users` → `users.sql`). For schema-only nodes, files keep the schema name (e.g., `my_schema` → `my_schema.sql`).

See configuration section above for detailed `sql_discovery` options.

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
topcat concat -i sql/ -e sql migrations/deploy.sql
```

**Result:** Files ordered as `schema.sql` → `user_auth.sql` → `user_profile.sql` → `active_users.sql`

### Dead Code Cleanup Workflow

```bash
# Step 1: Analyze and find dead branches
topcat analyze -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

# Step 2: Preview deletion (dry-run is default)
topcat clean -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

# Step 3: Execute deletion with confirmation
topcat clean -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches --mode execute

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
topcat concat -i sql/ -e sql auth.sql --include-prefix auth.
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
topcat export -i sql/ -e sql graph.dot dot
dot -Tpng graph.dot -o graph.png

# Export to Mermaid for documentation
topcat export -i sql/ -e sql graph.md mermaid

# Export dependencies of specific node
topcat export -i sql/ -e sql api_deps.json json --mode deps --node api_main
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
# Smart defaults handle discovery, header updates, and renaming
topcat update -i sql/ -e sql --mode execute

# With custom schema pattern
topcat update -i sql/ -e sql --schema-pattern "myapp_\\w+" --mode execute

# Validate your manual headers match reality
topcat update -i sql/ -e sql --merge-strategy validate
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
topcat clean -i sql/ -e sql dead-branches --mode execute
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
