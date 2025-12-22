//! Handler for PostgreSQL SCHEMA objects.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::pg_dump::SCHEMA_PATTERN;
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL SCHEMA objects.
///
/// Schemas are namespaces that contain database objects. They are
/// foundation objects that must be created before any objects they contain.
pub struct SchemaHandler;

impl PatternProvider for SchemaHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&SCHEMA_PATTERN]
    }
}

impl DependencyExtractor for SchemaHandler {
    // Schemas don't have implicit dependencies
}

impl Categorizer for SchemaHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Schema
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Schemas go in their own directory at the root
        base_dir.join(&obj.name).join("_schema.sql")
    }
}

impl Configurable for SchemaHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Schema)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for SchemaHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Schema objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::Schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_handler_layer() {
        assert_eq!(SchemaHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_schema_handler_is_primary() {
        assert!(SchemaHandler::is_primary());
    }

    #[test]
    fn test_schema_handler_category() {
        assert_eq!(SchemaHandler::category(), ObjectCategory::Schema);
    }

    #[test]
    fn test_schema_output_path() {
        let obj = RawObject::new(
            ObjectType::Schema,
            Some("public".to_string()),
            "my_schema".to_string(),
            "CREATE SCHEMA my_schema;".to_string(),
        );
        let path = SchemaHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/my_schema/_schema.sql"));
    }
}
