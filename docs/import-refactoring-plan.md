# Import Module Refactoring Plan

**Created:** 2025-11-27
**Completed:** 2025-12-22
**Status:** Complete
**Scope:** Refactor `src/commands/import/` into per-type submodules with composable traits

## Goals

1. Create a submodule per PostgreSQL object type, nested within category directories
2. Use composable traits for extensibility
3. Implement a layered architecture: ImportSource → RawObject → ObjectHandler
4. Use registry pattern for attachment relationships
5. Enable future database introspection support (not just pg_dump)

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Granularity | Per-type modules nested in categories | Balance between separation and organization |
| Trait design | Composable traits | Maximum flexibility, single responsibility |
| Import source | Layered approach | Decouples sources from handlers, easy to add new sources |
| Attachments | Registry pattern | Decoupled, both types contribute to relationship |
| Pattern ownership | Patterns owned by sources | Different sources (pg_dump versions, MySQL, introspection) need different patterns |
| Dependency extraction | Source populates `extracted_deps` on `RawObject` | Handlers remain source-agnostic, receive pre-extracted deps |

## Current State (as of Phase 3 in progress)

```
src/commands/import/
├── mod.rs                 (64 lines)    - Module entry point and CLI dispatch
├── object_types.rs        (828 lines)   - PostgreSQL object type definitions
├── dependencies.rs        (375 lines)   - Dependency analysis
├── pg_dump.rs            (1,295 lines)  - Main parser and file writer (to be refactored)
│
├── sources/                             - NEW: Import sources
│   ├── mod.rs
│   ├── raw_object.rs                    - RawObject with extracted_deps
│   ├── traits.rs                        - ImportSource trait
│   └── pg_dump/
│       ├── mod.rs
│       └── patterns.rs   (592 lines)    - Moved from import/patterns.rs
│
├── handlers/                            - NEW: 45 type handlers in 10 categories
│   ├── mod.rs, traits.rs, registry.rs
│   ├── schema_objects/ (7), types/ (3), routines/ (3)
│   ├── attachments/ (10), fts/ (4), fdw/ (3)
│   ├── operators/ (5), replication/ (2), security/ (3), global/ (5)
│
└── output/                              - NEW: File writing infrastructure
    ├── mod.rs
    ├── header_builder.rs
    └── file_writer.rs
```

## Target Directory Structure

```
src/commands/import/
├── mod.rs                          # CLI dispatch, re-exports
│
├── sources/                        # Layer 1: Import sources
│   ├── mod.rs                      # Re-exports, ImportSource dispatch
│   ├── traits.rs                   # ImportSource trait definition
│   ├── raw_object.rs               # RawObject intermediate representation
│   └── pg_dump/                    # pg_dump parser implementation
│       ├── mod.rs
│       ├── parser.rs               # Main parsing logic (sequential text processing)
│       └── patterns.rs             # All regex patterns (moved from current patterns.rs)
│
├── handlers/                       # Layer 3: Type-specific handlers
│   ├── mod.rs                      # Handler registry initialization, dispatch
│   ├── traits.rs                   # Composable traits
│   ├── registry.rs                 # Handler + attachment registries
│   │
│   ├── schema_objects/             # Category: core schema objects
│   │   ├── mod.rs
│   │   ├── schema.rs
│   │   ├── extension.rs
│   │   ├── table.rs
│   │   ├── view.rs
│   │   ├── materialized_view.rs
│   │   ├── foreign_table.rs
│   │   └── sequence.rs
│   │
│   ├── types/                      # Category: type system
│   │   ├── mod.rs
│   │   ├── type_handler.rs         # Type with subcategories (enum, composite, etc.)
│   │   ├── domain.rs
│   │   └── collation.rs
│   │
│   ├── routines/                   # Category: callable objects
│   │   ├── mod.rs
│   │   ├── function.rs
│   │   ├── procedure.rs
│   │   └── aggregate.rs
│   │
│   ├── attachments/                # Category: table/object attachments
│   │   ├── mod.rs
│   │   ├── index.rs
│   │   ├── constraint.rs
│   │   ├── fk_constraint.rs
│   │   ├── trigger.rs
│   │   ├── policy.rs
│   │   ├── row_security.rs
│   │   ├── default.rs
│   │   ├── statistics.rs
│   │   └── rule.rs
│   │
│   ├── fts/                        # Category: full-text search
│   │   ├── mod.rs
│   │   ├── configuration.rs
│   │   ├── dictionary.rs
│   │   ├── parser.rs
│   │   └── template.rs
│   │
│   ├── fdw/                        # Category: foreign data
│   │   ├── mod.rs
│   │   ├── wrapper.rs
│   │   ├── server.rs
│   │   └── user_mapping.rs
│   │
│   ├── operators/                  # Category: operators and casts
│   │   ├── mod.rs
│   │   ├── operator.rs
│   │   ├── operator_class.rs
│   │   ├── operator_family.rs
│   │   ├── access_method.rs
│   │   └── cast.rs
│   │
│   ├── replication/                # Category: logical replication
│   │   ├── mod.rs
│   │   ├── publication.rs
│   │   └── subscription.rs
│   │
│   ├── security/                   # Category: security objects
│   │   ├── mod.rs
│   │   ├── acl.rs
│   │   ├── default_acl.rs
│   │   └── security_label.rs
│   │
│   └── global/                     # Category: database-level objects
│       ├── mod.rs
│       ├── language.rs
│       ├── event_trigger.rs
│       ├── transform.rs
│       ├── conversion.rs
│       └── comment.rs
│
├── output/                         # File writing logic
│   ├── mod.rs
│   ├── file_writer.rs              # Writes objects to files
│   └── header_builder.rs           # Builds topcat headers
│
└── config.rs                       # ObjectTypeConfig, import settings
```

