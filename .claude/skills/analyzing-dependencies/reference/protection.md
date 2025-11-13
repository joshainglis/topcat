# Root Node Protection Patterns

## Overview

Root node protection prevents critical entry points from being marked as dead branches during analysis and cleanup operations.

## Why Protection is Needed

Without external usage checking, the dead branches algorithm correctly identifies all unreferenced nodes as dead. However, certain files ARE entry points (API handlers, migrations, CLI commands) that should never be deleted, even if nothing in the dependency graph depends on them.

## Four Protection Methods

### 1. Exact Node Names

Protect specific nodes by their exact name:

```bash
topcat analyze -i sql/ -e sql \
  --root-nodes api_main \
  --root-nodes worker_main \
  --root-nodes migration_001 \
  dead-branches
```

**Use cases**:
- Known critical entry points
- Specific files that must never be deleted
- When you have a short list of protected files

**Example config**:
```toml
[analysis]
root_nodes = ["api_main", "worker_main", "cli_entry"]
```

### 2. Glob Patterns

Protect files matching glob patterns:

```bash
topcat analyze -i sql/ -e sql \
  --root-pattern "**/api/*.sql" \
  --root-pattern "**/migrations/*.sql" \
  --root-pattern "**/workers/*.sql" \
  dead-branches
```

**Pattern syntax**:
- `*` - matches any characters except `/`
- `**` - matches any characters including `/`
- `?` - matches single character
- `[abc]` - matches one of the characters
- `{a,b}` - matches either pattern

**Use cases**:
- Protecting entire directories
- Files following naming conventions
- When entry points are organized in specific folders

**Example config**:
```toml
[analysis]
root_patterns = [
    "**/api/*.sql",
    "**/migrations/*.sql",
    "**/workers/*.sql",
    "**/*_entry_point.sql"
]
```

### 3. Regex Patterns

Protect nodes matching regular expressions:

```bash
topcat analyze -i sql/ -e sql \
  --root-regex "^api_.*" \
  --root-regex "^worker_.*" \
  --root-regex "^migration_\\d{3}$" \
  dead-branches
```

**Use cases**:
- Complex naming patterns
- Prefix/suffix conventions (e.g., all nodes starting with "api_")
- When glob patterns aren't expressive enough

**Example config**:
```toml
[analysis]
root_regex = [
    "^api_.*",           # All nodes starting with "api_"
    "^worker_.*",        # All nodes starting with "worker_"
    "^migration_\\d{3}$", # migration_001, migration_002, etc.
    ".*_entry_point$"    # All nodes ending with "_entry_point"
]
```

### 4. Directory-Based

Protect all files in specific directories:

```bash
topcat analyze -i sql/ -e sql \
  --root-dir sql/entry_points/ \
  --root-dir sql/migrations/ \
  --root-dir sql/api/ \
  dead-branches
```

**Use cases**:
- When all files in a directory are entry points
- Organized project structure with clear entry point directories
- Simplest pattern when directories are well-organized

**Example config**:
```toml
[analysis]
root_dirs = [
    "sql/entry_points/",
    "sql/migrations/",
    "sql/api/"
]
```

## Combining Protection Methods

All four methods can be used together:

```bash
topcat analyze -i sql/ -e sql \
  --root-nodes critical_function \
  --root-pattern "**/api/*.sql" \
  --root-regex "^migration_.*" \
  --root-dir sql/entry_points/ \
  dead-branches
```

**Config file example**:
```toml
[analysis]
# Protect specific critical files
root_nodes = ["database_setup", "schema_init"]

# Protect entire API and worker directories
root_patterns = [
    "**/api/*.sql",
    "**/workers/*.sql"
]

# Protect all migrations and CLI commands by naming convention
root_regex = [
    "^migration_\\d{3}",
    "^cli_.*"
]

# Protect entire entry points directory
root_dirs = ["sql/entry_points/"]
```

## How Matching Works

For each node in the graph, Topcat checks if it matches ANY of the protection patterns:

