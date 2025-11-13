# Analysis Commands Reference

## Dead Branches Detection

**Purpose**: Find complete dead subtrees that can be deleted together using transitive closure.

**Algorithm**: Iteratively finds not just leaf nodes, but all nodes whose only dependents are also dead.

### Basic Usage

```bash
topcat analyze -i sql/ -e sql dead-branches
```

### With External Usage Checking

```bash
topcat analyze -i sql/ -e sql \
  --external-check-dir src/api \
  --external-check-dir src/workers \
  --external-check-pattern "*.py" \
  --external-check-pattern "*.rs" \
  dead-branches
```

### With Root Node Protection

```bash
topcat analyze -i sql/ -e sql \
  --root-nodes api_main \
  --root-nodes worker_main \
  --root-pattern "**/api/*.sql" \
  dead-branches
```

### Output Format

```
🌳 Dead Branches Analysis
═══════════════════════════════════════════════════════════

📊 Found 6 node(s) in dead branches:

   • Leaf nodes (initial): 1
   • Additional nodes (pulled in): 5
   • Total nodes in dead branches: 6

💡 Benefit: Trimming avoids 5 additional deletion iteration(s)

┌─────────┬──────────┬──────────┬─────────────────────┐
│ Node    │ Type     │ Deps     │ File Path           │
├─────────┼──────────┼──────────┼─────────────────────┤
│ a       │ 🍃 Leaf  │ 2        │ sql/a.sql           │
│ b       │ 🌿 Branch│ 1        │ sql/b.sql           │
└─────────┴──────────┴──────────┴─────────────────────┘
```

## Orphan Detection

**Purpose**: Find files with no dependencies AND no dependents (completely isolated).

```bash
topcat analyze -i sql/ -e sql orphans
```

**Use cases**:
- Experimental or test files
- Leftover development artifacts
- Files that can be safely removed

## Leaf Nodes

**Purpose**: Files with dependencies but no dependents (endpoints in the graph).

```bash
topcat analyze -i sql/ -e sql leaf-nodes
```

**Use cases**:
- Understanding graph endpoints
- Identifying potential dead code (if not used externally)
- Finding files that don't contribute to others

## Root Nodes

**Purpose**: Files with dependents but no dependencies (starting points).

```bash
topcat analyze -i sql/ -e sql root-nodes
```

**Use cases**:
- Identifying entry points
- Finding foundation/base files
- Understanding graph structure

## Unrequired Nodes

**Purpose**: Files not required by any other files (similar to orphans + leaf nodes combined).

```bash
topcat analyze -i sql/ -e sql unrequired
```

## Cycle Detection

**Purpose**: Detect circular dependencies (violates DAG requirement).

```bash
# Verbose output
topcat analyze -i sql/ -e sql cycles

# CI/CD mode (exit code 1 if cycles found)
topcat analyze -i sql/ -e sql --quiet cycles
```

### Output Format

```
🔄 Cycle Detection Analysis
═══════════════════════════════════════════════════════════

⚠️  Found 1 cycle(s) in the dependency graph:

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Cycle #1
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Participants:
+------+-----------------+
| Node | File Path       |
+=======================+
| a    | sql/a.sql       |
| b    | sql/b.sql       |
| c    | sql/c.sql       |
+------+-----------------+

Cycle Path:
  a → b
  b → c
  c → a

💡 How to fix cycles:
   1. Remove one of the dependencies in the cycle
   2. Use 'exists' instead of 'requires' for soft dependencies
   3. Restructure code to break circular dependencies
   4. Use layers to enforce ordering between groups
```

## Missing Dependencies

**Purpose**: Find references to non-existent dependencies.

```bash
# Verbose output
topcat analyze -i sql/ -e sql missing

# CI/CD mode (exit code 1 if missing found)
topcat analyze -i sql/ -e sql --quiet missing
```

### Output Format

