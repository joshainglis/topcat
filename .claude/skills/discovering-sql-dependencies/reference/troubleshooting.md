# Troubleshooting SQL Discovery

## Common Issues

### Dependencies Not Found

**Symptom**: Expected dependencies missing from discovered list

**Diagnosis**:
```bash
# Enable verbose mode to see discovery process
topcat -i sql/ -o output.sql --enable-sql-discovery -v --dry

# Check if schema pattern matches
echo "schema_name" | grep -E "your_pattern"
```

**Solutions**:
- Adjust schema pattern to match naming convention
- Add type/extension mappings for special cases
- Use `!` prefix for dependencies that can't be discovered

### Circular Dependencies

**Symptom**: Error about circular dependencies after enabling discovery

**Diagnosis**:
```bash
# Visualize the dependency graph
topcat -i sql/ -o output.sql -v 2>&1 | grep "digraph" -A 1000 > graph.dot
dot -Tpng graph.dot -o graph.png
```

**Solutions**:
- Use layers to break cycles
- Review if manual dependencies were incorrect
- Consider if objects truly have circular dependencies

### Performance Issues

**Symptom**: Discovery takes too long on large codebases

**Solutions**:
```bash
# Process in batches
for dir in sql/*/; do
    topcat -i "$dir" -o "${dir}.sql" --enable-sql-discovery
done

# Disable discovery temporarily
topcat --merge-strategy header-only
```

## Debug Techniques

### Enable Debug Logging

```bash
# Rust debug logging
RUST_LOG=topcat::sql_parser=debug cargo run -- \
  -i sql/ -o output.sql --enable-sql-discovery

# Trace level for maximum detail
RUST_LOG=trace cargo run -- \
  -i sql/ -o output.sql --enable-sql-discovery
```

### Test Individual Files

```bash
# Test discovery on single file
echo "SELECT * FROM schema.table" | \
  cargo run -- --stdin --enable-sql-discovery \
  --schema-pattern "schema_\\w+" -v
```

### Validate Patterns

```rust
// Test pattern matching
use regex::Regex;

let pattern = r"(?:app|test)_\w+";
let re = Regex::new(pattern).unwrap();

assert!(re.is_match("app_users"));
assert!(re.is_match("test_data"));
assert!(!re.is_match("other_table"));
```