# Import Module Refactoring - Phase 6: Final Cleanup

## Current State

**Warnings:** 5 remaining (extensibility APIs)
**Tests:** All passing
**Clippy:** Clean
**PatternProvider:** All 25 handlers now have proper implementations

## Remaining Warnings

These are extensibility APIs that may be addressed or documented:

1. `build_default_configs()` - ObjectTypeConfig static method
2. `attachment_registry_mut()` - HandlerRegistry method
3. `parent_types` on AttachmentRule - field
4. `register()` / `get_rules()` on AttachmentRegistry
5. `with_metadata()` / `with_extracted_dep()` on RawObject

## Options

### Option A: Document as Intentional API Surface

Add doc comments explaining these are extensibility points for:
- Custom handlers in the future
- Dynamic attachment rules
- RawObject builder patterns

### Option B: Remove Unused Code

If these truly won't be used:
- Delete the dead code
- Simplify the APIs

### Option C: Wire Up Usage

Find legitimate uses for these APIs:
- Use `build_default_configs()` in orchestrator initialization
- Use attachment registry methods in attachment processing
- Use RawObject builder methods in parser

## Success Criteria

1. **Decide on remaining warnings**: Document, remove, or use them
2. **All tests pass**: `cargo test --lib --tests`
3. **Clippy clean**: `cargo clippy --all-targets`
4. **No allow directives**: `grep -r "#![allow(" src/commands/import/`
5. **Update docs**: Mark Phase 6 complete in import-refactoring-plan.md

## Quick Reference

```bash
# Check current warnings
cargo build 2>&1 | grep -E "never (used|read)"

# Run tests
cargo test --lib --tests

# Run clippy
cargo clippy --all-targets

# Search for allow directives
grep -r "#!\[allow(" src/commands/import/
```