1. Check if node name is in `root_nodes` (exact match)
2. Check if file path matches any `root_patterns` (glob)
3. Check if node name matches any `root_regex` (regex)
4. Check if file path is within any `root_dirs` (directory)

If ANY match is found, the node is marked as a root node and protected.

## Config File vs CLI

**Config file approach** (recommended for production):
```toml
# topcat.toml
[analysis]
root_patterns = ["**/api/*.sql", "**/workers/*.sql"]
```

```bash
topcat analyze -i sql/ -e sql --sql-config topcat.toml dead-branches
```

**Benefits**:
- Version control the protection rules
- Share configuration across team
- Consistent protection across runs
- Less typing

**CLI approach** (for ad-hoc testing):
```bash
topcat analyze -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches
```

**Benefits**:
- Quick testing and experimentation
- Override config file rules
- One-off analysis

## Real-World Examples

### Example 1: API Service with Migrations

```toml
[analysis]
# API endpoints are entry points
root_patterns = [
    "**/api/handlers/*.sql",
    "**/api/endpoints/*.sql"
]

# All migrations must be preserved
root_regex = ["^v\\d+_.*"]

# Background workers
root_dirs = ["sql/workers/"]
```

### Example 2: Multi-Schema Database

```toml
[analysis]
# Public API functions
root_patterns = [
    "public_api/**/*.sql",
    "*/public_functions.sql"
]

# Schema initialization files
root_nodes = [
    "auth.schema_init",
    "billing.schema_init",
    "analytics.schema_init"
]

# Migration directories per schema
root_dirs = [
    "migrations/auth/",
    "migrations/billing/",
    "migrations/analytics/"
]
```

### Example 3: Microservice Architecture

```toml
[analysis]
# Service entry points by naming convention
root_regex = [
    "^service_.*_main$",
    "^worker_.*_entry$"
]

# Shared schemas that multiple services depend on
root_nodes = [
    "shared.events_schema",
    "shared.common_types"
]

# API gateway definitions
root_patterns = ["gateway/routes/*.sql"]
```

## Combining with External Usage Checking

For maximum accuracy, combine root node protection with external usage checking:

```bash
topcat analyze -i sql/ -e sql \
  --root-pattern "**/api/*.sql" \
  --external-check-dir src/ \
  --external-check-pattern "*.py" \
  dead-branches
```

This approach:
- Root nodes protect known entry points
- External checking catches unknown external references
- Minimizes false positives

## Testing Protection Patterns

Test your protection patterns before cleanup:

```bash
# 1. Run analysis without protection to see what would be marked dead
topcat analyze -i sql/ -e sql dead-branches

# 2. Add protection and verify critical files are excluded
topcat analyze -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

# 3. Use dry-run cleanup to preview
topcat clean -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

# 4. Only then use --no-dry-run
topcat clean -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches --no-dry-run
```

## Common Patterns by Use Case

| Use Case | Recommended Method | Example |
|----------|-------------------|---------|
| API endpoints | Glob patterns | `**/api/*.sql` |
| Database migrations | Regex | `^v\\d+_.*` or `^migration_\\d{3}$` |
| Entry point directory | Directory-based | `sql/entry_points/` |
| Specific critical files | Exact names | `root_nodes = ["init"]` |
| Background workers | Glob + Regex | `**/workers/*.sql` + `^worker_.*` |
| CLI commands | Regex | `^cli_.*` |
| Schema initializers | Exact names | Schema-qualified names |

## Troubleshooting

### Protected file still showing as dead

1. Check pattern syntax (glob vs regex)
2. Verify file path matches pattern
3. Use verbose mode to see matching: `-v`
4. Check if pattern should match node name or file path

### Too many files protected

1. Make patterns more specific
2. Use exact names instead of patterns
3. Check for overly broad regex (e.g., `.*` matches everything)

### Config file not applying

1. Verify config file path: `--sql-config topcat.toml`
2. Check TOML syntax is valid
3. Ensure `[analysis]` section exists
4. CLI flags override config file
