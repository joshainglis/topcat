# Migration Guide: Manual to Automatic Discovery

## Pre-Migration Assessment

### Analyze Current Dependencies

```bash
# Count files with manual dependencies
grep -l "^-- requires:" sql/*.sql | wc -l

# List unique dependency patterns
grep "^-- requires:" sql/*.sql | sort -u

# Find complex dependencies
grep "^-- requires:.*,.*,.*," sql/*.sql
```

## Migration Strategy

### Step 1: Validation

Verify discovery finds existing dependencies:

```bash
topcat -i sql/ -o /tmp/test.sql \
  --sql-config topcat.toml \
  --merge-strategy validate \
  -v 2>&1 | tee validation.log

# Review warnings
grep "WARNING" validation.log
```

### Step 2: Test on Subset

Start with a single schema or directory:

```bash
# Test on specific subdirectory
topcat -i sql/schema_auth/ -o /tmp/auth.sql \
  --enable-sql-discovery \
  --schema-pattern "auth_\\w+"

# Compare with manual version
diff /tmp/auth_manual.sql /tmp/auth_discovery.sql
```

### Step 3: Gradual Migration

Use `union` strategy for safety:

```bash
# Combine manual and discovered
topcat -i sql/ -o output.sql \
  --sql-config topcat.toml \
  --merge-strategy union
```

### Step 4: Full Migration

Switch to discovery-only:

```bash
# Generate updated headers to review
topcat -i sql/ -o output.sql \
  --sql-config topcat.toml \
  --merge-strategy discovery-only \
  --generate-headers /tmp/review_headers

# After review, update in-place
topcat -i sql/ -o output.sql \
  --sql-config topcat.toml \
  --update-headers
```

## Common Migration Issues

### Missing External Dependencies

```sql
-- Original (extension not discovered)
-- name: use_crypto
-- requires: pgcrypto_extension

-- Solution: Use override prefix
-- name: use_crypto
-- requires: !pgcrypto_extension
```

### Cross-Schema Dependencies

```toml
# Ensure pattern matches all schemas
schema_pattern = "(?:auth|app|billing)_\\w+"
```

### Dynamic Dependencies

```sql
-- Dynamic SQL won't be discovered
-- requires: !dynamic_table_1, !dynamic_table_2
```

## Rollback Plan

If discovery causes issues:

```bash
# Keep backup of original files
cp -r sql/ sql_backup/

# Revert to manual headers
topcat --merge-strategy header-only

# Or restore from backup
rm -rf sql/ && cp -r sql_backup/ sql/
```