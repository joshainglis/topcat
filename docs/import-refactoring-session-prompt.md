# Import Module Refactoring - Complete Handler Integration

## Critical Directive

**DO NOT LEAVE SCAFFOLDING.** Every piece of infrastructure must be wired up and actively used. If code exists, it must serve a purpose. Dead code warnings are bugs, not "future work."

## Current State

The import module has a handler-based architecture that is partially integrated:

### Already Wired Up (Working)
1. **HandlerRegistry.get()** - Used to look up handlers by ObjectType
2. **AttachmentRegistry.find_parent()** - Used to find parent objects for attachments
3. **RegisteredHandler.extract_pattern_deps()** - Used to extract pattern-based dependencies
4. **AttachmentRegistry.is_attachment()** - Used to check if object is an attachment type

### NOT Wired Up (Must Be Implemented)

Run `cargo build 2>&1 | grep "never used\|never read"` to see current dead code.

Key unused infrastructure that MUST be wired up:

1. **Traits** (in `handlers/traits.rs`):
   - `Categorizer::category()` and `output_path()` - Should replace manual path construction
   - `Renderer` trait - Should be used for rendering objects
   - `Configurable` trait - Should provide type-specific configuration
   - `PatternProvider` trait - Should provide identification patterns
   - `ObjectHandler` trait - Composite trait combining all the above
   - `DependencyExtractor::implicit_dependency_types()` - Should enhance dependency analysis

2. **OutputConfig** (in `handlers/traits.rs`):
   - Has same fields as OrchestratorConfig (include_acl, include_owner, etc.)
   - Either consolidate with OrchestratorConfig or wire up properly

3. **ObjectTypeConfig** (in `object_types.rs`):
   - Fields: `skip`, `attach_to_parent`, `parent_types`, `subdirectory`
   - `build_default_configs()` method
   - Should drive type-specific behavior

4. **RegisteredHandler methods**:
   - `implicit_dep_types()` - Should be used in dependency analysis
   - `layer()`, `is_primary()`, `is_table_attachment()`, `is_global()` - Should replace direct ObjectType calls
   - `object_type()`, `default_config()` - Should be used for handler access

5. **HandlerRegistry methods**:
   - `has_handler()`, `registered_types()` - Useful for validation
   - `attachment_registry_mut()` - For dynamic rule registration

6. **AttachmentRegistry methods**:
   - `register()`, `get_rules()` - For dynamic rule management

7. **RelatedObjects** (in `handlers/traits.rs`):
   - `new()`, `is_empty()`, `count()` - For collecting ACLs, comments, etc.

8. **HeaderBuilder methods** (in `output/header_builder.rs`):
   - `layer()`, `require()`, `requires()` - Additional builder methods

9. **RawObject methods** (in `sources/raw_object.rs`):
   - `qualified_name()`, `is_primary()`, `is_table_attachment()`
   - `get_metadata()`, `with_metadata()`, `with_extracted_dep()`, `extracted_deps()`

10. **Unused regex patterns** (in `sources/pg_dump/patterns.rs`):
    - FTS_CONFIG_PATTERN, FTS_DICTIONARY_PATTERN, FDW_PATTERN
    - PUBLICATION_PATTERN, AGGREGATE_PATTERN, COLLATION_PATTERN
    - CONVERSION_PATTERN, LANGUAGE_PATTERN, MATERIALIZED_VIEW_PATTERN, SEQUENCE_PATTERN
    - Either use these or remove them

11. **Other unused methods**:
    - `source_name()` on ImportSource trait
    - `qualified_target()` on SecurityStatement
    - `has_default_acl()` on PgDumpParser
    - `is_empty()` on CollectedObject

## Implementation Strategy

### Option A: Full Handler-Centric Architecture
Refactor the orchestrator to fully delegate to handlers:
1. Replace `categorize_object()` with `handler.category()` and `handler.output_path()`
2. Replace manual rendering with `Renderer::render()`
3. Use `Configurable::default_config()` for type-specific settings
4. Use trait bounds where appropriate

### Option B: Consolidate and Simplify
If the trait-based architecture is over-engineered:
1. Remove unused traits entirely
2. Keep the working parts (registry, attachment resolution, pattern extraction)
3. Delete the scaffolded but unused infrastructure
4. Simplify ObjectTypeConfig if not needed

### Recommended: Hybrid Approach
1. Wire up what adds value (implicit_dependency_types, output_path, etc.)
2. Remove what's truly redundant (duplicate patterns, unused traits)
3. Consolidate OutputConfig with OrchestratorConfig

## Files to Focus On

1. **`sources/pg_dump/orchestrator.rs`** - Main integration point
   - Uses `categorize_object()` - could use handler's Categorizer
   - Manually constructs paths - could use `output_path()`
   - Has its own config - could integrate with OutputConfig

2. **`handlers/registry.rs`** - Handler registration
   - `RegisteredHandler` has many unused methods
   - Wire them up or simplify

3. **`handlers/traits.rs`** - Trait definitions
   - Many traits are defined but not used as bounds
   - Either use them properly or remove

4. **`sources/pg_dump/patterns.rs`** - Regex patterns
   - Many patterns defined but never used
   - Either wire into parser/handlers or delete

5. **`object_types.rs`** - ObjectTypeConfig
   - `build_default_configs()` never called
   - Config fields never read
   - Either use or remove

## Success Criteria

1. **Zero dead code warnings** when running:
   ```bash
   cargo build 2>&1 | grep -E "never (used|read)"
   ```

2. **All tests pass**:
   ```bash
   cargo test --lib --tests
   ```

3. **Clippy clean** (no suppressed warnings needed):
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```

4. **No `#![allow(dead_code)]` or `#![allow(unused_imports)]`** in the import module

## Anti-Patterns to Avoid

- Adding `#[allow(dead_code)]` to silence warnings
- Saying "this is scaffolded for future use"
- Leaving methods that are "useful but not currently used"
- Creating infrastructure without immediate consumers

## Current `#![allow(...)]` That Must Be Removed

1. `src/commands/import/mod.rs` - `// #![allow(dead_code)]` (commented out)
2. `src/commands/import/handlers/mod.rs` - `#![allow(unused_imports)]`
3. `src/commands/import/handlers/attachments/mod.rs` - `#![allow(unused_imports)]`
4. `src/commands/import/handlers/fdw/mod.rs` - `#![allow(unused_imports)]`
5. `src/commands/import/handlers/fts/mod.rs` - `#![allow(unused_imports)]`
6. `src/commands/import/handlers/operators/mod.rs` - `#![allow(unused_imports)]`
7. `src/commands/import/handlers/routines/mod.rs` - `#![allow(unused_imports)]`
8. `src/commands/import/handlers/schema_objects/mod.rs` - `#![allow(unused_imports)]`
9. `src/commands/import/handlers/types/mod.rs` - `#![allow(unused_imports)]`

Each of these represents code that isn't being used. Fix it or delete it.

## What Was Done This Session

1. Wired up `HandlerRegistry.get()` for handler lookups
2. Wired up `AttachmentRegistry.find_parent()` and `is_attachment()` - replaced `find_parent_from_deps()`
3. Wired up `RegisteredHandler.extract_pattern_deps()` for pattern-based dependency extraction
4. Removed duplicate `find_parent_from_deps()` method
5. Updated test for function subcategorization

## Key Principle

If you write code, USE it. If you can't use it now, DON'T write it. The codebase should have zero speculative infrastructure.