## Core Abstractions

### 1. Import Source Trait (`sources/traits.rs`)

```rust
use crate::commands::import::sources::RawObject;
use anyhow::Result;

/// Trait for import sources (pg_dump files, database introspection, etc.)
pub trait ImportSource {
    /// Extract all objects from the source
    fn extract_objects(&mut self) -> Result<Vec<RawObject>>;

    /// Collect security statements (GRANT/REVOKE/OWNER)
    fn collect_security(&mut self) -> Result<Vec<SecurityStatement>>;

    /// Source-specific metadata
    fn source_name(&self) -> &'static str;
}

pub struct SecurityStatement {
    pub kind: SecurityKind,
    pub target_schema: Option<String>,
    pub target_name: String,
    pub target_type: Option<ObjectType>,
    pub content: String,
}

pub enum SecurityKind {
    Grant,
    Revoke,
    Owner,
}
```

### 2. Intermediate Representation (`sources/raw_object.rs`)

```rust
use std::collections::HashMap;

/// A dependency extracted by the source using source-specific patterns.
///
/// This allows sources to extract structural dependencies during parsing
/// (e.g., trigger → function relationships) without handlers needing to
/// know about source-specific patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedDep {
    /// The name of the dependency (qualified or unqualified).
    pub name: String,
    /// The type of the dependency.
    pub dep_type: ObjectType,
}

/// Intermediate representation produced by ImportSource, consumed by handlers
#[derive(Debug, Clone)]
pub struct RawObject {
    /// The PostgreSQL object type
    pub obj_type: ObjectType,

    /// Schema name (None for global objects like extensions, casts)
    pub schema: Option<String>,

    /// Object name
    pub name: String,

    /// Full identity for overloaded objects (e.g., function signatures)
    pub identity: String,

    /// SQL content
    pub content: String,

    /// Owner if specified
    pub owner: Option<String>,

    /// Target type for secondary objects (e.g., "COLUMN" for DEFAULT)
    pub target_type: Option<String>,

    /// Source-specific metadata (flexible for different sources)
    pub source_metadata: HashMap<String, String>,

    /// Dependencies extracted by the source using source-specific patterns.
    ///
    /// Sources populate this during parsing by analyzing the SQL content with
    /// their own patterns. This decouples handlers from source-specific pattern
    /// knowledge - handlers just read the pre-extracted dependencies.
    ///
    /// Examples:
    /// - Trigger → Function (extracted from EXECUTE FUNCTION clause)
    /// - Foreign Table → Server (extracted from SERVER clause)
    /// - Subscription → Publication (extracted from PUBLICATION clause)
    pub extracted_deps: Vec<ExtractedDep>,
}

impl RawObject {
    pub fn qualified_name(&self) -> String {
        match &self.schema {
            Some(s) => format!("{}.{}", s, self.name),
            None => self.name.clone(),
        }
    }

    pub fn is_global(&self) -> bool {
        self.schema.is_none()
    }
}
```

### 3. Composable Handler Traits (`handlers/traits.rs`)

