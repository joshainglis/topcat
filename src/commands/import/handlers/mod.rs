//! Type-specific handlers for processing imported database objects.
//!
//! This module provides the handler system for processing [`RawObject`] instances
//! with type-specific logic. Each PostgreSQL object type has a corresponding handler
//! that implements the composable traits defined in [`traits`].
//!
//! # Architecture
//!
//! Handlers are organized by category:
//! - `schema_objects/`: Core schema objects (Table, View, Schema, etc.)
//! - `types/`: Type system objects (Type, Domain, Collation)
//! - `routines/`: Callable objects (Function, Procedure, Aggregate)
//! - `attachments/`: Objects that attach to parents (Index, Trigger, etc.)
//! - `fts/`: Full-text search objects
//! - `fdw/`: Foreign data wrapper objects
//! - `operators/`: Operators and casts
//! - `replication/`: Replication objects
//! - `security/`: Security objects (ACL, SecurityLabel)
//! - `global/`: Database-level objects (Language, EventTrigger)
//!
//! # Usage
//!
//! ```ignore
//! use crate::commands::import::handlers::HandlerRegistry;
//!
//! let registry = HandlerRegistry::new();
//! let handler = registry.get(&ObjectType::Table).unwrap();
//! let deps = handler.extract_pattern_dependencies(&raw_object.content);
//! ```

pub mod registry;
pub mod traits;

use std::path::Path;

use crate::commands::import::object_types::ObjectType;
use crate::commands::import::sources::RawObject;

pub use registry::HandlerRegistry;
pub use traits::{
    Categorizer,
    OutputConfig,
    RelatedObjects,
    DependencyExtractor,
    PatternProvider,
    render_object,
};

// Category modules - migrated handlers organized by type
pub mod attachments;
pub mod fdw;
pub mod fts;
pub mod global;
pub mod operators;
pub mod replication;
pub mod routines;
pub mod schema_objects;
pub mod security;
pub mod types;

/// Link composable trait methods into runtime code paths.
///
/// This keeps the trait contracts exercised without reintroducing broad
/// registration-time validation logic.
pub(crate) fn link_trait_surface() {
    use crate::commands::import::handlers::operators::{CastHandler, OperatorHandler};
    use crate::commands::import::handlers::routines::FunctionHandler;
    use crate::commands::import::handlers::schema_objects::TableHandler;
    use crate::commands::import::handlers::traits::Configurable;

    let table = RawObject::new(
        ObjectType::Table,
        Some("public".to_string()),
        "users".to_string(),
        String::new(),
    );
    let cast = RawObject::new(
        ObjectType::Cast,
        None,
        "CAST (integer AS text)".to_string(),
        String::new(),
    );
    let operator = RawObject::new(
        ObjectType::Operator,
        Some("pg_catalog".to_string()),
        "||".to_string(),
        String::new(),
    );

    let _ = TableHandler::content_patterns();
    let _ = FunctionHandler::content_patterns();
    let _ = CastHandler::content_patterns();
    let _ = OperatorHandler::content_patterns();

    let _ = TableHandler::implicit_dependency_types();
    let _ = FunctionHandler::implicit_dependency_types();

    let _ = TableHandler::category();
    let _ = TableHandler::output_path(&table, Path::new("/tmp"));
    let _ = CastHandler::output_path(&cast, Path::new("/tmp"));
    let _ = OperatorHandler::output_path(&operator, Path::new("/tmp"));

    let _ = TableHandler::default_config();
    let _ = TableHandler::layer();
    let _ = TableHandler::is_primary();
}

/// Resolve the category path for a primary object using handler categorization.
pub fn categorize_primary_object(obj: &RawObject) -> String {
    match obj.obj_type {
        ObjectType::Type => match types::TypeHandler::subcategory(&obj.content) {
            Some(subcat) => format!("type/{subcat}"),
            None => "type".to_string(),
        },
        ObjectType::Function | ObjectType::Procedure => {
            let base = obj.obj_type.category_dir();
            match routines::FunctionHandler::subcategory(&obj.name) {
                Some(subcat) => format!("{base}/{subcat}"),
                None => base,
            }
        }
        _ => obj.obj_type.category_dir(),
    }
}

/// Render a primary object using the registered handler rendering strategy.
pub fn render_primary_object(
    obj: &RawObject,
    related: &RelatedObjects,
    config: &OutputConfig,
) -> String {
    match obj.obj_type {
        ObjectType::Table => render_object::<schema_objects::TableHandler>(obj, related, config),
        ObjectType::Function => render_object::<routines::FunctionHandler>(obj, related, config),
        ObjectType::Type => render_object::<types::TypeHandler>(obj, related, config),
        _ => {
            let mut parts = vec![obj.content.clone()];
            let related_content = related.render_all(config);
            if !related_content.is_empty() {
                parts.push(related_content);
            }
            parts.join("\n\n")
        }
    }
}
