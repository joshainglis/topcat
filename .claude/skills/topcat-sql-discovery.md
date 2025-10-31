# SQL Dependency Discovery Skill

This skill provides comprehensive guidance for using Topcat's automatic SQL dependency discovery feature.

## Overview

Topcat can automatically discover dependencies from SQL content, eliminating the need for manual header maintenance. The analyzer extracts dependencies from DDL statements, DML references, function calls, type references, and model generation patterns.

## Module Structure

- **sql_config.rs** - Configuration for SQL discovery (patterns, mappings, merge strategies)
- **sql_parser.rs** - `SqlAnalyzer` that extracts dependencies from SQL content using regex patterns
- **header_generator.rs** - Generates/updates file headers with discovered dependencies

## Automatic Dependency Extraction

The SQL analyzer discovers dependencies from:

### DDL Statements
- Extracts object names from CREATE/ALTER/DROP statements
- Handles tables, views, functions, procedures, types

### DML References
- Finds schema.object references in SELECT, JOIN, INSERT, UPDATE, DELETE
- Recognizes qualified names (schema.table format)

### Function Calls
- Detects function and procedure dependencies
- Handles both qualified and unqualified calls

### Type References
- Handles custom types and casts (e.g., `::TSTZRANGE`)
- Maps types to their definitions

### Model Generation
- Special patterns for code generation procedures
- Recognizes model building function calls

## Configuration

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

### TOML Configuration File

Create a `topcat.toml` file for complex configurations:

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

## Merge Strategies

Control how discovered dependencies combine with manual ones:

### discovery-only (default)
Replace manual dependencies with discovered ones. Best for new projects or when fully migrating to automatic discovery.

### header-only
Ignore discovered dependencies, use only manual headers. Useful when discovery is temporarily broken or during debugging.

### union
Combine both manual and discovered dependencies. Good for gradual migration or when some dependencies can't be discovered automatically.

### header-with-fallback
Use manual dependencies if present, otherwise use discovered. Allows selective manual overrides while using discovery for most files.

### validate
Check for mismatches between manual and discovered dependencies. Useful for verifying that discovery is working correctly before switching strategies.

## Override Mechanism

Use `!` prefix to force a dependency to be kept regardless of discovery:

```sql
-- name: my_table
-- requires: !special_dep, other_dep
```

The `!special_dep` will always be included even if not found in the SQL content. This is useful for:
- External dependencies (e.g., extensions)
- Dependencies that can't be discovered automatically
- Temporary workarounds for discovery limitations

## Header Generation

### In-place Update

Update existing files directly:

```bash
cargo run -- -i sql/ -o output.sql --enable-sql-discovery --update-headers
```

**Warning**: This modifies your source files. Ensure you have backups or version control.

### Generate to Directory

Create updated copies in a new directory:

```bash
cargo run -- -i sql/ -o output.sql --enable-sql-discovery --generate-headers /tmp/updated_sql
```

This is safer as it preserves original files.

## Usage Examples

### Basic Discovery

```bash
# Simple schema pattern
cargo run -- -i sql/ -o output.sql --enable-sql-discovery --schema-pattern "myschema_\\w+"
```

### With Configuration File

```bash
# Using TOML config
cargo run -- -i sql/ -o output.sql --sql-config topcat.toml
```

### Validation Mode

```bash
# Check manual vs discovered
cargo run -- -i sql/ -o output.sql --sql-config topcat.toml --merge-strategy validate
```

## Implementation Details

### Discovery Workflow

1. `file_dag.rs:build_graph()` creates `SqlAnalyzer` if discovery enabled
2. For each file, `perform_sql_discovery()` reads content and calls `analyzer.analyze()`
3. Results stored in `FileNode.discovered_deps`
4. `FileNode.merge_dependencies()` merges based on strategy
5. Graph validation proceeds as normal

### Pattern Matching

The analyzer uses configurable regex patterns:

- **Schema Pattern**: Matches schema names (e.g., `c_\\w+`, `test_\\w+`)
- **Dependency Pattern**: Matches `schema.object` references
- **Model Generation**: Patterns for procedure calls
- **Type Cast**: Patterns for `::TYPE` syntax

### Transformations

Apply transformations to normalize object names:

- **Strip Suffixes**: Remove common suffixes (e.g., `_or_ref`)
- **Map to Extensions**: Convert function names to extensions (e.g., `digest` → `pgcrypto`)
- **Custom Type Mappings**: Map type casts to actual type definitions

## Migration Path

For existing projects with manual dependencies:

### Step 1: Validate Current Setup

```bash
topcat --sql-config config.toml --merge-strategy validate
```

Review warnings about mismatches between manual and discovered dependencies.

### Step 2: Test Discovery

```bash
# Test on a subset first
topcat -i sql/subset/ -o test.sql --sql-config config.toml --merge-strategy discovery-only
```

### Step 3: Switch Strategies

Once confident, switch to your chosen strategy:

```bash
topcat --sql-config config.toml --merge-strategy discovery-only
```

### Step 4: Update Headers

Remove manual dependencies once discovery is working:

```bash
# First generate to a temp directory to review
topcat --sql-config config.toml --generate-headers /tmp/review

# Then update in-place if satisfied
topcat --sql-config config.toml --update-headers
```

## Testing

SQL discovery features have comprehensive test coverage:

```bash
# Run SQL-specific tests
cargo test sql_

# Test specific module
cargo test sql_parser
cargo test sql_config
cargo test header_generator

# Run with output for debugging
cargo test sql_ -- --nocapture
```

## Best Practices

1. **Start Simple**: Use CLI arguments for basic patterns before moving to TOML
2. **Test Incrementally**: Validate on a subset before processing entire codebase
3. **Use Version Control**: Commit before updating headers in-place
4. **Document Overrides**: Comment why dependencies use `!` prefix
5. **Monitor Warnings**: Pay attention to validation mode output
6. **Keep Config in Repo**: Version control your `topcat.toml` file

## Troubleshooting

### Dependencies Not Found

- Check schema pattern matches your naming convention
- Verify dependency is actually referenced in SQL
- Use `--dry` with `-v` to see discovery debug output
- Consider using override prefix `!` for external dependencies

### Circular Dependencies

- Discovery respects existing validation rules
- Check if manual headers had incorrect dependencies
- Use layers to break circular dependencies

### Performance Issues

- Large files may take time to analyze
- Consider using `--subdir-filter` to process subsets
- Disable discovery with `--merge-strategy header-only` if needed

## Common Patterns

### PostgreSQL Extensions

```toml
[[sql_discovery.extension_mappings]]
object = "digest"
extension = "pgcrypto"

[[sql_discovery.extension_mappings]]
object = "uuid_generate_v4"
extension = "uuid-ossp"
```

### Custom Types

```toml
[[sql_discovery.type_mappings]]
from = "JSONB"
to = "pg_catalog.jsonb"

[[sql_discovery.type_mappings]]
from = "TSTZRANGE"
to = "c_tmf.t_time_period"
```

### Schema Patterns

```toml
# Multiple schemas
schema_pattern = "(?:public|app|test)_\\w+"

# Prefix-based
schema_pattern = "myapp_\\w+"

# Complex patterns
schema_pattern = "(?:c|d[pio]|codegen)_\\w+"
```