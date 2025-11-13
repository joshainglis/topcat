---
name: managing-schemas
description: Guides schema-based organization and analysis for multi-schema database projects. Use when working with multiple schemas, analyzing cross-schema dependencies, or filtering operations by schema.
---

# Managing Schemas

## When to Use This Skill

- Working with multi-schema database projects
- Analyzing cross-schema dependencies
- Filtering analysis/cleanup to specific schemas
- Understanding schema distribution and coupling
- Organizing large SQL codebases by schema

## Quick Command Reference

```bash
# Schema operations
topcat schema -i sql/ -e sql list                    # View all schemas
topcat schema -i sql/ -e sql analyze my_schema       # Detailed schema view
topcat schema -i sql/ -e sql dependencies            # Cross-schema deps

# Schema filtering
topcat analyze -i sql/ -e sql --schema my_schema dead-branches
topcat clean -i sql/ -e sql --schema billing orphans --no-dry-run
topcat export -i sql/ -e sql --schema auth -o auth.json json
```

## Schema Extraction

Topcat automatically extracts schemas from node names:

**Supported patterns**:
- Dot notation: `schema.table` → schema: "schema"
- Double colon: `schema::table` → schema: "schema"
- No separator: `mytable` → schema: (none)

**Example**:
```sql
-- name: auth.users_table
-- Creates node "auth.users_table" with schema "auth"

-- name: billing::invoices
-- Creates node "billing::invoices" with schema "billing"

-- name: standalone_function
-- Creates node with no schema
```

No configuration needed - extraction is automatic!

## Schema Commands

### List Schemas

View all schemas with statistics and distribution:

```bash
topcat schema -i sql/ -e sql list
```

**Output**:
```
╭─────────────┬───────┬──────────────┬─────────────────────────────────╮
│ Schema      ┆ Files ┆ Dependencies ┆ Distribution                    │
╞═════════════╪═══════╪══════════════╪═════════════════════════════════╡
│ auth        ┆ 5     ┆ 12           ┆ ████████████████████░░░░░░░░░░░ │
│ billing     ┆ 3     ┆ 8            ┆ ████████████░░░░░░░░░░░░░░░░░░░ │
│ public      ┆ 2     ┆ 4            ┆ ████████░░░░░░░░░░░░░░░░░░░░░░░ │
╰─────────────┴───────┴──────────────┴─────────────────────────────────╯
```

Shows:
- File count per schema
- Dependency count
- Visual bar chart

### Analyze Schema

Detailed view of a specific schema:

```bash
topcat schema -i sql/ -e sql analyze auth
```

**Output**:
```
Schema: auth

Files (5):
  - auth.users_table
  - auth.sessions
  - auth.permissions

Internal Dependencies (3):
  auth.sessions → auth.users_table
  auth.permissions → auth.users_table
  auth.audit_log → auth.sessions

External Dependencies (2):

  To schema 'public':
    auth.users_table → public.extensions

  To schema 'billing':
    auth.users_table → billing.customers

Dependent Schemas (1):
  billing
```

Shows:
- All files in schema
- Dependencies within schema (internal)
- Dependencies to other schemas (external)
- Which schemas depend on this schema

### Cross-Schema Dependencies

View all cross-schema relationships:

```bash
topcat schema -i sql/ -e sql dependencies
```

**Output**:
```
Cross-Schema Dependencies:

╭───────────────┬───┬─────────────╮
│ Source Schema ┆ → ┆ Target Schema │
╞═══════════════╪═══╪═════════════╡
│ auth          ┆ → ┆ public      │
│ auth          ┆ → ┆ billing     │
│ billing       ┆ → ┆ auth        │
│ billing       ┆ → ┆ public      │
╰───────────────┴───┴─────────────╯
```

Helps identify:
- Schema coupling
- Circular dependencies between schemas
- Architectural boundaries

## Schema Filtering

Filter analyze, clean, and export operations to specific schemas:

### Analysis with Schema Filter

```bash
# Analyze only one schema
topcat analyze -i sql/ -e sql --schema auth dead-branches
topcat analyze -i sql/ -e sql --schema billing orphans

# Multiple schemas
topcat analyze -i sql/ -e sql \
  --schema auth \
  --schema billing \
  leaf-nodes
```

### Cleanup with Schema Filter

```bash
# Clean only specific schema
topcat clean -i sql/ -e sql --schema auth orphans --no-dry-run

# Preview cleanup for schema
topcat clean -i sql/ -e sql --schema billing dead-branches
```

### Export with Schema Filter

