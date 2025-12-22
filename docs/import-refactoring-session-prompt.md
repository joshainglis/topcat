# Import Module Refactoring - Phase 4 Session Prompt

## Context

You are continuing a multi-session refactoring of `src/commands/import/`. Read the full plan: `docs/import-refactoring-plan.md`

## Completed

- **Phase 1:** Infrastructure (sources/, handlers/, output/ modules)
- **Phase 2:** All handler categories migrated (10 categories, 45 handlers total)
- **Phase 3:** Parser refactoring complete
  - Created `PgDumpParser` implementing `ImportSource` trait
  - Created `ImportOrchestrator` for file writing
  - Wired up `pg_dump.rs` to use new modules (simplified from 1,295 to 132 lines)
  - All 9 import CLI tests pass

## Current Goal: Phase 4 - Integrate Handlers into Orchestrator

**Objective:** The `ImportOrchestrator` currently implements categorization, rendering, and dependency logic directly. Refactor it to delegate to the handler system that was scaffolded in Phase 2.

### Current Problem

The orchestrator has hardcoded logic that should live in handlers:

```rust
// In orchestrator.rs - this should delegate to handlers
fn categorize_object(&self, obj: &RawObject) -> String {
    match obj.obj_type {
        ObjectType::Type => { /* hardcoded */ }
        ObjectType::Function => { /* hardcoded */ }
        // ... 20+ cases
    }
}
```

Meanwhile, handlers already implement these traits but aren't used:
- `Categorizer::output_path()` - determines file location
- `Renderer::render()` - renders object with attachments
- `DependencyExtractor::extract_pattern_dependencies()` - extracts deps
- `Configurable::layer()` - determines layer assignment

### Architecture Goal

```
ImportOrchestrator
    ↓ looks up handler via
HandlerRegistry.get(obj_type)
    ↓ delegates to
Handler (implements Categorizer, Renderer, etc.)
    ↓ returns
Categorization, rendered content, dependencies
```

### Tasks

#### 4a. Update HandlerRegistry to be usable

The registry exists but needs to be instantiated and used. Currently it's defined but never created.

**File:** `handlers/registry.rs`

**Changes needed:**
1. Add a `new()` constructor that registers all handlers
2. Ensure `get()` returns a usable handler reference
3. The registry should be created once and passed to the orchestrator

```rust
impl HandlerRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
            attachment_registry: AttachmentRegistry::new(),
        };
        registry.register_all_handlers();
        registry
    }

    fn register_all_handlers(&mut self) {
        // Register handlers from each category
        schema_objects::register_handlers(self);
        types::register_handlers(self);
        routines::register_handlers(self);
        // ... etc
    }
}
```

#### 4b. Implement register_handlers for each category

Each handler category module needs a `register_handlers` function.

**Example for `handlers/schema_objects/mod.rs`:**
```rust
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(ObjectType::Table, TableHandler);
    registry.register(ObjectType::View, ViewHandler);
    registry.register(ObjectType::Schema, SchemaHandler);
    // ... etc
}
```

Do this for all 10 categories:
- `schema_objects/mod.rs`
- `types/mod.rs`
- `routines/mod.rs`
- `attachments/mod.rs`
- `fts/mod.rs`
- `fdw/mod.rs`
- `operators/mod.rs`
- `replication/mod.rs`
- `security/mod.rs`
- `global/mod.rs`

#### 4c. Update Orchestrator to use HandlerRegistry

**File:** `sources/pg_dump/orchestrator.rs`

**Changes:**
1. Add `HandlerRegistry` field to `ImportOrchestrator`
2. Create registry in constructor
3. Replace `categorize_object()` with handler delegation
4. Replace `categorize_function()` with handler delegation

```rust
pub struct ImportOrchestrator {
    config: OrchestratorConfig,
    logger: Logger,
    dep_analyzer: Option<DependencyAnalyzer>,
    handler_registry: HandlerRegistry,  // ADD THIS
    // ... existing fields
}

impl ImportOrchestrator {
    pub fn new(config: OrchestratorConfig, logger: Logger) -> Self {
        Self {
            handler_registry: HandlerRegistry::new(),
            // ... rest
        }
    }

    fn categorize_object(&self, obj: &RawObject) -> String {
        // Delegate to handler
        if let Some(handler) = self.handler_registry.get(&obj.obj_type) {
            handler.category()  // or output_path logic
        } else {
            // Fallback for unknown types
            "unknown".to_string()
        }
    }
}
```

#### 4d. Implement Categorizer trait properly on handlers

