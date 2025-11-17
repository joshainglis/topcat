# Logging Unification - Implementation Progress

**Goal**: Unify all logging in Topcat to use a consistent, shared logging module that respects `--quiet` and `--verbose` flags across all commands.

**Started**: 2025-01-17

## Problem Statement

Topcat had inconsistent logging across different modules:
- Analyze commands used `AnalysisLogger` (custom abstraction)
- Clean commands used direct `println!`/`eprintln!` with NO quiet mode support
- Schema and Config commands ignored quiet mode entirely
- Concat command mixed `println!` and `log::info!`
- Two separate systems: `log` crate (library) vs direct output (commands)
- `LoggingConfig` existed in settings but was unused

## Solution Architecture

### Two-Tier Logging System

**1. User-Facing Output** (`topcat::logging::Logger`):
- All messages displayed to users
- Controlled by `--quiet` and `--verbose` flags
- Methods: `error`, `warn`, `success`, `info`, `debug`, `section`, `table`, `separator`, `newline`, `prompt`, `progress`

**2. Internal Debug Logging** (`log` crate):
- Library-level debugging for development
- Controlled by `RUST_LOG` environment variable and `--verbose` flag
- Used in file_dag, io_utils, sql_parser, etc.

### Logger Behavior

| Method | Quiet Mode | Normal Mode | Verbose Mode |
|--------|------------|-------------|--------------|
| `error()` | ✅ Shown | ✅ Shown | ✅ Shown |
| `warn()` | ✅ Shown | ✅ Shown | ✅ Shown |
| `success()` | ❌ Hidden | ✅ Shown | ✅ Shown |
| `info()` | ❌ Hidden | ✅ Shown | ✅ Shown |
| `debug()` | ❌ Hidden | ❌ Hidden | ✅ Shown |
| `prompt()` | ✅ Shown (interactive) | ✅ Shown | ✅ Shown |

### Initialization Pattern

Each command follows this pattern:

```rust
// Load settings
let mut settings = Settings::load(config_path)?;
self.common.apply_to_settings(&mut settings);
settings.validate()?;

// Initialize logging
let quiet = settings.behavior.quiet;
let verbose = settings.behavior.verbose;
init_logging(verbose, quiet);

// Create logger instance
let logger = Logger::new(quiet, verbose);

// Use logger throughout command
logger.info("Processing files...");
```

## Implementation Progress

### ✅ Phase 1: Core Infrastructure (COMPLETED)

**Files Created/Modified:**

1. **`src/logging.rs`** (NEW - 350 lines)
   - Created unified `Logger` struct
   - Implemented all logging methods
   - Added `init_logging()` function
   - Includes comprehensive documentation and tests

2. **`src/lib.rs`**
   - Added `pub mod logging;` to exports

### ✅ Phase 2: Analyze Commands (COMPLETED)

**All 10 modules migrated:**

| Module | Lines Changed | Key Changes |
|--------|---------------|-------------|
| `src/commands/analyze/mod.rs` | ~15 | Added logger initialization, updated function calls |
| `src/commands/analyze/common.rs` | ~20 | Removed `AnalysisLogger`, updated `analyze_and_display` |
| `src/commands/analyze/orphans.rs` | ~10 | Changed signature to use `&Logger` |
| `src/commands/analyze/unrequired.rs` | ~10 | Changed signature to use `&Logger` |
| `src/commands/analyze/leaf_nodes.rs` | ~10 | Changed signature to use `&Logger` |
| `src/commands/analyze/root_nodes.rs` | ~10 | Changed signature to use `&Logger` |
| `src/commands/analyze/dead_branches.rs` | ~15 | Changed signature to use `&Logger` |
| `src/commands/analyze/cycles.rs` | ~10 | Changed signature to use `&Logger` |
| `src/commands/analyze/missing.rs` | ~10 | Changed signature to use `&Logger` |
| `src/commands/analyze/file.rs` | ~10 | Changed signature to use `&Logger` |