```rust
use crate::commands::import::sources::RawObject;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::PathBuf;

/// Provides regex patterns for pg_dump parsing
/// Implemented by types that can be identified via regex in pg_dump output
pub trait PatternProvider {
    /// Returns regex patterns that identify this object type in content
    fn content_patterns() -> Vec<&'static Lazy<Regex>> {
        vec![]
    }
}

/// Extracts dependencies from object content
pub trait DependencyExtractor {
    /// Extract dependencies using pattern matching on content
    /// Returns (dependency_name, dependency_type) pairs
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        vec![]
    }

    /// Object types this type commonly depends on (for filtering SQL analysis)
    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![]
    }
}

/// Categorizes object for output organization
pub trait Categorizer {
    /// Primary category for output directory structure
    fn category() -> ObjectCategory;

    /// Optional subcategory based on content analysis
    fn subcategory(_content: &str) -> Option<String> {
        None
    }

    /// Compute output path for this object
    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf;
}

/// Renders object to final SQL output
pub trait Renderer {
    /// Render the object with its related objects to SQL
    fn render(
        obj: &RawObject,
        related: &RelatedObjects,
        config: &OutputConfig,
    ) -> String;
}

/// Default configuration for this object type
pub trait Configurable {
    /// Default ObjectTypeConfig
    fn default_config() -> ObjectTypeConfig;

    /// Dependency layer (Prepend, Normal, Append)
    fn layer() -> Layer;

    /// Whether this is a primary (standalone) object type
    fn is_primary() -> bool {
        true
    }
}

/// Combined trait for full object handling
pub trait ObjectHandler:
    PatternProvider + DependencyExtractor + Categorizer + Renderer + Configurable + Send + Sync
{
    fn object_type() -> ObjectType;
}
```

### 4. Registry Pattern (`handlers/registry.rs`)

```rust
use std::collections::HashMap;
use std::sync::Arc;

/// Central registry for all object handlers
pub struct HandlerRegistry {
    handlers: HashMap<ObjectType, Arc<dyn ObjectHandler>>,
    attachment_registry: AttachmentRegistry,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
            attachment_registry: AttachmentRegistry::new(),
        };
        registry.register_all_handlers();
        registry
    }

    pub fn get_handler(&self, obj_type: ObjectType) -> Option<&Arc<dyn ObjectHandler>> {
        self.handlers.get(&obj_type)
    }

    pub fn attachment_registry(&self) -> &AttachmentRegistry {
        &self.attachment_registry
    }

    fn register_all_handlers(&mut self) {
        // Register all handlers from each category module
        schema_objects::register_handlers(self);
        types::register_handlers(self);
        routines::register_handlers(self);
        attachments::register_handlers(self);
        fts::register_handlers(self);
        fdw::register_handlers(self);
        operators::register_handlers(self);
        replication::register_handlers(self);
        security::register_handlers(self);
        global::register_handlers(self);
    }

    pub fn register(&mut self, handler: Arc<dyn ObjectHandler>) {
        let obj_type = handler.object_type();
        self.handlers.insert(obj_type, handler);
    }
}

/// Registry for attachment relationships
pub struct AttachmentRegistry {
    /// Maps attachment type → rules for finding parent
    rules: HashMap<ObjectType, Vec<AttachmentRule>>,
}

pub struct AttachmentRule {
    /// Possible parent types for this attachment
    pub parent_types: Vec<ObjectType>,

    /// Function to extract parent identity from attachment content
    pub extract_parent: Box<dyn Fn(&RawObject) -> Option<ParentIdentity> + Send + Sync>,
}

pub struct ParentIdentity {
    pub schema: Option<String>,
    pub name: String,
    pub obj_type: ObjectType,
}

impl AttachmentRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            rules: HashMap::new(),
        };
        registry.register_attachment_rules();
        registry
    }

    pub fn find_parent(&self, attachment: &RawObject) -> Option<ParentIdentity> {
        let rules = self.rules.get(&attachment.obj_type)?;
        for rule in rules {
            if let Some(parent) = (rule.extract_parent)(attachment) {
                return Some(parent);
            }
        }
        None
    }

    fn register_attachment_rules(&mut self) {
        // Index → Table
        self.rules.insert(ObjectType::Index, vec![
            AttachmentRule {
                parent_types: vec![ObjectType::Table],
                extract_parent: Box::new(|obj| {
                    // Extract table name from CREATE INDEX ... ON table_name
                    extract_index_table(obj)
                }),
            },
        ]);

        // Trigger → Function (primary), Table (secondary)
        self.rules.insert(ObjectType::Trigger, vec![
            AttachmentRule {
                parent_types: vec![ObjectType::Function],
                extract_parent: Box::new(|obj| {
                    extract_trigger_function(obj)
                }),
            },
        ]);

        // ... more attachment rules
    }
}
```

### 5. Related Objects Container

```rust
/// Container for objects that attach to a primary object
#[derive(Debug, Default)]
pub struct RelatedObjects {
    pub indexes: Vec<RawObject>,
    pub constraints: Vec<RawObject>,
    pub fk_constraints: Vec<RawObject>,
    pub triggers: Vec<RawObject>,
    pub policies: Vec<RawObject>,
    pub row_security: Vec<RawObject>,
    pub defaults: Vec<RawObject>,
    pub statistics: Vec<RawObject>,
    pub rules: Vec<RawObject>,
    pub comments: Vec<RawObject>,
    pub acl: Vec<String>,
    pub owner: Option<String>,
}

impl RelatedObjects {
    /// Render all related objects in correct order
    pub fn render_all(&self, config: &OutputConfig) -> String {
        let mut parts = vec![];

        // Order matters for dependencies
        for seq in &self.sequences { parts.push(seq.content.clone()); }
        for idx in &self.indexes { parts.push(idx.content.clone()); }
        for con in &self.constraints { parts.push(con.content.clone()); }
        for fk in &self.fk_constraints { parts.push(fk.content.clone()); }
        for trig in &self.triggers { parts.push(trig.content.clone()); }
        // ... etc

        parts.join("\n\n")
    }
}
```

