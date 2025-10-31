# Debugging Guide

## Debug Logging

### Environment Variables

```bash
# Enable all debug logs
RUST_LOG=debug cargo run -- -i input/ -o output.sql

# Module-specific logging
RUST_LOG=topcat::file_dag=debug cargo run -- -i input/ -o output.sql

# Trace level (most verbose)
RUST_LOG=trace cargo test test_name

# Multiple modules
RUST_LOG=topcat::file_dag=debug,topcat::sql_parser=trace cargo run
```

### Adding Debug Output

```rust
// Use debug! macro
use log::debug;

debug!("Processing file: {:?}", file_path);
debug!("Dependencies found: {:?}", deps);

// Use dbg! for quick debugging
let value = dbg!(complex_calculation());

// Print to stderr
eprintln!("Debug: value = {:?}", value);
```

## Using Debuggers

### LLDB (macOS)

```bash
# Build with debug symbols
cargo build

# Run with lldb
lldb target/debug/topcat

# LLDB commands
(lldb) b file_dag.rs:123              # Set breakpoint
(lldb) r -i tests/input -o output.sql # Run with args
(lldb) p variable_name                # Print variable
(lldb) bt                             # Backtrace
(lldb) c                              # Continue
```

### GDB (Linux)

```bash
# Run with gdb
gdb target/debug/topcat

# GDB commands
(gdb) break file_dag.rs:123
(gdb) run -i tests/input -o output.sql
(gdb) print variable_name
(gdb) backtrace
(gdb) continue
```

### VS Code Debugging

```json
// .vscode/launch.json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Topcat",
      "cargo": {
        "args": [
          "build",
          "--bin=topcat"
        ],
        "filter": {
          "name": "topcat",
          "kind": "bin"
        }
      },
      "args": [
        "-i",
        "tests/input",
        "-o",
        "output.sql"
      ],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

## Graph Visualization

### Generate DOT Files

```bash
# Extract graph from verbose output
cargo run -- -i input/ -o output.sql -v 2>&1 | \
  sed -n '/digraph/,/^}/p' > graph.dot

# Convert to image
dot -Tpng graph.dot -o graph.png
dot -Tsvg graph.dot -o graph.svg  # For web viewing

# Interactive viewing
xdot graph.dot  # Linux
```

### Analyze Graph Structure

```bash
# Count nodes and edges
grep -c "^\s*\"" graph.dot  # Nodes
grep -c "\->" graph.dot      # Edges

# Find specific node
grep "node_name" graph.dot

# Extract subgraph
grep -E "(node1|node2|node3)" graph.dot > subgraph.dot
```

## Common Debugging Scenarios

### Cycle Detection

```rust
// Add debug output to cycle detection
match graph.validate() {
Err(TopCatError::CycleDetected { cycle, layer }) => {
eprintln ! ("Cycle found in layer {}:", layer);
for (i, node) in cycle.iter().enumerate() {
eprintln ! ("  {} -> {}", node, cycle[(i + 1) % cycle.len()]);
}
}
_ => {}
}
```

### Missing Dependencies

```bash
# Find which files reference a missing dependency
grep -r "requires:.*missing_dep" input/

# Check if file exists
find input/ -name "*missing_dep*"
```

### Performance Issues

```bash
# Profile with flamegraph
cargo install flamegraph
cargo flamegraph --bin topcat -- -i large_input/ -o output.sql

# Time different phases
time cargo run -- -i input/ -o /dev/null --dry  # Parse only
time cargo run -- -i input/ -o output.sql       # Full run
```

## Test Debugging

### Isolate Failing Tests

```bash
# Run single test with output
cargo test exact_test_name -- --exact --nocapture

# Skip other tests
cargo test -- --skip slow_test

# Show test timings
cargo test -- --show-output --report-time
```

### Debug Test Fixtures

```rust
#[test]
fn debug_test() {
    let temp_dir = TempDir::new().unwrap();

    // Print temp directory location
    eprintln!("Test dir: {:?}", temp_dir.path());

    // Keep directory for inspection (won't auto-delete)
    let path = temp_dir.into_path();
    eprintln!("Preserved at: {:?}", path);
}
```