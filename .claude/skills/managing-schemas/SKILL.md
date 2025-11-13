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

View all schemas with statistics:

```bash
topcat schema -i sql/ -e sql list
```

Shows file count, dependency count, and distribution bar chart for each schema.

### Analyze Schema

Detailed view of a specific schema:

```bash
topcat schema -i sql/ -e sql analyze auth
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

### Visualize Schema Architecture

```bash
# Export each schema separately
topcat export -i sql/ -e sql --schema auth -o auth.dot dot
topcat export -i sql/ -e sql --schema billing -o billing.dot dot

# Generate images
dot -Tpng auth.dot -o auth.png
dot -Tpng billing.dot -o billing.png
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

## Common Patterns

### Layered Architecture

Good separation of concerns:

```
┌─────────────┐
│   public    │  (Shared utilities, extensions)
└─────────────┘
       ↑
       │
┌──────┴──────┬──────────────┬──────────────┐
│    auth     │   billing    │  reporting   │
└─────────────┴──────────────┴──────────────┘
```

Only allow dependencies flowing up (to `public`), no horizontal dependencies.

### Microservice Pattern

Each service has isolated schema with minimal cross-schema dependencies.

### Core-Periphery Pattern

Core schema is widely depended upon, peripheral schemas depend on core but not each other.

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

- **Schema patterns**: See [reference/patterns.md](reference/patterns.md) - Naming conventions, best practices, and migration patterns
- **Cross-schema analysis**: See [reference/dependencies.md](reference/dependencies.md) - Analyzing coupling, circular dependencies, and architectural patterns
- **Advanced filtering**: See [reference/filtering.md](reference/filtering.md) - Complex filter combinations, performance optimization, and practical examples

## Related Skills

- `analyzing-dependencies` - Use schema filtering with analysis commands
- `exporting-graphs` - Export schema-specific visualizations
- `understanding-architecture` - How schema extraction works internally