## Example Type Module Implementation

### `handlers/schema_objects/table.rs`

```rust
use crate::commands::import::{
    handlers::traits::*,
    sources::RawObject,
    config::{ObjectTypeConfig, Layer, ObjectCategory},
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};

/// Handler for PostgreSQL TABLE objects
pub struct TableHandler;

impl PatternProvider for TableHandler {
    fn content_patterns() -> Vec<&'static Lazy<Regex>> {
        vec![&ALTER_TABLE_PATTERN]
    }
}

impl DependencyExtractor for TableHandler {
    fn extract_pattern_dependencies(_content: &str) -> Vec<(String, ObjectType)> {
        // Tables don't have pattern-based structural dependencies
        // (foreign keys are separate FK_CONSTRAINT objects)
        vec![]
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![
            ObjectType::Type,
            ObjectType::Domain,
            ObjectType::Schema,
            ObjectType::Collation,
        ]
    }
}

impl Categorizer for TableHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Table
    }

    fn subcategory(_content: &str) -> Option<String> {
        None  // Tables don't have subcategories
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("table")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for TableHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig {
            skip: false,
            attach_to_parent: false,
            parent_types: vec![],
            subdirectory: None,
        }
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for TableHandler {
    fn render(
        obj: &RawObject,
        related: &RelatedObjects,
        config: &OutputConfig,
    ) -> String {
        let mut parts = vec![obj.content.clone()];

        // Append related objects in dependency order
        parts.push(related.render_all(config));

        parts.into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

impl ObjectHandler for TableHandler {
    fn object_type() -> ObjectType {
        ObjectType::Table
    }
}

// Pattern used by this handler
pub static ALTER_TABLE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)ALTER\s+TABLE\s+(?:ONLY\s+)?(?:IF\s+EXISTS\s+)?(?:\"?(\w+)\"?\.)?\"?(\w+)\"?")
        .unwrap()
});

/// Register this handler with the registry
pub fn register(registry: &mut HandlerRegistry) {
    registry.register(Arc::new(TableHandler));
}
```

### `handlers/attachments/trigger.rs`

```rust
use crate::commands::import::{
    handlers::{traits::*, registry::*},
    sources::RawObject,
    config::{ObjectTypeConfig, Layer, ObjectCategory},
};
use once_cell::sync::Lazy;
use regex::Regex;

pub struct TriggerHandler;

impl PatternProvider for TriggerHandler {
    fn content_patterns() -> Vec<&'static Lazy<Regex>> {
        vec![&TRIGGER_PATTERN]
    }
}

impl DependencyExtractor for TriggerHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Extract function dependency from EXECUTE FUNCTION/PROCEDURE
        if let Some(caps) = TRIGGER_PATTERN.captures(content) {
            if let Some(func_name) = caps.name("function") {
                let schema = caps.name("func_schema").map(|m| m.as_str().to_string());
                let qualified = match schema {
                    Some(s) => format!("{}.{}", s, func_name.as_str()),
                    None => func_name.as_str().to_string(),
                };
                deps.push((qualified, ObjectType::Function));
            }
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Function, ObjectType::Table]
    }
}

impl Categorizer for TriggerHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Trigger
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Triggers are attached to functions, so this is fallback only
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("trigger")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for TriggerHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig {
            skip: false,
            attach_to_parent: true,
            parent_types: vec![ObjectType::Function],
            subdirectory: None,
        }
    }

    fn layer() -> Layer {
        Layer::Append  // Triggers created after tables and functions
    }

    fn is_primary() -> bool {
        false  // Attachment type
    }
}

impl Renderer for TriggerHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

impl ObjectHandler for TriggerHandler {
    fn object_type() -> ObjectType {
        ObjectType::Trigger
    }
}

pub static TRIGGER_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?ixs)
        CREATE\s+(?:CONSTRAINT\s+)?TRIGGER\s+
        (?:\"?(?P<name>\w+)\"?)\s+
        (?:BEFORE|AFTER|INSTEAD\s+OF)\s+
        .*?
        ON\s+(?:\"?(?P<schema>\w+)\"?\.)?\"?(?P<table>\w+)\"?
        .*?
        EXECUTE\s+(?:FUNCTION|PROCEDURE)\s+
        (?:\"?(?P<func_schema>\w+)\"?\.)?\"?(?P<function>\w+)\"?
    ").unwrap()
});

/// Register attachment rule for triggers
pub fn register_attachment(registry: &mut AttachmentRegistry) {
    registry.register(ObjectType::Trigger, AttachmentRule {
        parent_types: vec![ObjectType::Function],
        extract_parent: Box::new(|obj| {
            TRIGGER_PATTERN.captures(&obj.content).and_then(|caps| {
                let func_name = caps.name("function")?.as_str().to_string();
                let schema = caps.name("func_schema").map(|m| m.as_str().to_string());
                Some(ParentIdentity {
                    schema,
                    name: func_name,
                    obj_type: ObjectType::Function,
                })
            })
        }),
    });
}

pub fn register(registry: &mut HandlerRegistry) {
    registry.register(Arc::new(TriggerHandler));
}
```

