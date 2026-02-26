//! Handlers for PostgreSQL callable objects.
//!
//! This module contains handlers for routine objects:
//! - [`FunctionHandler`]: User-defined functions
//! - [`ProcedureHandler`]: Stored procedures (PostgreSQL 11+)
//! - [`AggregateHandler`]: User-defined aggregate functions

mod aggregate;
mod function;
mod procedure;

// FunctionHandler is used by the orchestrator's categorize_object
pub use aggregate::AggregateHandler;
pub use function::FunctionHandler;
pub use procedure::ProcedureHandler;
use std::path::{Path, PathBuf};

use super::registry::HandlerRegistry;
use super::traits::{OutputConfig, RelatedObjects};
use crate::commands::import::object_types::ObjectType;
use crate::commands::import::sources::RawObject;

pub(super) fn routine_implicit_dependency_types() -> Vec<ObjectType> {
    vec![ObjectType::Type, ObjectType::Domain, ObjectType::Extension]
}

pub(super) fn routine_output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
    let schema = obj.schema.as_deref().unwrap_or("public");
    base_dir
        .join(schema)
        .join("functions")
        .join(format!("{}.sql", obj.name))
}

pub(super) fn routine_render(
    obj: &RawObject,
    related: &RelatedObjects,
    config: &OutputConfig,
) -> String {
    let mut parts = vec![obj.content.clone()];

    let related_content = related.render_all(config);
    if !related_content.is_empty() {
        parts.push(related_content);
    }

    parts.join("\n\n")
}

/// Register all routine handlers with the registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(function::create_handler());
    registry.register(procedure::create_handler());
    registry.register(aggregate::create_handler());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::object_types::ObjectType;
    use crate::commands::import::sources::RawObject;
    use std::path::Path;

    #[test]
    fn test_register_handlers() {
        let mut registry = HandlerRegistry::new();
        register_handlers(&mut registry);

        assert!(registry.has_handler(&ObjectType::Function));
        assert!(registry.has_handler(&ObjectType::Procedure));
        assert!(registry.has_handler(&ObjectType::Aggregate));
    }

    #[test]
    fn test_shared_routine_helpers() {
        let obj = RawObject::new(
            ObjectType::Procedure,
            Some("public".to_string()),
            "refresh_cache".to_string(),
            "CREATE PROCEDURE refresh_cache() LANGUAGE SQL AS $$ SELECT 1; $$".to_string(),
        );
        let related = RelatedObjects::new();

        assert_eq!(
            routine_implicit_dependency_types(),
            vec![ObjectType::Type, ObjectType::Domain, ObjectType::Extension]
        );
        assert_eq!(
            routine_output_path(&obj, Path::new("/output")),
            PathBuf::from("/output/public/functions/refresh_cache.sql")
        );
        assert!(
            routine_render(&obj, &related, &OutputConfig::default())
                .contains("CREATE PROCEDURE refresh_cache()")
        );
    }
}
