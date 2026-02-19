//! Handlers for core schema-level PostgreSQL objects.
//!
//! This module contains handlers for the primary structural objects that
//! define a database schema: tables, views, sequences, and the schemas/extensions
//! they belong to.
//!
//! # Object Types
//!
//! - `SchemaHandler` - Schema namespaces
//! - `ExtensionHandler` - PostgreSQL extensions
//! - `TableHandler` - Regular tables
//! - `ViewHandler` - Views
//! - `MaterializedViewHandler` - Materialized views
//! - `ForeignTableHandler` - Foreign tables (FDW)
//! - `SequenceHandler` - Sequences

mod extension;
mod foreign_table;
mod materialized_view;
mod schema;
mod sequence;
mod table;
mod view;

pub use table::TableHandler;

use super::registry::HandlerRegistry;

/// Register all schema object handlers with the registry.
///
/// This replaces the default handlers for schema object types with
/// type-specific implementations that include pattern dependencies
/// and proper categorization.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    // Schema and Extension (Prepend layer)
    registry.register(schema::create_handler());
    registry.register(extension::create_handler());

    // Core structural objects (Normal layer)
    registry.register(table::create_handler());
    registry.register(view::create_handler());
    registry.register(materialized_view::create_handler());
    registry.register(foreign_table::create_handler());
    registry.register(sequence::create_handler());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::object_types::ObjectType;

    #[test]
    fn test_register_handlers() {
        let mut registry = HandlerRegistry::new();
        register_handlers(&mut registry);

        // Verify all schema object handlers are registered
        assert!(registry.has_handler(&ObjectType::Schema));
        assert!(registry.has_handler(&ObjectType::Extension));
        assert!(registry.has_handler(&ObjectType::Table));
        assert!(registry.has_handler(&ObjectType::View));
        assert!(registry.has_handler(&ObjectType::MaterializedView));
        assert!(registry.has_handler(&ObjectType::ForeignTable));
        assert!(registry.has_handler(&ObjectType::Sequence));
    }
}