## Migration Phases

### Phase 1: Create Infrastructure (No Breaking Changes)

**Goal:** Set up new directory structure and abstractions without changing behavior.

**Tasks:**
- [ ] Create `sources/mod.rs` with module declarations
- [ ] Create `sources/traits.rs` with `ImportSource` trait
- [ ] Create `sources/raw_object.rs` with `RawObject` struct
- [ ] Create `handlers/mod.rs` with module declarations
- [ ] Create `handlers/traits.rs` with composable traits
- [ ] Create `handlers/registry.rs` with registry pattern
- [ ] Create `output/mod.rs` with module declarations
- [ ] Create `output/header_builder.rs` (extract from dependencies.rs)
- [ ] Create `output/file_writer.rs` (extract from pg_dump.rs)
- [ ] Create `config.rs` (extract ObjectTypeConfig from object_types.rs)

**Verification:** Existing tests still pass, no functionality change.

### Phase 2: Migrate Type Handlers

**Goal:** Create per-type modules, moving logic from existing files.

Each category is independent and can be done in parallel:

#### 2a. Schema Objects ✅
- [x] `handlers/schema_objects/mod.rs`
- [x] `handlers/schema_objects/schema.rs`
- [x] `handlers/schema_objects/extension.rs`
- [x] `handlers/schema_objects/table.rs`
- [x] `handlers/schema_objects/view.rs`
- [x] `handlers/schema_objects/materialized_view.rs`
- [x] `handlers/schema_objects/foreign_table.rs`
- [x] `handlers/schema_objects/sequence.rs`

#### 2b. Types ✅
- [x] `handlers/types/mod.rs`
- [x] `handlers/types/type_handler.rs` (with subcategory logic)
- [x] `handlers/types/domain.rs`
- [x] `handlers/types/collation.rs`

#### 2c. Routines ✅
- [x] `handlers/routines/mod.rs`
- [x] `handlers/routines/function.rs` (with categorization logic)
- [x] `handlers/routines/procedure.rs`
- [x] `handlers/routines/aggregate.rs`

#### 2d. Attachments ✅
- [x] `handlers/attachments/mod.rs`
- [x] `handlers/attachments/index.rs`
- [x] `handlers/attachments/constraint.rs`
- [x] `handlers/attachments/fk_constraint.rs`
- [x] `handlers/attachments/trigger.rs`
- [x] `handlers/attachments/policy.rs`
- [x] `handlers/attachments/row_security.rs`
- [x] `handlers/attachments/default.rs`
- [x] `handlers/attachments/statistics.rs`
- [x] `handlers/attachments/rule.rs`

#### 2e. Full-Text Search ✅
- [x] `handlers/fts/mod.rs`
- [x] `handlers/fts/configuration.rs`
- [x] `handlers/fts/dictionary.rs`
- [x] `handlers/fts/parser.rs`
- [x] `handlers/fts/template.rs`

#### 2f. Foreign Data ✅
- [x] `handlers/fdw/mod.rs`
- [x] `handlers/fdw/wrapper.rs`
- [x] `handlers/fdw/server.rs`
- [x] `handlers/fdw/user_mapping.rs`

#### 2g. Operators ✅
- [x] `handlers/operators/mod.rs`
- [x] `handlers/operators/operator.rs`
- [x] `handlers/operators/operator_class.rs`
- [x] `handlers/operators/operator_family.rs`
- [x] `handlers/operators/access_method.rs`
- [x] `handlers/operators/cast.rs`

#### 2h. Replication ✅
- [x] `handlers/replication/mod.rs`
- [x] `handlers/replication/publication.rs`
- [x] `handlers/replication/subscription.rs`

#### 2i. Security ✅
- [x] `handlers/security/mod.rs`
- [x] `handlers/security/acl.rs`
- [x] `handlers/security/default_acl.rs`
- [x] `handlers/security/security_label.rs`

