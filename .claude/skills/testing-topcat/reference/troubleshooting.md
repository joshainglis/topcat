# Troubleshooting Common Issues

## Test Failures

### Flaky Tests

**Symptom**: Tests pass sometimes, fail others

**Solutions**:
```bash
# Run single-threaded
cargo test -- --test-threads=1

# Run specific test multiple times
for i in {1..10}; do
    cargo test test_name || break
done

# Increase timeouts in tests
#[test]
fn test_with_timeout() {
    let result = timeout(Duration::from_secs(10), async_operation());
    assert!(result.is_ok());
}
```

### File System Issues

**Symptom**: Tests fail with "Permission denied" or "File not found"

**Solutions**:
```rust
// Ensure proper cleanup
#[test]
fn test_with_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let result = std::panic::catch_unwind(|| {
        // Test code here
    });

    // Cleanup happens automatically when temp_dir drops
    result.unwrap();
}

// Add retry logic
fn write_with_retry(path: &Path, content: &str) -> Result<()> {
    for _ in 0..3 {
        match std::fs::write(path, content) {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(io::ErrorKind::PermissionDenied, "Failed after retries"))
}
```

### Memory Issues

**Symptom**: Tests fail with "out of memory" or segfaults

**Debug steps**:
```bash
# Run with memory limit
ulimit -v 500000  # 500MB limit
cargo test

# Use valgrind (Linux)
valgrind --leak-check=full cargo test test_name

# Check for stack overflow
RUST_MIN_STACK=8388608 cargo test  # 8MB stack
```

## Debugging Test Output

### No Output Visible

**Problem**: Test output not showing

**Solutions**:
```bash
# Show all output
cargo test -- --nocapture

# Show only for failed tests
cargo test -- --show-output

# With logging
RUST_LOG=debug cargo test -- --nocapture
```

### Too Much Output

**Problem**: Debug output overwhelming

**Solutions**:
```rust
// Conditional debug output
#[test]
fn test_with_conditional_output() {
    let verbose = std::env::var("VERBOSE").is_ok();

    if verbose {
        eprintln!("Debug: processing started");
    }

    // Test logic

    if verbose {
        eprintln!("Debug: result = {:?}", result);
    }
}
```

## Platform-Specific Issues

### Windows Line Endings

**Problem**: Tests fail on Windows due to CRLF

**Solution**:
```rust
// Normalize line endings
fn normalize_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn test_cross_platform() {
    let expected = normalize_line_endings(expected_output);
    let actual = normalize_line_endings(&actual_output);
    assert_eq!(expected, actual);
}
```

### Path Separators

**Problem**: Tests fail due to path separator differences

**Solution**:
```rust
use std::path::PathBuf;

#[test]
fn test_paths() {
    // Use PathBuf instead of string concatenation
    let path = PathBuf::from("tests")
        .join("input")
        .join("file.sql");

    // Convert to string when needed
    let path_str = path.to_string_lossy();
}
```

## CI/CD Issues

### Tests Pass Locally but Fail in CI

**Common causes**:
- Different environment variables
- Missing dependencies
- File permissions
- Timezone differences

**Debug steps**:
```yaml
# Add debugging to CI
- name: Debug environment
  run: |
    env | sort
    cargo --version
    rustc --version
    pwd
    ls -la

- name: Run tests with debug
  run: RUST_LOG=debug cargo test -- --nocapture
```

### Timeout Issues

**Problem**: CI jobs timeout

**Solutions**:
```yaml
# Increase timeout
- name: Run tests
  timeout-minutes: 30
  run: cargo test

# Split tests
- name: Run unit tests
  run: cargo test --lib

- name: Run integration tests
  run: cargo test --test '*'
```

## Common Error Messages

### "Cycle detected"

```bash
# Debug cycle
cargo run -- -i input/ -o output.sql -v 2>&1 | grep -A5 "Cycle"

# Visualize cycle
cargo run -- -i input/ -o output.sql -v 2>&1 | \
  grep "digraph" -A 1000 | dot -Tpng -o cycle.png
```

### "Missing dependency"

```bash
# Find missing dependency
grep -r "requires:.*missing_name" input/

# Check if file exists with different name
find input/ -type f | xargs grep "^-- name:" | grep -i "missing"
```

### "Could not compile"

```bash
# Clean and rebuild
cargo clean
cargo build

# Update dependencies
cargo update

# Check for version conflicts
cargo tree -d  # Show duplicate dependencies
```