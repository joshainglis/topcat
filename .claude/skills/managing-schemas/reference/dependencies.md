# Cross-Schema Dependencies

## Overview

Cross-schema dependencies occur when nodes in one schema depend on nodes in another schema. Understanding and managing these dependencies is crucial for:

- Maintaining architectural boundaries
- Preventing tight coupling between modules/services
- Planning migrations and refactoring
- Understanding system architecture

## Analyzing Cross-Schema Dependencies

### View All Cross-Schema Dependencies

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
│ reporting     ┆ → ┆ auth        │
│ reporting     ┆ → ┆ billing     │
╰───────────────┴───┴─────────────╯
```

This shows which schemas depend on which other schemas.

### Detailed Schema Analysis

```bash
topcat schema -i sql/ -e sql analyze auth
```

**Output includes**:
```
External Dependencies (2):

  To schema 'public':
    auth.users_table → public.extensions
    auth.sessions → public.pgcrypto

  To schema 'billing':
    auth.users_table → billing.customers

Dependent Schemas (1):
  billing
  reporting
```

Shows:
- **External dependencies**: What nodes in this schema depend on nodes in other schemas
- **Dependent schemas**: Which other schemas depend on this schema

## Dependency Types

### Outbound Dependencies

Dependencies **from** this schema **to** other schemas:

```sql
-- File: auth.users_table (in auth schema)
-- name: auth.users_table
-- requires: public.extensions
-- requires: billing.customer_types

CREATE TABLE auth.users (...);
```

This creates:
- `auth` → `public` (cross-schema)
- `auth` → `billing` (cross-schema)

### Inbound Dependencies

Dependencies **from** other schemas **to** this schema:

```bash
# Check what depends on auth schema
topcat schema -i sql/ -e sql analyze auth
```

Look for "Dependent Schemas" section.

## Coupling Analysis

### Low Coupling (Good)

```
auth     →  public (shared utilities only)
billing  →  public (shared utilities only)
```

**Characteristics**:
- Dependencies only to shared/common schemas
- No circular dependencies between business domains
- Clear architectural layers

### High Coupling (Warning)

```
auth     →  billing
billing  →  auth
auth     →  reporting
reporting → auth
```

**Characteristics**:
- Circular dependencies between schemas
- Many cross-schema connections
- Unclear architectural boundaries

### Detecting Circular Cross-Schema Dependencies

```bash
# Check for cycles in specific schemas
topcat analyze -i sql/ -e sql \
  --schema auth \
  --schema billing \
  cycles
```

If schemas are circularly dependent, this will show the cycle.

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

**Implementation**:
- Only allow dependencies flowing up (to `public`)
- No horizontal dependencies (auth ↔ billing)
- Each domain schema is independent

**Check**:
```bash
topcat schema -i sql/ -e sql dependencies
# Should only show: auth→public, billing→public, reporting→public
```

### Microservice Pattern

Each service has isolated schema:

```
┌─────────────┐   ┌──────────────┐   ┌──────────────┐
│ user_service│   │order_service │   │billing_service│
└─────────────┘   └──────────────┘   └──────────────┘
      ↓                  ↓                   ↓
  No cross-schema dependencies (or minimal)
```

**Check**:
```bash
topcat schema -i sql/ -e sql dependencies
# Should show minimal or no cross-schema deps
```

### Core-Periphery Pattern

Core schema is widely depended upon:

```
        ┌───────────────┐
        │     core      │
        └───────────────┘
               ↑
       ┌───────┼───────┐
       │       │       │
   ┌───┴──┐ ┌──┴───┐ ┌┴─────┐
   │ auth │ │billing│ │reports│
   └──────┘ └───────┘ └──────┘
```

**Check**:
```bash
topcat schema -i sql/ -e sql analyze core
# Should show many "Dependent Schemas"

topcat schema -i sql/ -e sql analyze auth
# Should show dependency "To schema 'core'"
```

## Finding Problematic Dependencies

### Schema with Too Many Outbound Dependencies

```bash
# Analyze specific schema
topcat schema -i sql/ -e sql analyze auth

# Look for long "External Dependencies" section
# If auth depends on many other schemas, consider refactoring
```

### Schema with Too Many Inbound Dependencies

```bash
topcat schema -i sql/ -e sql analyze auth

# Look for long "Dependent Schemas" section
# If many schemas depend on auth, it may be too coupled
```

### Breaking Circular Dependencies

**Problem**: `auth` and `billing` depend on each other.

**Solutions**:

1. **Extract common dependencies** to shared schema:
   ```
   Before:
   auth ↔ billing

   After:
   auth → shared ← billing
   ```

2. **Use soft dependencies** for one direction:
   ```sql
   -- In billing
   -- name: billing.invoices
   -- exists: auth.users  (soft dependency, no ordering)
   ```

3. **Refactor to remove dependency**:
   - Duplicate small functionality
   - Use application-layer integration instead

## Export Visualization

### Visualize Cross-Schema Dependencies

```bash
# Export full graph with schema highlighting
topcat export -i sql/ -e sql -o architecture.dot dot