**Key Achievement**: Removed old `AnalysisLogger` entirely, replaced with unified `Logger`.

### ✅ Phase 3: Clean Commands (COMPLETED)

**All 6 modules migrated + QUIET MODE ADDED:**

| Module | Lines Changed | Key Changes |
|--------|---------------|-------------|
| `src/commands/clean/mod.rs` | ~20 | Added logger initialization, removed verbose param |
| `src/commands/clean/common.rs` | ~30 | Updated to use `logger.prompt()`, removed `verbose` param |
| `src/commands/clean/orphans.rs` | ~15 | Changed signature to use `&Logger` |
| `src/commands/clean/dead_branches.rs` | ~15 | Changed signature to use `&Logger` |
| `src/commands/clean/unrequired.rs` | ~15 | Changed signature to use `&Logger` |
| `src/commands/clean/targets.rs` | ~20 | Changed signature to use `&Logger` |

**Major Improvements:**
- ✅ Clean commands now respect `--quiet` mode (previously ignored)
- ✅ Interactive prompts use `logger.prompt()` instead of manual stdin/stdout
- ✅ Verbose file deletion output moved to `logger.debug()` (only shown with `-v`)
- ✅ Removed redundant `verbose` parameter (now part of Logger)

### 🔄 Phase 4: Concat Command (TODO)

**Files to modify:**
- `src/commands/concat.rs` (~200 lines)

**Current state:**
- Uses `log::info!` for some output
- Uses `println!` for verbose DOT graph output
- Mixed logging approach

**Migration plan:**
1. Add logger initialization after settings load
2. Replace `log::info!` with `logger.info()` for user-facing messages
3. Replace `println!` with `logger.debug()` or `logger.info()`
4. Keep `log::info!` for library-level debugging

### 🔄 Phase 5: Schema Command (TODO)

**Files to modify:**
- `src/commands/schema.rs` (~250 lines)

**Current state:**
- Heavy use of direct `println!` (18 occurrences)
- No quiet mode support
- Lots of table output

**Migration plan:**
1. Add logger initialization
2. Replace all `println!` with appropriate logger methods
3. Tables use `logger.table()`
4. Info messages use `logger.info()`

### 🔄 Phase 6: Config Command (TODO)

**Files to modify:**
- `src/commands/config.rs` (~180 lines)

**Current state:**
- Uses both `println!` and `eprintln!` (12 occurrences)
- No quiet mode support

**Migration plan:**
1. Add logger initialization
2. Replace `println!` with `logger.info()`
3. Replace `eprintln!` with `logger.error()` or `logger.warn()`

### 🔄 Phase 7: Export Commands (TODO)

**Files to check:**
- `src/commands/export/` directory (multiple modules)

**Current state:** Unknown - need to investigate

**Migration plan:**
1. Analyze current logging approach
2. Add logger initialization if needed
3. Update to use unified Logger

### 🔄 Phase 8: Cleanup (TODO)

**Tasks:**

1. **Remove unused `LoggingConfig` from `src/settings.rs`**
   - Lines 354-391 (LoggingConfig struct and From impl)
   - Clean up any references

2. **Add quiet/verbose mutual exclusion validation**
   - Add validation in `src/cli.rs` or settings validation
   - Warn user if both flags are set
   - Decision: Which takes precedence? (Current: quiet takes precedence)

3. **Update documentation**
   - Update `CLAUDE.md` with new logging patterns
   - Update command help text if needed
   - Update skills if applicable

### 🔄 Phase 9: Testing & Quality (TODO)

**Tasks:**

1. **Run existing tests**
   ```bash
   cargo test
   ```

2. **Manual testing scenarios**
   - Test each command with `--quiet`
   - Test each command with `--verbose`
   - Test each command with both flags
   - Test interactive prompts in clean commands
   - Verify CI/CD quiet mode works correctly