#### 2j. Global Objects ✅
- [x] `handlers/global/mod.rs`
- [x] `handlers/global/language.rs`
- [x] `handlers/global/event_trigger.rs`
- [x] `handlers/global/transform.rs`
- [x] `handlers/global/conversion.rs`
- [x] `handlers/global/comment.rs`

**Verification:** Each category's tests pass after migration.

### Phase 3: Refactor pg_dump Parser

**Goal:** Update pg_dump parser to use new abstractions. Separate parsing from file writing.

**Key architectural change:** Patterns are source-specific. The pg_dump parser owns its patterns and populates `RawObject.extracted_deps` during parsing. Handlers read pre-extracted dependencies rather than calling patterns directly.

**Completed Tasks:**
- [x] Add `ExtractedDep` struct and `extracted_deps` field to `RawObject`
- [x] Create `sources/pg_dump/` directory structure
- [x] Move `patterns.rs` to `sources/pg_dump/patterns.rs`
- [x] Update all pattern imports across handlers (17 files)

**Completed Tasks (Phase 3d-3f):**
- [x] Created `sources/pg_dump/parser.rs` implementing `ImportSource` trait
- [x] Parser produces `Vec<RawObject>` with `extracted_deps` populated
- [x] Populates `extracted_deps` during parsing for:
  - Trigger → Function (EXECUTE FUNCTION)
  - ForeignTable → Server (SERVER clause)
  - Subscription → Publication (PUBLICATION clause)
  - Server → ForeignDataWrapper (FOREIGN DATA WRAPPER)
  - UserMapping → Server (SERVER clause)
  - EventTrigger → Function (EXECUTE FUNCTION)
  - Statistics → Table (FROM table)
  - Rule → Table (ON table)
  - Index/Constraint/Policy → Table (ALTER TABLE pattern)
  - Cast → Type (to type)
  - Operator → Function (FUNCTION = clause)
- [x] Created `sources/pg_dump/orchestrator.rs` for file writing
  - Uses `extracted_deps` to resolve parent relationships
  - Uses `output/header_builder.rs` for header generation
  - Handles global objects, security statements, default ACL

**Completed Tasks (Phase 3g):**
- [x] Wire up `pg_dump.rs` to use new parser + orchestrator
- [x] Simplified `pg_dump.rs` from 1,295 lines to 132 lines (CLI args + execute())
- [x] All 9 import CLI tests pass

**Remaining for Phase 4 (cleanup):**
- [ ] Remove unused code (dead code warnings)
- [ ] Update handlers to read from `RawObject.extracted_deps` (or remove unused traits)
- [ ] Remove `extract_pattern_dependencies` from handler traits (or make it a fallback)

**Current Directory Structure:**
```
src/commands/import/
├── sources/
│   ├── mod.rs              # Exports pg_dump, RawObject, ExtractedDep, SecurityStatement
│   ├── raw_object.rs       # RawObject with extracted_deps field
│   ├── traits.rs           # ImportSource trait, SecurityStatement
│   └── pg_dump/
│       ├── mod.rs
│       ├── parser.rs       # NEW: PgDumpParser implementing ImportSource
│       ├── orchestrator.rs # NEW: ImportOrchestrator for file writing
│       └── patterns.rs     # Moved from import/patterns.rs
├── handlers/               # Complete (imports updated)
├── output/
│   ├── mod.rs
│   └── header_builder.rs   # HeaderBuilder, build_header, build_global_header
└── pg_dump.rs              # Original parser + file writer (still working)
```

**Verification:** Full integration tests pass (9 import CLI tests currently passing).

### Architecture Note: Pattern Ownership and extracted_deps

**Problem:** Handlers were calling source-specific patterns directly (e.g., `TRIGGER_PATTERN.captures(content)`), which would break if we add different import sources (database introspection, MySQL dumps, etc.).

**Solution:** Sources own their patterns and extract dependencies during parsing:

1. Each source (pg_dump, future introspection) has its own patterns
2. During parsing, the source uses its patterns to populate `RawObject.extracted_deps`
3. Handlers read from `extracted_deps` instead of calling patterns
4. Handlers become source-agnostic

**Migration path:**
- Currently: Handlers still call patterns (transitional state)
- Phase 3 remaining: Parser populates `extracted_deps`
- Phase 4: Remove `extract_pattern_dependencies` from handlers or make it fallback-only

**Benefits:**
- pg_dump v14 vs v10 can have different patterns
- Database introspection won't use regex at all (SQL queries)
- Handlers test against `RawObject.extracted_deps`, not pattern matching

### Phase 4: Handler Integration ✅ (Mostly Complete)

**Goal:** Wire up all handler traits and infrastructure. Eliminate dead code by USING it.

