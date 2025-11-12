---
name: clippy-fixing
description: Systematically runs clippy and fixes linting issues following project patterns. Use when running lints, addressing clippy warnings, or improving code quality before commits.
---

# Running Clippy and Fixing Issues

## Quick Start

### Basic Workflow

```bash
# 1. Run clippy to identify issues
cargo clippy --all-targets -- -D warnings

# 2. Fix automatically when possible
cargo clippy --fix --allow-dirty --allow-staged --all-targets

# 3. Run tests after fixes
cargo test

# 4. Verify build still works
cargo build
```

## Clippy Command Reference

### Standard Clippy Run

```bash
# Check all targets (lib, bins, tests)
cargo clippy --all-targets

# Treat warnings as errors (CI mode)
cargo clippy --all-targets -- -D warnings

# Check specific target
cargo clippy --lib
cargo clippy --bin topcat
cargo clippy --test analysis_tests
```

### Auto-fix Mode

```bash
# Fix issues automatically
cargo clippy --fix --allow-dirty --allow-staged --all-targets

# Fix with warnings as errors
cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings
```

**Notes**:
- `--allow-dirty`: Allow fixes in uncommitted workspace
- `--allow-staged`: Allow fixes in staged files
- `--all-targets`: Include bins, tests, examples

## Common Clippy Warnings in Topcat

### 1. Unnecessary Borrows

**Warning**: `needless_borrow`

```rust
// Before (clippy warning)
let name = &node.name.clone();

// After (fixed)
let name = &node.name;
```

### 2. Unnecessary Clones

**Warning**: `redundant_clone`

```rust
// Before (clippy warning)
fn process(s: &str) {
    let owned = s.to_string().clone();  // Unnecessary clone
}

// After (fixed)
fn process(s: &str) {
    let owned = s.to_string();
}
```

### 3. Manual String Formatting

**Warning**: `useless_format`

```rust
// Before (clippy warning)
let s = format!("{}", name);

// After (fixed)
let s = name.to_string();
```

### 4. Single Match

**Warning**: `single_match`

```rust
// Before (clippy warning)
match result {
    Ok(value) => println!("{}", value),
    _ => {}
}

// After (fixed)
if let Ok(value) = result {
    println!("{}", value);
}
```

### 5. Manual Filter-Map

**Warning**: `manual_filter_map`

```rust
// Before (clippy warning)
let results: Vec<_> = items
    .iter()
    .filter(|x| x.is_some())
    .map(|x| x.unwrap())
    .collect();

// After (fixed)
let results: Vec<_> = items
    .iter()
    .filter_map(|x| x.as_ref())
    .collect();
```

### 6. Explicit Into/From

**Warning**: `useless_conversion`

```rust
// Before (clippy warning)
let path: PathBuf = path_buf.into();  // Already PathBuf

// After (fixed)
let path = path_buf;
```

### 7. Large Enum Variants

**Warning**: `large_enum_variant`

```rust
// Before (clippy warning)
enum Error {
    Small(String),
    Large(VeryLargeStruct),  // Box large variants
}

// After (fixed)
enum Error {
    Small(String),
    Large(Box<VeryLargeStruct>),
}
```

## Systematic Fix Workflow

```
Clippy Fix Checklist:
- [ ] Step 1: Run clippy and review all warnings
- [ ] Step 2: Categorize warnings (auto-fix vs manual)
- [ ] Step 3: Run auto-fix for safe warnings
- [ ] Step 4: Manually fix remaining warnings
- [ ] Step 5: Run tests after each batch of fixes
- [ ] Step 6: Verify build succeeds
- [ ] Step 7: Run clippy again to confirm all fixed
- [ ] Step 8: Commit fixes with descriptive message
```

### Step-by-Step Process

#### 1. Initial Assessment

```bash
cargo clippy --all-targets 2>&1 | tee clippy-output.txt
```

Review output and count warnings by type.

#### 2. Run Auto-fix

```bash
cargo clippy --fix --allow-dirty --allow-staged --all-targets
```

#### 3. Test After Auto-fix

```bash
cargo test --lib --tests
```

If tests fail, review changes and fix manually.

#### 4. Manual Fixes

For warnings that can't be auto-fixed:
- Read clippy explanation: `rustc --explain E0XXX`
- Apply fix following project patterns
- Test immediately after each fix

#### 5. Verify All Fixed

```bash
cargo clippy --all-targets -- -D warnings
echo "Exit code: $?"
```

Exit code 0 means success.

## Reference Documentation

**Common Fixes**: See [reference/common-fixes.md](reference/common-fixes.md)
**Rust 2024**: See [reference/rust-2024.md](reference/rust-2024.md)
**Clippy Script**: See [scripts/clippy-check.sh](scripts/clippy-check.sh)

## Best Practices

### DO

✓ Run clippy before committing
✓ Fix warnings in small batches
✓ Test after each batch of fixes
✓ Review auto-fixes before committing
✓ Understand warnings before fixing
✓ Use `--all-targets` to catch test issues
✓ Treat warnings as errors in CI (`-D warnings`)

### DON'T

✗ Blindly apply auto-fixes without review
✗ Fix all warnings at once without testing
✗ Ignore warnings (fix or allow explicitly)
✗ Use `#[allow(...)]` without good reason
✗ Skip testing after fixes
✗ Mix clippy fixes with feature changes

## Allowing Specific Warnings

When a warning is intentional:

```rust
// Module-level
#![allow(clippy::too_many_arguments)]

// Function-level
#[allow(clippy::needless_return)]
fn explicit_return() -> i32 {
    return 42;
}

// Item-level
#[allow(clippy::large_enum_variant)]
enum MyEnum {
    Large(LargeStruct),
}
```

**Note**: Only allow warnings with clear justification. Add comment explaining why.

## Integration with Development

### Pre-commit Check

```bash
# Add to git pre-commit hook or CI
cargo clippy --all-targets -- -D warnings && cargo test
```

### CI Configuration

```yaml
# GitHub Actions example
- name: Run Clippy
  run: cargo clippy --all-targets -- -D warnings
```

## Troubleshooting

### Clippy Fails But Code Compiles

Clippy is stricter than compiler. Fix warnings or allow explicitly.

### Auto-fix Breaks Tests

Review the changes clippy made. Some auto-fixes may change semantics.
Revert and fix manually.

### Too Many Warnings

Fix in batches by category:
1. Unnecessary clones/borrows
2. Manual implementations (filter_map, etc.)
3. Style issues
4. Performance issues

### Conflicting Lints

Some lints may conflict. Use `#[allow(...)]` for one:

```rust
#[allow(clippy::needless_return)]
fn needs_explicit_return() -> i32 {
    return 42;  // Intentional for clarity
}
```
