---
name: discovering-sql-dependencies
description: Analyzes SQL files to automatically extract and manage dependencies, eliminating manual header maintenance. Use when working with SQL dependency discovery, configuring automatic extraction patterns, or migrating from manual to automatic dependency management in Topcat.
---

# Discovering SQL Dependencies

## Quick Start

Enable automatic dependency discovery to eliminate manual header maintenance:

```bash
# Basic discovery with schema pattern
topcat -i sql/ -o output.sql --enable-sql-discovery --schema-pattern "myapp_\\w+"

# Using configuration file
topcat -i sql/ -o output.sql --sql-config topcat.toml

# Update headers in-place
topcat -i sql/ -o output.sql --enable-sql-discovery --update-headers
```

## How Discovery Works

Topcat analyzes SQL content to extract dependencies from:

- CREATE/ALTER/DROP statements
- Schema-qualified references (schema.table)
- Function and procedure calls
- Type references and casts
- Model generation patterns

## Configuration

### Quick CLI Setup

```bash
--enable-sql-discovery           # Enable discovery
--schema-pattern "pattern"       # Schema name pattern
--merge-strategy strategy        # How to merge dependencies
--update-headers                 # Update files in-place
```

### TOML Configuration

Create `topcat.toml` for complex setups:

```toml
[sql_discovery]
enabled = true
schema_pattern = "(?:app|test)_\\w+"
merge_strategy = "discovery-only"

[[sql_discovery.type_mappings]]
from = "JSONB"
to = "pg_catalog.jsonb"
```

For detailed configuration options, see [reference/config.md](reference/config.md).

## Merge Strategies

Choose how discovered dependencies combine with manual ones:

- **discovery-only** (default): Replace manual with discovered
- **union**: Combine both manual and discovered
- **header-only**: Use only manual headers
- **validate**: Check for mismatches
- **header-with-fallback**: Manual if present, otherwise discovered

## Migration Workflow

Migrate existing manual dependencies to automatic discovery:

```
Migration Checklist:
- [ ] Step 1: Validate current setup with --merge-strategy validate
- [ ] Step 2: Test discovery on subset with --subdir-filter
- [ ] Step 3: Review discovered dependencies with --dry -v
- [ ] Step 4: Switch to discovery-only strategy
- [ ] Step 5: Update headers with --update-headers or --generate-headers
```

For detailed migration guide, see [reference/migration.md](reference/migration.md).

## Override Dependencies

Force retention of specific dependencies with `!` prefix:

```sql
-- name: my_function
-- requires: !pg_extension, discovered_dep
```

The `!pg_extension` will always be included even if not discovered.

## Advanced Topics

**Pattern configuration**: See [reference/patterns.md](reference/patterns.md)
**Type and extension mappings**: See [reference/config.md](reference/config.md)
**Troubleshooting discovery**: See [reference/troubleshooting.md](reference/troubleshooting.md)

## Common Use Cases

### PostgreSQL Extensions

```toml
[[sql_discovery.extension_mappings]]
object = "uuid_generate_v4"
extension = "uuid-ossp"
```

### Multiple Schemas

```toml
schema_pattern = "(?:public|app|auth|billing)_\\w+"
```

### Type Mappings

```toml
[[sql_discovery.type_mappings]]
from = "TSTZRANGE"
to = "temporal_types.time_range"
```