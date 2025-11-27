//! Handler for PostgreSQL TABLE objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL TABLE objects.
///
/// Tables are primary structural objects that store data. They belong to the
/// Normal layer and can have many attached objects (indexes, constraints,
/// triggers, policies, etc.).
pub struct TableHandler;

impl PatternProvider for TableHandler {
    // Tables are identified by metadata, not content patterns
}

impl DependencyExtractor for TableHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Type, ObjectType::Domain, ObjectType::Schema]
    }
}

impl Categorizer for TableHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Table
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
        ObjectTypeConfig::default_for(ObjectType::Table)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for TableHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Table objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::Table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_handler_layer() {
        assert_eq!(TableHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_table_handler_is_primary() {
        assert!(TableHandler::is_primary());
    }

    #[test]
    fn test_table_handler_category() {
        assert_eq!(TableHandler::category(), ObjectCategory::Table);
    }

    #[test]
    fn test_table_implicit_deps() {
        let deps = TableHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::Domain));
        assert!(deps.contains(&ObjectType::Schema));
    }

    #[test]
    fn test_table_output_path() {
        let obj = RawObject::new(
            ObjectType::Table,
            Some("public".to_string()),
            "users".to_string(),
            "CREATE TABLE users (...);".to_string(),
        );
        let path = TableHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/table/users.sql"));
    }

    #[test]
    fn test_table_output_path_no_schema() {
        let obj = RawObject::new(
            ObjectType::Table,
            None,
            "users".to_string(),
            "CREATE TABLE users (...);".to_string(),
        );
        let path = TableHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/table/users.sql"));
    }
}
