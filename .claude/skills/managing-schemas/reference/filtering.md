# Advanced Schema Filtering

## Overview

Schema filtering allows you to scope operations to specific schemas, making it easier to work with large multi-schema projects. This reference covers advanced filtering techniques and combinations.

## Basic Schema Filtering

### Single Schema

```bash
# Analyze only one schema
topcat analyze -i sql/ -e sql --schema auth dead-branches

# Clean specific schema
topcat clean -i sql/ -e sql --schema billing orphans --no-dry-run

# Export specific schema
topcat export -i sql/ -e sql --schema auth -o auth.json json
```

### Multiple Schemas

```bash
# Analyze multiple schemas
topcat analyze -i sql/ -e sql \
  --schema auth \
  --schema billing \
  leaf-nodes

# Clean multiple schemas
topcat clean -i sql/ -e sql \
  --schema deprecated_v1 \
  --schema deprecated_v2 \
  orphans --no-dry-run
```

## Combining Filters

### Schema + Root Node Protection

Protect specific nodes within a schema:

```bash
# Analyze schema with protected entry points
topcat analyze -i sql/ -e sql \
  --schema auth \
  --root-nodes auth.api_login \
  --root-nodes auth.api_logout \
  dead-branches
```

### Schema + Root Pattern Protection

Protect pattern-matched nodes within schema:

```bash
# Protect all API endpoints in auth schema
topcat analyze -i sql/ -e sql \
  --schema auth \
  --root-pattern "auth.api_*" \
  dead-branches

# Clean billing with pattern protection
topcat clean -i sql/ -e sql \
  --schema billing \
  --root-pattern "billing.migrations_*" \
  orphans --no-dry-run
```

### Schema + External Usage Checking

Combine schema filtering with external usage checks:

```bash
# Check if "dead" auth schema nodes are used in Python code
topcat analyze -i sql/ -e sql \
  --schema auth \
  --external-check-dir src/ \
  --external-check-pattern "*.py" \
  dead-branches
```

### Schema + Layer Filtering

While there's no direct layer filter, you can use schema filtering with layer-aware analysis:

```bash
# Analyze schema (respects layer constraints)
topcat analyze -i sql/ -e sql \
  --schema auth \
  --layers prepend,normal,append \
  dead-branches
```

## Advanced Filtering Workflows

### Per-Schema Health Check

Check each schema independently:

```bash
#!/bin/bash
# health-check.sh

schemas=$(topcat schema -i sql/ -e sql list | tail -n +2 | awk '{print $1}')

for schema in $schemas; do
    echo "=== Checking $schema ==="

    # Check for cycles
    if topcat analyze -i sql/ -e sql --schema $schema --quiet cycles; then
        echo "✓ No cycles"
    else
        echo "✗ Cycles detected"
    fi

    # Check for missing deps
    if topcat analyze -i sql/ -e sql --schema $schema --quiet missing; then
        echo "✓ No missing dependencies"
    else
        echo "✗ Missing dependencies"
    fi

    # Check for orphans
    orphan_count=$(topcat analyze -i sql/ -e sql --schema $schema orphans | wc -l)
    echo "ℹ Orphans: $orphan_count"

    echo ""
done
```

### Schema-Specific Cleanup Pipeline

```bash
#!/bin/bash
# cleanup-schema.sh

SCHEMA=$1

if [ -z "$SCHEMA" ]; then
    echo "Usage: $0 <schema>"
    exit 1
fi

# Step 1: Analyze
echo "Step 1: Analyzing $SCHEMA for issues..."
topcat analyze -i sql/ -e sql --schema $SCHEMA dead-branches
topcat analyze -i sql/ -e sql --schema $SCHEMA orphans

# Step 2: Preview cleanup
echo "Step 2: Previewing cleanup..."
topcat clean -i sql/ -e sql --schema $SCHEMA dead-branches
topcat clean -i sql/ -e sql --schema $SCHEMA orphans

# Step 3: Confirm
read -p "Proceed with cleanup? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    topcat clean -i sql/ -e sql --schema $SCHEMA dead-branches --no-dry-run
    topcat clean -i sql/ -e sql --schema $SCHEMA orphans --no-dry-run
    echo "Cleanup complete!"
fi
```