# Use fdp layout for better schema clustering
fdp -Tpng architecture.dot -o architecture.png
```

DOT format uses:
- **Schema-based coloring** - Nodes colored by schema
- **Schema subgraphs** - Visual grouping
- **Cross-schema edges** - Red dashed lines

### Export Individual Schemas

```bash
# Export each schema separately
for schema in auth billing public; do
    topcat export -i sql/ -e sql \
        --schema $schema \
        -o ${schema}.dot dot
    dot -Tpng ${schema}.dot -o ${schema}.png
done
```

Compare images to see coupling.

## Best Practices

### 1. Define Clear Architectural Layers

```toml
# Document in topcat.toml
[analysis]
# Expected schema dependency flow
# public ← auth, billing, reporting
```

### 2. Regular Dependency Audits

```bash
# Run in CI/CD
topcat schema -i sql/ -e sql dependencies > schema-deps.txt
git add schema-deps.txt
# Review changes in PRs
```

### 3. Prevent Unwanted Dependencies

Use schema filtering in analysis:

```bash
# Ensure auth doesn't depend on billing
topcat schema -i sql/ -e sql analyze auth | grep "billing"
# If found, fail CI build
```

### 4. Document Allowed Dependencies

In your project documentation:

```markdown
## Schema Dependency Policy

Allowed:
- Any schema → public (shared utilities)
- reporting → auth, billing (read-only)

Prohibited:
- auth ↔ billing (circular)
- Any schema → reporting (reporting is leaf layer)
```

### 5. Use Export for Architecture Reviews

```bash
# Generate architecture diagram for reviews
topcat export -i sql/ -e sql -o architecture.md mermaid
# Add to PR description
```

## Troubleshooting

### Cannot Find Cross-Schema Dependency

**Symptom**: You know there's a dependency but it's not showing.

**Check**:
1. Verify node names use schema notation:
   ```bash
   topcat analyze -i sql/ -e sql root-nodes
   ```
2. Check dependency metadata:
   ```sql
   -- Make sure dependency includes schema
   -- requires: billing.customers  (not just customers)
   ```

### Too Many Cross-Schema Dependencies

**Symptom**: Schema analysis shows many external dependencies.

**Solutions**:
1. **Refactor** - Extract common code to shared schema
2. **Soft dependencies** - Use `exists` instead of `requires` where ordering doesn't matter
3. **Remove** - Use application-layer integration instead

### Circular Schema Dependencies

**Symptom**: Two schemas depend on each other.

**Solutions**:
1. **Break cycle** - Remove one dependency
2. **Introduce mediator schema** - Extract common functionality
3. **Use soft dependencies** - Change one direction to `exists`

## Advanced Queries

### Find All Dependencies Between Two Schemas

```bash
# Export JSON
topcat export -i sql/ -e sql -o graph.json json

# Query with jq
jq '.edges[] |
    select(.source | startswith("auth")) |
    select(.target | startswith("billing"))' graph.json
```

### Count Dependencies Per Schema Pair

```bash
# Export JSON
topcat export -i sql/ -e sql -o graph.json json

# Count edges between schemas
jq '.edges |
    group_by(.source | split(".")[0]) |
    map({
        schema: .[0].source | split(".")[0],
        count: length
    })' graph.json
```

### Find Schema with Most External Dependencies

```bash
# Analyze each schema and count external deps
for schema in $(topcat schema -i sql/ -e sql list | tail -n +2 | awk '{print $1}'); do
    count=$(topcat schema -i sql/ -e sql analyze $schema | grep -A 100 "External Dependencies" | grep "→" | wc -l)
    echo "$schema: $count external dependencies"
done
```

## Migration Strategies

### Splitting a Monolithic Schema

```
Before: all files in public schema
After: auth, billing, reporting schemas

Steps:
1. Plan new schema structure
2. Rename nodes with schema prefixes
3. Run analysis to find missing deps
4. Update dependencies
5. Verify with topcat analyze
```

### Merging Schemas

```
Before: auth_v1, auth_v2 schemas
After: single auth schema

Steps:
1. Identify all nodes in both schemas
2. Resolve naming conflicts
3. Update all dependencies
4. Rename nodes to single schema
5. Verify dependencies
```

## Related Commands

- `topcat schema list` - View all schemas
- `topcat schema analyze <name>` - Detailed schema view
- `topcat schema dependencies` - Cross-schema dependencies
- `topcat export --schema <name>` - Export specific schema
- `topcat analyze --schema <name>` - Filter analysis by schema