Each handler needs proper `Categorizer` implementation. Currently they may be stubs.

**Example for `handlers/schema_objects/table.rs`:**
```rust
impl Categorizer for TableHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Table
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir.join(schema).join("table").join(format!("{}.sql", obj.name))
    }
}
```

Review and fix implementations in:
- All 45 handlers across 10 categories

#### 4e. Integrate Renderer trait for file content

The orchestrator's `CollectedObject::render()` method should delegate to handlers.

**Changes:**
1. Handler's `Renderer::render()` takes the object and related attachments
2. Orchestrator calls handler to render instead of doing it inline

```rust
impl Renderer for TableHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];
        parts.push(related.render_all(config));
        parts.join("\n\n")
    }
}
```

#### 4f. Use AttachmentRegistry for parent resolution

Currently `find_parent_from_deps()` in orchestrator uses `extracted_deps`. The `AttachmentRegistry` can provide more sophisticated parent lookup.

**Changes:**
1. Register attachment rules in each attachment handler
2. Orchestrator uses `AttachmentRegistry::find_parent()`

```rust
// In handlers/attachments/trigger.rs
pub fn register_attachment_rules(registry: &mut AttachmentRegistry) {
    registry.register(ObjectType::Trigger, AttachmentRule {
        parent_types: vec![ObjectType::Function],
        extract_parent: Arc::new(|obj| {
            // Use extracted_deps or pattern matching
            obj.extracted_deps.first().map(|dep| ParentIdentity {
                schema: extract_schema(&dep.name),
                name: extract_name(&dep.name),
                obj_type: dep.dep_type,
            })
        }),
    });
}
```

#### 4g. Remove duplicate code from orchestrator

After delegation is working, remove the hardcoded logic from orchestrator:
- `categorize_object()` match statement
- `categorize_function()` method
- `handle_global_object()` categorization logic

These should all delegate to handlers.

#### 4h. Remove duplicate code from dependencies.rs

Remove the now-unused functions:
- `build_header()` - replaced by `output/header_builder.rs`
- `build_global_header()` - replaced by `output/header_builder.rs`

#### 4i. Remove ObjectTypeConfig from object_types.rs

The `ObjectTypeConfig` struct and `build_default_configs()` are unused now that handlers provide this via the `Configurable` trait.

#### 4j. Run tests and clippy

```bash
cargo test --test cli_import_tests
cargo test --lib --tests
cargo clippy --all-targets -- -D warnings
```

### Key Files to Modify

| File | Changes |
|------|---------|
| `handlers/registry.rs` | Add `new()`, implement registration |
| `handlers/*/mod.rs` | Add `register_handlers()` function |
| `handlers/*/[handler].rs` | Ensure traits are properly implemented |
| `orchestrator.rs` | Add registry, delegate to handlers |
| `dependencies.rs` | Remove duplicate `build_header` functions |
| `object_types.rs` | Remove `ObjectTypeConfig` |

### Handler Trait Checklist

For each of the 45 handlers, verify implementation of:

- [ ] `Categorizer::category()` - returns `ObjectCategory`
- [ ] `Categorizer::output_path()` - returns file path
- [ ] `Configurable::layer()` - returns `Layer`
- [ ] `Configurable::is_primary()` - returns bool
- [ ] `Renderer::render()` - renders object content

For attachment handlers (10 handlers), also verify:
- [ ] `register_attachment_rules()` - registers parent lookup

### Verification

After each major change:
```bash
cargo build
cargo test --test cli_import_tests
```

Final verification:
```bash
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
```

### Current Handler Categories

| Category | Handlers | File |
|----------|----------|------|
| schema_objects | 7 | table, view, materialized_view, foreign_table, sequence, schema, extension |
| types | 3 | type_handler, domain, collation |
| routines | 3 | function, procedure, aggregate |
| attachments | 10 | index, constraint, fk_constraint, trigger, policy, row_security, default, statistics, rule, comment |
| fts | 4 | configuration, dictionary, parser, template |
| fdw | 3 | wrapper, server, user_mapping |
| operators | 5 | operator, operator_class, operator_family, access_method, cast |
| replication | 2 | publication, subscription |
| security | 3 | acl, default_acl, security_label |
| global | 5 | language, event_trigger, transform, conversion, comment |

### After Phase 4

The import module will have:
- Fully integrated handler system
- Clean separation: parsing (PgDumpParser) → handlers (per-type logic) → orchestrator (coordination)
- Easy extensibility: add new object type by creating a handler
- No dead code warnings