### Cross-Schema Impact Analysis

Analyze impact of changes to one schema on others:

```bash
# Export dependencies of a critical node in auth schema
topcat export -i sql/ -e sql \
  --mode dependents \
  --node auth.users_table \
  -o auth_users_impact.json json

# Check which schemas are affected
jq '.nodes[] | .schema' auth_users_impact.json | sort -u
```

## Filter Combinations Reference

| Schema Filter | Root Protection | External Check | Use Case |
|---------------|-----------------|----------------|----------|
| ✓ | ✗ | ✗ | Basic schema isolation |
| ✓ | ✓ | ✗ | Protected cleanup within schema |
| ✓ | ✗ | ✓ | Verify external usage in schema |
| ✓ | ✓ | ✓ | Comprehensive safe cleanup |
| ✗ | ✓ | ✗ | Global protection (all schemas) |
| ✗ | ✗ | ✓ | Global external usage check |

## Schema Filtering with Export Modes

### Export Dependencies Within Schema

```bash
# Find all deps of a node, filtered to same schema
topcat export -i sql/ -e sql \
  --schema auth \
  --mode deps \
  --node auth.sessions \
  -o auth_sessions_deps.json json
```

This shows only dependencies within the `auth` schema.

### Export Dependents Within Schema

```bash
# Find what depends on a node within schema
topcat export -i sql/ -e sql \
  --schema billing \
  --mode dependents \
  --node billing.customers \
  -o billing_customers_impact.json json
```

Shows only dependents within the `billing` schema.

### Cross-Schema Dependency Export

To see cross-schema dependencies, don't use schema filter:

```bash
# Export all dependents (including other schemas)
topcat export -i sql/ -e sql \
  --mode dependents \
  --node auth.users_table \
  -o auth_users_all_dependents.json json
```

## Filtering by Schema in Configuration

### Configuration File Setup

```toml
# topcat.toml
[analysis]
# Default schemas to analyze
default_schemas = ["auth", "billing"]

# Schemas to exclude from analysis
exclude_schemas = ["deprecated", "test"]

# Per-schema root patterns
[analysis.schema_patterns]
auth = ["auth.api_*", "auth.migrations_*"]
billing = ["billing.api_*"]
```

**Note**: Configuration file schema filtering is a future enhancement. Currently, use CLI flags.

## Practical Examples

### Example 1: Migrate Schema with Protection

```bash
# Analyze old schema with protection for still-used endpoints
topcat analyze -i sql/ -e sql \
  --schema legacy_v1 \
  --root-pattern "legacy_v1.api_*" \
  --external-check-dir src/ \
  --external-check-pattern "*.java" \
  dead-branches

# Clean up truly dead code
topcat clean -i sql/ -e sql \
  --schema legacy_v1 \
  --root-pattern "legacy_v1.api_*" \
  --external-check-dir src/ \
  --external-check-pattern "*.java" \
  dead-branches --no-dry-run
```

### Example 2: Feature Flag Schema Cleanup

```bash
# Check if feature flag schema is still needed
topcat analyze -i sql/ -e sql \
  --schema feature_x \
  --external-check-dir src/ \
  --external-check-pattern "*.ts" \
  --external-check-pattern "*.tsx" \
  orphans

# If all orphaned, safe to delete
topcat clean -i sql/ -e sql \
  --schema feature_x \
  unrequired --no-dry-run
```

### Example 3: Service Schema Health Check

