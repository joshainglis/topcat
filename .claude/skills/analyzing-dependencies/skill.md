---
name: analyzing-dependencies
description: Guides dependency analysis and safe cleanup operations in Topcat. Use when analyzing dead branches, finding orphans, detecting cycles, cleaning unused files, or integrating with CI/CD pipelines.
---

# Analyzing Dependencies

## When to Use This Skill

- Analyzing dependency graphs for dead code
- Safely cleaning up unused files
- Detecting circular dependencies or missing refs
- Setting up CI/CD checks
- Protecting critical entry points from deletion

## Quick Command Reference

```bash
# Analysis
topcat analyze -i sql/ -e sql dead-branches    # Find dead subtrees
topcat analyze -i sql/ -e sql orphans          # Find isolated files
topcat analyze -i sql/ -e sql cycles           # Detect circular deps
topcat analyze -i sql/ -e sql missing          # Find missing refs
topcat analyze -i sql/ -e sql file path.sql    # Analyze single file

# Cleanup (dry-run by default)
topcat clean -i sql/ -e sql dead-branches      # Preview deletion
topcat clean -i sql/ -e sql orphans --no-dry-run  # Actually delete

# CI/CD
topcat analyze -i sql/ -e sql --quiet cycles   # Exit code 0/1
```

## Safe Cleanup Workflow

Follow this checklist when cleaning up files:

```
Pre-Cleanup Checklist:
- [ ] Step 1: Identify candidates
      topcat analyze -i sql/ -e sql dead-branches

- [ ] Step 2: Add external usage checking
      topcat analyze -i sql/ -e sql --external-check-dir src/ --external-check-pattern "*.py" dead-branches

- [ ] Step 3: Protect entry points
      topcat analyze -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

- [ ] Step 4: Preview deletion (dry-run)
      topcat clean -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

- [ ] Step 5: Execute with confirmation
      topcat clean -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches --no-dry-run

- [ ] Step 6: Force mode for automation (optional)
      topcat clean -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches --no-dry-run --force
```

## Analysis Commands

### Dead Branches

Finds complete dead subtrees (transitive closure):

```bash
# Basic
topcat analyze -i sql/ -e sql dead-branches

# With protection
topcat analyze -i sql/ -e sql \
  --root-nodes api_main \
  --external-check-dir src/ \
  --external-check-pattern "*.py" \
  dead-branches
```

Shows 🍃 leaf vs 🌿 branch nodes and deletion iteration savings.

### Node Classification

```bash
orphans       # No dependencies AND no dependents
leaf-nodes    # Have dependencies but no dependents
root-nodes    # Have dependents but no dependencies
unrequired    # Not required by any other files
```

### Health Checks

```bash
cycles        # Detect circular dependencies (exit 1 if found)
missing       # Find missing dependency refs (exit 1 if found)
file <path>   # Deep analysis of single file
```

## Root Node Protection

Protect critical entry points from being marked as dead:

### Four Methods

```bash
# 1. Exact names
--root-nodes api_main --root-nodes worker_main

# 2. Glob patterns
--root-pattern "**/api/*.sql" --root-pattern "**/migrations/*.sql"

# 3. Regex patterns
--root-regex "^api_.*" --root-regex "^worker_.*"

# 4. Directory-based
--root-dir sql/entry_points/ --root-dir sql/migrations/
```

### Config File

```toml
# topcat.toml
[analysis]
root_patterns = ["**/api/*.sql", "**/workers/*.sql"]
root_nodes = ["critical_function"]
```

Then:
```bash
topcat analyze -i sql/ -e sql --sql-config topcat.toml dead-branches
```

## External Usage Checking

Prevent false positives by checking external references:

```bash
topcat analyze -i sql/ -e sql \
  --external-check-dir src/api \
  --external-check-dir src/workers \
  --external-check-pattern "*.py" \
  --external-check-pattern "*.rs" \
  dead-branches
```

Uses parallel scanning with caching for performance.

## Cleanup Operations

### Safety Features

All cleanup commands include:
- Dry-run by default (must use `--no-dry-run`)
- Confirmation prompts (unless `--force`)
- Root node protection
- External usage checking
- Dependency validation

### Commands

```bash
# Preview
topcat clean -i sql/ -e sql dead-branches

# Execute
topcat clean -i sql/ -e sql dead-branches --no-dry-run

# Automation (no confirmation)
topcat clean -i sql/ -e sql dead-branches --no-dry-run --force
```

Cleanup types: `dead-branches`, `orphans`, `unrequired`, `targets <pattern>`

## CI/CD Integration

### Quiet Mode

```bash
# Exit code 0 = pass, 1 = fail
topcat analyze -i sql/ -e sql --quiet cycles

# In pipeline
if ! topcat analyze -i sql/ -e sql --quiet cycles; then
    echo "ERROR: Circular dependencies detected!"
    exit 1
fi
```

### Pre-commit Hook

```bash
#!/bin/bash
topcat analyze -i sql/ -e sql --quiet cycles || {
    echo "Commit rejected: circular dependencies found"
    exit 1
}
```

### Automated Cleanup

```bash
# CI job to clean orphans
topcat clean -i sql/ -e sql \
  --root-pattern "**/api/*.sql" \
  --sql-config topcat.toml \
  orphans --no-dry-run --force
```

## Schema Filtering

Limit analysis/cleanup to specific schemas:

```bash
topcat analyze -i sql/ -e sql --schema my_schema dead-branches
topcat clean -i sql/ -e sql --schema billing orphans --no-dry-run
```

See `managing-schemas` skill for detailed schema workflows.

## Common Issues

| Issue | Solution |
|-------|----------|
| False positives | Add `--external-check-dir` to check external code |
| Entry points marked dead | Use `--root-nodes` or `--root-pattern` |
| Too aggressive cleanup | Preview with dry-run, add more protection |
| Cycles blocking concat | Use `analyze cycles` to identify, break with layers |

## Detailed References

- **All command options**: See [reference/commands.md](reference/commands.md)
- **Protection patterns**: See [reference/protection.md](reference/protection.md)
- **CI/CD examples**: See [reference/ci-cd.md](reference/ci-cd.md)
- **Troubleshooting**: See [reference/troubleshooting.md](reference/troubleshooting.md)

## Related Skills

- `managing-schemas` - Schema-based organization and filtering
- `exporting-graphs` - Visualize dependencies before cleanup
- `understanding-architecture` - How analysis algorithms work