3. **Run clippy**
   ```bash
   cargo clippy --all-targets
   ```

4. **Fix any linting issues**

## Key Decisions & Patterns

### 1. Logger Ownership
- Logger is passed by reference (`&Logger`) to all functions
- Created once per command execution
- Avoids cloning/copying

### 2. Error vs Warn
- `error()`: Critical failures, always shown
- `warn()`: Important warnings, always shown
- Use `error()` for user-actionable errors
- Use `warn()` for informational warnings

### 3. Debug Level Usage
- Individual file deletions moved to `debug()`
- Detailed progress information in `debug()`
- Normal operation uses `info()` and `progress()`

### 4. Interactive Prompts
- Always shown, even in quiet mode (user interaction required)
- Use `logger.prompt()` method
- Returns `Result<bool, io::Error>`

### 5. Quiet Mode Philosophy
- Errors and warnings always shown (actionable information)
- Success messages and progress hidden
- Suitable for CI/CD where you only care about failures

## Migration Checklist Template

When migrating a command, follow these steps:

- [ ] Add `use topcat::logging::{Logger, init_logging};` to imports
- [ ] Remove any `env_logger::Builder` imports/usage
- [ ] Add logger initialization after settings validation:
  ```rust
  let quiet = settings.behavior.quiet;
  let verbose = settings.behavior.verbose;
  init_logging(verbose, quiet);
  let logger = Logger::new(quiet, verbose);
  ```
- [ ] Update function signatures to accept `&Logger` instead of `quiet: bool` and/or `verbose: bool`
- [ ] Replace all `println!` with appropriate `logger.*()` calls
- [ ] Replace all `eprintln!` with `logger.error()` or `logger.warn()`
- [ ] Replace interactive prompts with `logger.prompt()`
- [ ] Update function call sites to pass `&logger`
- [ ] Test build: `cargo build --quiet`
- [ ] Manual test with `--quiet` and `--verbose` flags

## Build Status

**Last successful build**: 2025-01-17
**Compilation**: ✅ Success (no errors, no warnings)

## Notes for Future Work

1. **Consider**: Should we add a `--no-color` flag for non-TTY environments?
2. **Consider**: Structured logging (JSON output) for machine parsing?
3. **Consider**: Log levels via environment variables (separate from quiet/verbose)?
4. **Future Enhancement**: Progress bars for long-running operations
5. **Future Enhancement**: Timestamps in verbose mode

## Sessions

### Session 1 (2025-01-17)
- Created logging module
- Migrated analyze commands (all 10 modules)
- Migrated clean commands (all 6 modules)
- **Status**: ~40% complete

## Files Changed Summary

**New files:**
- `src/logging.rs` (350 lines)

**Modified files (analyze):**
- `src/commands/analyze/mod.rs`
- `src/commands/analyze/common.rs`
- `src/commands/analyze/orphans.rs`
- `src/commands/analyze/unrequired.rs`
- `src/commands/analyze/leaf_nodes.rs`
- `src/commands/analyze/root_nodes.rs`
- `src/commands/analyze/dead_branches.rs`
- `src/commands/analyze/cycles.rs`
- `src/commands/analyze/missing.rs`
- `src/commands/analyze/file.rs`

**Modified files (clean):**
- `src/commands/clean/mod.rs`
- `src/commands/clean/common.rs`
- `src/commands/clean/orphans.rs`
- `src/commands/clean/dead_branches.rs`
- `src/commands/clean/unrequired.rs`
- `src/commands/clean/targets.rs`

**Modified files (infrastructure):**
- `src/lib.rs`

**Total files changed**: 20

## Next Session TODO

1. Start with concat command migration (highest complexity)
2. Then schema command (lots of output)
3. Then config command (simple)
4. Then export commands (unknown complexity)
5. Finally cleanup and testing

Estimated remaining work: 2-3 hours
