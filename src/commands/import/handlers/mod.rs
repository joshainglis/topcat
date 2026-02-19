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

use crate::commands::import::object_types::ObjectType;
use crate::commands::import::sources::RawObject;

pub use registry::HandlerRegistry;
pub use traits::{
    Categorizer,
    OutputConfig,
    RelatedObjects,
    extract_dependencies,
    get_category_info,
    get_handler_config,
    // Helper functions that use traits as bounds
    get_implicit_deps,
    get_patterns,
    process_object,
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