**Completed Tasks (2025-12-22):**
- [x] Consolidated `OutputConfig` with `OrchestratorConfig` via `to_output_config()`
- [x] Implemented `PatternProvider` for handlers with patterns (trigger, materialized_view, sequence, aggregate, collation, conversion, language, fts_config, fts_dictionary, fdw_wrapper, publication)
- [x] Wired up `validate_handler_traits()` to use all trait-bound helper functions
- [x] Used `ObjectHandler::object_type()` on TableHandler and TriggerHandler
- [x] Used `OutputConfig::new()` and `dry_run()`
- [x] Used `RelatedObjects::new()`, `is_empty()`, `count()`, `render_all()`
- [x] Used `HeaderBuilder` with `layer()`, `require()`, `requires()` chainable methods
- [x] Used `build_header()` and `build_global_header()` convenience functions
- [x] Used `RawObject::qualified_name()`, `is_global()`, `is_primary()`, `is_table_attachment()`, `extracted_deps()`, `get_metadata()`
- [x] Used `ParentIdentity::qualified_name()` for logging
- [x] Used `SecurityStatement::qualified_target()` for logging
- [x] Used `parser.source_name()` and `parser.has_default_acl()`
- [x] Used `handler_registry.registered_types()` and `has_handler()`
- [x] Used `ObjectTypeConfig.parent_types` and `subdirectory` for logging
- [x] Used `CollectedObject::is_empty()` for ACL attachment
- [x] Used `Categorizer::subcategory()` via `FunctionHandler`
- [x] Removed all `#![allow(unused_imports)]` from 7 handler modules
- [x] Cleaned up unused handler exports (kept only TriggerHandler, TableHandler, FunctionHandler)
- [x] Added `ObjectHandler` impl to TableHandler and TriggerHandler

**Warning Count:** 33 → 5 warnings

**Remaining Warnings (5):**
1. `build_default_configs()` on ObjectTypeConfig - static method for building config map
2. `attachment_registry_mut()` on HandlerRegistry - mutable accessor for dynamic registration
3. `parent_types` on AttachmentRule - field for parent type specification
4. `register()` and `get_rules()` on AttachmentRegistry - dynamic rule registration
5. `with_metadata()` and `with_extracted_dep()` on RawObject - builder pattern methods

These represent API extensibility points that are designed for future use (e.g., dynamic attachment rule registration, custom metadata).

### Phase 5: Pattern Provider Completion ✅

**Goal:** Every handler must have a proper `PatternProvider` implementation.

**Completed (2025-12-22):**
- [x] Created 17 new patterns in `sources/pg_dump/patterns.rs`:
  - `SCHEMA_PATTERN`, `TABLE_PATTERN`, `VIEW_PATTERN`
  - `FUNCTION_PATTERN`, `PROCEDURE_PATTERN`
  - `TYPE_PATTERN`, `DOMAIN_PATTERN`
  - `FTS_PARSER_PATTERN`, `FTS_TEMPLATE_PATTERN`
  - `INDEX_PATTERN`, `CONSTRAINT_PATTERN`, `FK_CONSTRAINT_PATTERN`
  - `DEFAULT_PATTERN`, `POLICY_PATTERN`, `ROW_SECURITY_PATTERN`
  - `CAST_PATTERN`, `OPERATOR_PATTERN`
- [x] Wired 8 handlers with existing patterns
- [x] Wired 17 handlers with new patterns
- [x] Removed all "identified by metadata" comments (25 total)
- [x] All tests pass
- [x] Clippy clean (5 remaining warnings for extensibility APIs)

**All 25 handlers now have proper `PatternProvider::content_patterns()` implementations.**

### Phase 6: Final Cleanup ✅

**Goal:** Remove any remaining dead code.

**Completed Tasks (2025-12-22):**
- [x] All patterns verified as actively used (47 patterns, no dead code)
- [x] Removed `subdirectory` field from `ObjectTypeConfig`
- [x] Removed `build_default_configs()` from `ObjectTypeConfig`
- [x] Removed `attachment_registry_mut()` from `HandlerRegistry`
- [x] Removed `register()` and `get_rules()` from `AttachmentRegistry`
- [x] Removed `parent_types` field from `AttachmentRule`
- [x] Removed `with_metadata()` and `with_extracted_dep()` from `RawObject`
- [x] Updated `deps_rule()` helper (no longer needs parent_type parameter)
- [x] Removed subdirectory logging from orchestrator.rs
- [x] All tests pass (540 tests)
- [x] Clippy clean (0 warnings)

**Verification:** All tests pass, clippy clean, no `#![allow(...)]` directives.

## Testing Strategy

### Unit Tests
- Each handler module should have unit tests for:
  - Pattern matching
  - Dependency extraction
  - Categorization
  - Rendering

