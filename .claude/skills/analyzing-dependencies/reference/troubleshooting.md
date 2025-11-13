# Troubleshooting Analysis and Cleanup

## Common Issues

### False Positives in Dead Branches

**Symptom**: Files that ARE used are marked as dead branches.

**Causes**:
1. File is referenced in external code (Python, Rust, etc.)
2. File is an entry point but not in dependency graph
3. SQL discovery not finding all references

**Solutions**:

```bash
# 1. Add external usage checking
topcat analyze -i sql/ -e sql \
  --external-check-dir src/ \
  --external-check-pattern "*.py" \
  --external-check-pattern "*.rs" \
  dead-branches

# 2. Protect known entry points
topcat analyze -i sql/ -e sql \
  --root-pattern "**/api/*.sql" \
  dead-branches

# 3. Enable SQL discovery to find more references
topcat analyze -i sql/ -e sql \
  --enable-sql-discovery \
  --schema-pattern "myapp_\\w+" \
  dead-branches

# 4. Use config file to persist protection
# Create topcat.toml with root_patterns
topcat analyze -i sql/ -e sql --sql-config topcat.toml dead-branches
```

### Entry Points Marked as Dead

**Symptom**: API handlers, migrations, or entry points show as dead.

**Cause**: These files have no dependents in the graph (correct behavior, but they shouldn't be deleted).

**Solution**: Use root node protection:

```bash
# Method 1: Glob patterns (recommended)
topcat analyze -i sql/ -e sql \
  --root-pattern "**/api/*.sql" \
  --root-pattern "**/migrations/*.sql" \
  dead-branches

# Method 2: Exact names
topcat analyze -i sql/ -e sql \
  --root-nodes api_main \
  --root-nodes worker_entry \
  dead-branches

# Method 3: Directory-based
topcat analyze -i sql/ -e sql \
  --root-dir sql/entry_points/ \
  dead-branches

# Method 4: Config file (best for production)
# topcat.toml
# [analysis]
# root_patterns = ["**/api/*.sql", "**/migrations/*.sql"]
topcat analyze -i sql/ -e sql --sql-config topcat.toml dead-branches
```

### Cycles Not Detected

**Symptom**: Concat fails with cycle error, but `analyze cycles` shows nothing.

**Causes**:
1. Using soft dependencies (`exists`) which don't create cycles
2. Filtering excludes cycle participants
3. Cycle involves multiple schemas

**Solutions**:

```bash
# 1. Check without any filtering
topcat analyze -i sql/ -e sql cycles

# 2. Check specific schema
topcat analyze -i sql/ -e sql --schema my_schema cycles

# 3. Check file content for 'exists' vs 'requires'
grep -r "-- exists:" sql/

# 4. Verbose concat to see cycle details
topcat concat -i sql/ -o /tmp/out.sql -v
```

### Missing Dependencies Not Found

**Symptom**: Concat fails with missing dependency, but `analyze missing` shows nothing.

**Causes**:
1. Dependency added manually in header, not discovered
2. SQL discovery not enabled
3. Extension or schema-qualified name not mapped

**Solutions**:

```bash
# 1. Enable SQL discovery
topcat analyze -i sql/ -e sql \
  --enable-sql-discovery \
  --schema-pattern "myapp_\\w+" \
  missing

# 2. Check discovery configuration
topcat concat -i sql/ -o /tmp/out.sql \
  --enable-sql-discovery \
  --dry-run -v

# 3. Add type/extension mappings in topcat.toml
# [[sql_discovery.type_mappings]]
# from = "JSONB"
# to = "pg_catalog.jsonb"

# 4. Check manual headers
grep -r "-- requires:" sql/
```

### Cleanup Too Aggressive

**Symptom**: Too many files marked for deletion.

**Solutions**:

```bash
# 1. Always use dry-run first
topcat clean -i sql/ -e sql dead-branches

# 2. Add more protection
topcat clean -i sql/ -e sql \
  --root-pattern "**/api/*.sql" \
  --root-pattern "**/migrations/*.sql" \
  --root-pattern "**/critical/*.sql" \
  dead-branches

# 3. Use external checking
topcat clean -i sql/ -e sql \
  --external-check-dir src/ \
  --external-check-pattern "*.py" \
  dead-branches

# 4. Clean more conservatively (orphans instead of dead-branches)
topcat clean -i sql/ -e sql orphans
```

### Protection Patterns Not Working

**Symptom**: Files still marked as dead despite protection patterns.

**Debugging**:

```bash
# 1. Check if pattern matches (verbose mode)
topcat analyze -i sql/ -e sql \
  --root-pattern "**/api/*.sql" \
  dead-branches -v

# 2. Verify file path matches pattern
# Glob patterns match FILE PATHS, not node names
# Regex patterns match NODE NAMES, not file paths

# If file is: sql/api/handlers/users.sql
# Node name is: api_users
# Use: --root-pattern "**/api/**/*.sql" (matches path)
# Or: --root-regex "^api_.*" (matches node name)

# 3. Check config file is being loaded
topcat analyze -i sql/ -e sql --sql-config topcat.toml dead-branches -v

# 4. Test pattern separately
topcat analyze -i sql/ -e sql root-nodes
# Should show protected nodes
```

### External Checking Not Working

**Symptom**: Files referenced in Python/Rust still marked as dead.

**Debugging**:

```bash
# 1. Check if external files exist
ls -la src/api/*.py

# 2. Verify pattern matches
ls src/**/*.py

# 3. Check if file names are actually referenced
# Topcat looks for node name in file content
grep -r "my_function" src/

# 4. Use multiple directories
topcat analyze -i sql/ -e sql \
  --external-check-dir src/api \
  --external-check-dir src/workers \
  --external-check-dir src/cli \
  --external-check-pattern "*.py" \
  dead-branches
```

### Performance Issues

**Symptom**: Analysis takes too long.

**Solutions**:

```bash
# 1. Use schema filtering
topcat analyze -i sql/ -e sql --schema critical_schema dead-branches

# 2. Limit external checking scope
topcat analyze -i sql/ -e sql \
  --external-check-dir src/api \  # Not entire src/
  --external-check-pattern "*.py" \
  dead-branches

# 3. Check graph size
topcat analyze -i sql/ -e sql root-nodes | wc -l

# 4. Use file filtering
topcat analyze -i sql/ -e sql \
  --include-glob "**/critical/*.sql" \
  dead-branches
```

### Cleanup Fails Partially

**Symptom**: Some files deleted, some fail.

**Cause**: Permission issues, files in use, or dependent files.

**Solutions**:

```bash
# 1. Check error messages
topcat clean -i sql/ -e sql dead-branches --no-dry-run

# 2. Verify file permissions
ls -la sql/

# 3. Check if files are open
lsof sql/*.sql

# 4. Ensure no dependencies remain
# Topcat validates dependencies before deletion
topcat analyze -i sql/ -e sql file path/to/failing-file.sql
```

## Exit Code Issues

### CI Fails Unexpectedly

**Symptom**: CI job fails but local check passes.

**Debugging**:

```bash
# 1. Run with verbose mode locally
topcat analyze -i sql/ -e sql cycles

# 2. Check quiet mode exit codes
topcat analyze -i sql/ -e sql --quiet cycles
echo $?

# 3. Ensure config file is available in CI
# Check if topcat.toml is in git
git ls-files | grep topcat.toml

# 4. Check working directory in CI
pwd
ls -la
```

### Quiet Mode Not Returning Exit Code

**Symptom**: `--quiet` flag doesn't suppress output or set exit code.

**Cause**: Command doesn't support quiet mode.

**Solution**: Only these commands support `--quiet`:
- `analyze cycles --quiet`
- `analyze missing --quiet`

Other commands always output (dead-branches, orphans, etc.).

## Graph Structure Issues

### No Root Nodes Found

**Symptom**: `analyze root-nodes` returns nothing.

**Cause**: All files have dependencies (uncommon but possible).

**Solutions**:

```bash
# 1. Check if this is expected
topcat analyze -i sql/ -e sql leaf-nodes

# 2. Verify files have proper headers
grep -r "-- name:" sql/ | head
grep -r "-- requires:" sql/ | head

# 3. Check for cycles (might be hiding structure)
topcat analyze -i sql/ -e sql cycles
```

### Everything is Dead Branches

**Symptom**: All or most files marked as dead.

**Causes**:
1. No external usage checking
2. Entry points not protected
3. Files truly are unused

**Solutions**:

```bash
# 1. Protect entry points
topcat analyze -i sql/ -e sql \
  --root-pattern "**/api/*.sql" \
  dead-branches

# 2. Add external checking
topcat analyze -i sql/ -e sql \
  --external-check-dir src/ \
  --external-check-pattern "*.py" \
  dead-branches

# 3. Check root nodes
topcat analyze -i sql/ -e sql root-nodes

# 4. Verify file is actually used
# If not protected or externally referenced, it IS dead
```

### No Dead Branches Found

**Symptom**: Expected dead code but nothing found.

**Possibilities**:
1. Code is actually all used (good!)
2. Too much protection applied
3. External checking too broad

**Verification**:

```bash
# 1. Check without protection
topcat analyze -i sql/ -e sql dead-branches

# 2. Check orphans instead
topcat analyze -i sql/ -e sql orphans

# 3. Check leaf nodes
topcat analyze -i sql/ -e sql leaf-nodes
```

## Configuration Issues

### Config File Not Loaded

**Symptom**: Config settings not applied.

**Debugging**:

```bash
# 1. Verify config file path
ls -la topcat.toml

# 2. Check TOML syntax
# Use online TOML validator or:
python3 -c "import tomli; tomli.load(open('topcat.toml','rb'))"

# 3. Explicit path
topcat analyze -i sql/ -e sql --sql-config ./topcat.toml dead-branches

# 4. Check section names
# Must be [analysis] not [analyze]
cat topcat.toml
```

### CLI Flags Override Config

**Symptom**: Config file settings ignored when using CLI flags.

**Cause**: This is expected behavior - CLI flags take precedence.

**Solution**: Either use config file OR CLI flags, not both:

```bash
# Config file only
topcat analyze -i sql/ -e sql --sql-config topcat.toml dead-branches

# CLI flags only
topcat analyze -i sql/ -e sql --root-pattern "**/api/*.sql" dead-branches

# Don't mix (CLI overrides config)
```

## Getting Help

### Verbose Output

Add `-v` for detailed information:

```bash
topcat analyze -i sql/ -e sql dead-branches -v
```

### Debug Logging

Set RUST_LOG environment variable:

```bash
RUST_LOG=topcat=debug topcat analyze -i sql/ -e sql cycles
RUST_LOG=topcat::file_dag=debug topcat analyze -i sql/ -e sql dead-branches
```

### Export Graph for Visual Inspection

```bash
# Export to JSON
topcat export -i sql/ -e sql -o /tmp/graph.json json

# Export to DOT and visualize
topcat export -i sql/ -e sql -o /tmp/graph.dot dot
dot -Tpng /tmp/graph.dot -o /tmp/graph.png
open /tmp/graph.png
```

### Single File Deep Dive

```bash
# Analyze specific problematic file
topcat analyze -i sql/ -e sql file sql/problematic.sql

# Show all dependencies and dependents
```

## Quick Diagnostic Commands

```bash
# Health check
topcat analyze -i sql/ -e sql cycles        # Check for cycles
topcat analyze -i sql/ -e sql missing       # Check for missing
topcat analyze -i sql/ -e sql root-nodes    # Check entry points
topcat analyze -i sql/ -e sql leaf-nodes    # Check endpoints
topcat analyze -i sql/ -e sql orphans       # Check isolated files

# File count
topcat analyze -i sql/ -e sql root-nodes | wc -l
topcat analyze -i sql/ -e sql dead-branches | grep "Node Name" | wc -l

# Export for external analysis
topcat export -i sql/ -e sql -o /tmp/graph.json json
```