```bash
# Export only auth schema
topcat export -i sql/ -e sql --schema auth -o auth.json json

# Export multiple schemas
topcat export -i sql/ -e sql \
  --schema auth \
  --schema billing \
  -o core.dot dot
```

## Common Workflows

### Multi-Schema Project Organization

```
Workflow Checklist:
- [ ] Step 1: List all schemas to understand distribution
      topcat schema -i sql/ -e sql list

- [ ] Step 2: Analyze each schema individually
      topcat schema -i sql/ -e sql analyze <schema_name>

- [ ] Step 3: Check cross-schema dependencies
      topcat schema -i sql/ -e sql dependencies

- [ ] Step 4: Identify coupling issues
      Look for unexpected cross-schema deps

- [ ] Step 5: Clean up by schema
      topcat clean -i sql/ -e sql --schema <name> orphans --no-dry-run
```

### Schema Health Check

```bash
# Check each schema for issues
for schema in auth billing public; do
    echo "Checking $schema..."
    topcat analyze -i sql/ -e sql --schema $schema cycles
    topcat analyze -i sql/ -e sql --schema $schema missing
    topcat analyze -i sql/ -e sql --schema $schema orphans
done
```

### Schema-Specific Cleanup

```bash
# Clean up one schema without affecting others
topcat clean -i sql/ -e sql \
  --schema deprecated_schema \
  unrequired --no-dry-run --force
```

### Visualize Schema Architecture

```bash
# Export each schema separately
topcat export -i sql/ -e sql --schema auth -o auth.dot dot
topcat export -i sql/ -e sql --schema billing -o billing.dot dot

# Generate images
dot -Tpng auth.dot -o auth.png
dot -Tpng billing.dot -o billing.png

# Or export cross-schema view
topcat export -i sql/ -e sql -o full.dot dot
```

## Schema-Aware Protection

Combine schema filtering with root node protection:

```bash
# Protect entry points within a schema
topcat analyze -i sql/ -e sql \
  --schema auth \
  --root-pattern "auth.api_*" \
  dead-branches

# Clean schema with protection
topcat clean -i sql/ -e sql \
  --schema billing \
  --root-nodes billing.invoices_table \
  orphans --no-dry-run
```

## Understanding Schema Extraction

### File Node Schema Field

Every `FileNode` has:
```rust
pub schema: Option<String>
```

Extracted automatically from node name during graph building.

### Extraction Rules

1. **Dot separator** (`.`):
   - `auth.users` → schema: "auth", name: "auth.users"
   - `a.b.c` → schema: "a", name: "a.b.c" (first segment)

2. **Double colon** (`::`):
   - `auth::users` → schema: "auth", name: "auth::users"
   - `a::b::c` → schema: "a", name: "a::b::c" (first segment)

3. **No separator**:
   - `users` → schema: None, name: "users"

4. **Mixed** (handled consistently):
   - `my_schema.table_name` → schema: "my_schema"
   - Uses first separator found

### Schema Definitions

Files can also DEFINE a schema (via SQL discovery):

```sql
-- Schema definition file
CREATE SCHEMA IF NOT EXISTS auth;
```

Node name: `auth` (the schema itself)
Schema field: `None` or special handling

## Use Cases

### Microservice Databases

Each service has its own schema:

```bash
# Check coupling between services
topcat schema -i sql/ -e sql dependencies

# Should show minimal cross-schema deps
```

### Gradual Migration

Migrating from one schema to another:

```bash
# Check what depends on old schema
topcat schema -i sql/ -e sql analyze old_schema

# Clean up unused files in old schema
topcat clean -i sql/ -e sql --schema old_schema orphans --no-dry-run
```

### Feature-Based Schemas

Different features in different schemas:

```bash
# Analyze feature schema
topcat schema -i sql/ -e sql analyze feature_x

# Export feature for documentation
topcat export -i sql/ -e sql --schema feature_x -o feature_x.md mermaid
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Schema not extracted | Check node name has `.` or `::` separator |
| Wrong schema name | First segment before separator is used |
| Schema filter not working | Verify schema name with `schema list` |
| Mixed schema notation | Be consistent: use `.` or `::`, not both |

## Detailed References

- **Schema patterns**: See [reference/patterns.md](reference/patterns.md)
- **Cross-schema analysis**: See [reference/dependencies.md](reference/dependencies.md)
- **Advanced filtering**: See [reference/filtering.md](reference/filtering.md)

## Related Skills

- `analyzing-dependencies` - Use schema filtering with analysis commands
- `exporting-graphs` - Export schema-specific visualizations
- `understanding-architecture` - How schema extraction works internally