### Integration Tests
- Existing `tests/` pg_dump import tests should continue to pass
- Add new tests for registry functionality
- Add tests for attachment resolution

### Test Locations
```
tests/
├── import/
│   ├── handlers/           # Unit tests for handlers
│   │   ├── table_test.rs
│   │   ├── trigger_test.rs
│   │   └── ...
│   ├── sources/
│   │   └── pg_dump_test.rs
│   └── integration/        # End-to-end import tests
│       └── pg_dump_import_test.rs
```

## Future Extensibility

### Adding Database Introspection Source

```rust
// sources/introspect/mod.rs
pub struct DbIntrospector {
    connection: PgConnection,
}

impl ImportSource for DbIntrospector {
    fn extract_objects(&mut self) -> Result<Vec<RawObject>> {
        let mut objects = vec![];

        // Query information_schema and pg_catalog
        objects.extend(self.extract_tables()?);
        objects.extend(self.extract_views()?);
        objects.extend(self.extract_functions()?);
        // ...

        Ok(objects)
    }

    fn collect_security(&mut self) -> Result<Vec<SecurityStatement>> {
        // Query pg_catalog for ACL info
    }

    fn source_name(&self) -> &'static str {
        "database"
    }
}
```

### Adding New Object Type

1. Create handler in appropriate category directory
2. Implement composable traits
3. Register in category's `register_handlers()` function
4. Add attachment rules if applicable
5. Add tests

## Estimated Effort

| Phase | Files | Lines (approx) | Effort |
|-------|-------|----------------|--------|
| Phase 1 | ~8 | ~400 | Small |
| Phase 2 | ~45 | ~2500 | Large (parallel) |
| Phase 3 | ~3 | ~800 | Medium |
| Phase 4 | ~5 | -500 (deletions) | Small |

**Total:** ~55 files touched, net ~2700 lines (mostly reorganization)

## Notes

- ObjectType enum stays in place (widely used)
- Layer enum stays in place
- ObjectCategory may need expansion
- Existing tests are the safety net
- Each phase should end with passing tests

## Session Checkpoints

Use these to track progress across sessions:

- [x] **Checkpoint 1:** Phase 1 complete - infrastructure in place (2025-11-27)
- [x] **Checkpoint 2:** Phase 2a-2c complete - core types migrated (2025-11-27)
  - [x] Phase 2a (schema_objects) complete (2025-11-27)
  - [x] Phase 2b (types) complete (2025-11-27)
  - [x] Phase 2c (routines) complete (2025-11-27)
  - [x] Phase 2d (attachments) complete (2025-11-28)
  - [x] Phase 2e (fts) complete (2025-11-28)
  - [x] Phase 2f (fdw) complete (2025-11-28)
  - [x] Phase 2g (operators) complete (2025-11-28)
  - [x] Phase 2h (replication) complete (2025-11-28)
  - [x] Phase 2i (security) complete (2025-11-28)
  - [x] Phase 2j (global) complete (2025-11-28)
- [x] **Checkpoint 3:** Phase 2j complete - all handlers migrated (2025-11-28)
- [x] **Checkpoint 4:** Phase 3 complete - parser refactored (2025-11-28)
  - [x] Phase 3a: ExtractedDep and extracted_deps field added (2025-11-28)
  - [x] Phase 3b: patterns.rs moved to sources/pg_dump/ (2025-11-28)
  - [x] Phase 3c: All imports updated (17 files) (2025-11-28)
  - [x] Phase 3d: PgDumpParser implements ImportSource (2025-11-28)
  - [x] Phase 3e: Parser produces Vec<RawObject> with extracted_deps populated (2025-11-28)
  - [x] Phase 3f: ImportOrchestrator created for file writing (2025-11-28)
  - [x] Phase 3g: pg_dump.rs now uses PgDumpParser + ImportOrchestrator (2025-11-28)
- [x] **Checkpoint 5:** Phase 4 complete - handler integration done (2025-12-22)
  - Warnings reduced: 33 → 5
  - All `#![allow(unused_imports)]` removed
  - All handler traits wired up via validate_handler_traits()
  - All helper functions used
  - OutputConfig, RelatedObjects, HeaderBuilder, RawObject methods used
- [x] **Checkpoint 6:** Phase 5 complete - all PatternProvider implementations done (2025-12-22)
  - Created 17 new patterns in patterns.rs
  - Wired all 25 handlers with proper `content_patterns()` implementation
  - Removed all "identified by metadata" comments
  - All tests pass, clippy clean (5 extensibility API warnings remain)
- [x] **Checkpoint 7:** Phase 6 complete - final cleanup done (2025-12-22)
  - Removed 8 dead code items across 4 files
  - All patterns verified as used (47 patterns)
  - 540 tests pass, clippy clean (0 warnings)
  - **Import refactoring complete!**