```bash
# Check auth service schema for issues
topcat analyze -i sql/ -e sql --schema auth_service cycles
topcat analyze -i sql/ -e sql --schema auth_service missing
topcat analyze -i sql/ -e sql --schema auth_service orphans

# Visualize auth service dependencies
topcat export -i sql/ -e sql \
  --schema auth_service \
  -o auth_service.dot dot
dot -Tpng auth_service.dot -o auth_service.png
```

### Example 4: Multi-Schema Refactoring

```bash
# Analyze impact of refactoring auth and billing together
topcat analyze -i sql/ -e sql \
  --schema auth \
  --schema billing \
  dead-branches

# Export combined view
topcat export -i sql/ -e sql \
  --schema auth \
  --schema billing \
  -o auth_billing.md mermaid
```

## Filtering with JSON Export and jq

### Filter Nodes by Schema

```bash
# Export full graph
topcat export -i sql/ -e sql -o graph.json json

# Query specific schema
jq '.nodes[] | select(.schema=="auth")' graph.json

# Count nodes per schema
jq '.nodes | group_by(.schema) | map({schema: .[0].schema, count: length})' graph.json
```

### Filter Edges Within Schema

```bash
# Find all edges within auth schema
jq '.edges[] |
    select(.source | startswith("auth")) |
    select(.target | startswith("auth"))' graph.json
```

### Find Cross-Schema Edges

```bash
# Find edges from auth to billing
jq '.edges[] |
    select(.source | startswith("auth")) |
    select(.target | startswith("billing"))' graph.json
```

## Performance Considerations

### Large Multi-Schema Projects

For projects with many schemas and files:

1. **Use schema filtering** to reduce analysis scope:
   ```bash
   # Faster than analyzing all schemas
   topcat analyze -i sql/ -e sql --schema auth dead-branches
   ```

2. **Parallelize per-schema analysis**:
   ```bash
   # Run analyses in parallel
   for schema in auth billing reporting; do
       (topcat analyze -i sql/ -e sql --schema $schema orphans > ${schema}_orphans.txt) &
   done
   wait
   ```

3. **Use quiet mode** for CI/CD:
   ```bash
   # Faster output
   topcat analyze -i sql/ -e sql --schema auth --quiet cycles
   ```

## Troubleshooting

### Schema Filter Not Working

**Symptom**: `--schema auth` doesn't filter expected files.

**Diagnosis**:
```bash
# Check schema names
topcat schema -i sql/ -e sql list

# Check node names
topcat analyze -i sql/ -e sql root-nodes | grep auth
```

**Common issues**:
- Node name uses underscore: `auth_users` (should be `auth.users`)
- Schema name typo: `--schema Auth` (case-sensitive)
- Missing schema separator in node names

### No Nodes Found After Filtering

**Symptom**: Schema filter returns empty results.

**Check**:
```bash
# Verify schema exists
topcat schema -i sql/ -e sql list | grep "my_schema"

# View nodes in schema
topcat schema -i sql/ -e sql analyze my_schema
```

### Filter Combination Not Working

**Symptom**: Schema + root pattern doesn't work as expected.

**Debug**:
```bash
# Test without schema filter
topcat analyze -i sql/ -e sql --root-pattern "auth.*" dead-branches

# Test without root pattern
topcat analyze -i sql/ -e sql --schema auth dead-branches

# Combine (both filters must match)
topcat analyze -i sql/ -e sql --schema auth --root-pattern "auth.api_*" dead-branches
```

## Best Practices

1. **Use schema filtering in CI/CD** - Faster builds, focused checks
2. **Combine with protection patterns** - Safer cleanup operations
3. **Filter before export** - Smaller output files, clearer visualizations
4. **Document schema boundaries** - Make filtering rules explicit
5. **Test filters** - Verify schema names with `schema list` first

## Related Commands

- `topcat schema list` - View available schemas
- `topcat schema analyze <name>` - Detailed schema view
- `topcat analyze --help` - All filtering options
- `topcat clean --help` - Cleanup with filtering
- `topcat export --help` - Export with filtering