```
🔍 Missing Dependencies Analysis
═══════════════════════════════════════════════════════════

⚠️  Found 2 file(s) with missing dependencies:

┌──────────────┬──────────────────┬─────────────────┐
│ File         │ Missing Dep      │ File Path       │
├──────────────┼──────────────────┼─────────────────┤
│ my_function  │ nonexistent_dep  │ sql/func.sql    │
└──────────────┴──────────────────┴─────────────────┘

💡 How to fix:
   1. Add the missing file to the input directory
   2. Fix the dependency reference in the header
   3. Use 'exists' for optional dependencies
```

## Single File Analysis

**Purpose**: Deep analysis of a specific file showing all relationships.

```bash
topcat analyze -i sql/ -e sql file sql/my_file.sql
```

### Output Format

```
📄 File Analysis: sql/my_file.sql
═══════════════════════════════════════════════════════════

Node Name: my_function
File Path: sql/my_file.sql
Layer: normal

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Direct Dependencies (2):
+──────────────+
| Dependency   |
+==============+
| base_schema  |
| extensions   |
+──────────────+

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Direct Dependents (1):
+───────────+
| Dependent |
+===========+
| api_main  |
+───────────+

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Summary:
  Node Type: Intermediate Node
  ✅ Required by other files
```

## Cleanup Commands

### Dead Branches Cleanup

```bash
# Preview (dry-run - default)
topcat clean -i sql/ -e sql dead-branches

# Execute with confirmation
topcat clean -i sql/ -e sql dead-branches --no-dry-run

# Force mode (no confirmation)
topcat clean -i sql/ -e sql dead-branches --no-dry-run --force
```

### Other Cleanup Types

```bash
# Clean orphans
topcat clean -i sql/ -e sql orphans --no-dry-run

# Clean unrequired
topcat clean -i sql/ -e sql unrequired --no-dry-run

# Clean specific files with glob pattern
topcat clean -i sql/ -e sql targets "old_*.sql" --no-dry-run
```

### With Protection

```bash
# Combine all protection mechanisms
topcat clean -i sql/ -e sql \
  --root-nodes api_main \
  --root-pattern "**/api/*.sql" \
  --root-regex "^critical_.*" \
  --root-dir sql/migrations/ \
  --external-check-dir src/ \
  --external-check-pattern "*.py" \
  dead-branches --no-dry-run
```

### With Schema Filtering

```bash
# Clean only specific schema(s)
topcat clean -i sql/ -e sql \
  --schema my_schema \
  orphans --no-dry-run
```

## Safety Features

All cleanup operations include:

1. **Dry-run by default** - Must explicitly use `--no-dry-run` to delete
2. **Confirmation prompts** - Interactive "Are you sure?" unless `--force`
3. **Root node protection** - Respects all 4 protection pattern types
4. **External usage checking** - Filters files referenced in external code
5. **Dependency validation** - Prevents deletion of files still required
6. **Error handling** - Reports failures, continues with remaining files
7. **Preview tables** - Shows exactly what will be deleted before deletion

## Command Line Options

### Common Flags

```bash
-i, --input-dirs <DIR>              # Input directories (required)
-o, --output <FILE>                 # Output file (for concat)
-e, --include-exts <EXT>            # File extensions to include
--schema <NAME>                     # Filter by schema
--quiet                             # Suppress output, return only exit codes

# Protection
--root-nodes <NAME>                 # Exact node names to protect
--root-pattern <PATTERN>            # Glob patterns for protection
--root-regex <REGEX>                # Regex patterns for protection
--root-dir <DIR>                    # Directory-based protection

# External checking
--external-check-dir <DIR>          # Directories to check for references
--external-check-pattern <PATTERN>  # File patterns to scan

# Cleanup options
--no-dry-run                        # Actually delete files
--force                             # Skip confirmation prompts

# Config
--sql-config <FILE>                 # Load config from TOML file
```

## Performance Notes

- External usage checking uses parallel scanning (rayon)
- Progress bars appear for operations >100 items
- File content caching speeds up repeated lookups
- Schema filtering reduces scope for large projects
