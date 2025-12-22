//! Handler for PostgreSQL TYPE objects.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{
    Layer, ObjectCategory, ObjectType, ObjectTypeConfig, TypeSubcategory,
};
use crate::commands::import::sources::pg_dump::TYPE_PATTERN;
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL TYPE objects.
///
/// Types define custom data types in PostgreSQL. They include:
/// - Enum types (`CREATE TYPE ... AS ENUM`)
/// - Composite types (`CREATE TYPE ... AS (...)`)
/// - Range types (`CREATE TYPE ... AS RANGE`)
/// - Base types (with input/output functions)
///
/// Types belong to the Prepend layer as they are foundational objects
/// that other objects depend on.
pub struct TypeHandler;

impl PatternProvider for TypeHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&TYPE_PATTERN]
    }
}

impl DependencyExtractor for TypeHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Types may depend on other types (composite types) and schemas
        vec![ObjectType::Schema]
    }
}

impl Categorizer for TypeHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Type
    }

    fn subcategory(content: &str) -> Option<String> {
        Some(
            TypeSubcategory::from_content(content)
                .subdirectory()
                .to_string(),
        )
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        let subcategory = TypeSubcategory::from_content(&obj.content).subdirectory();
        base_dir
            .join(schema)
            .join("type")
            .join(subcategory)
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for TypeHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Type)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for TypeHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Type objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::Type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_handler_layer() {
        assert_eq!(TypeHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_type_handler_is_primary() {
        assert!(TypeHandler::is_primary());
    }

    #[test]
    fn test_type_handler_category() {
        assert_eq!(TypeHandler::category(), ObjectCategory::Type);
    }

    #[test]
    fn test_type_subcategory_enum() {
        let content = "CREATE TYPE status AS ENUM ('active', 'inactive');";
        assert_eq!(TypeHandler::subcategory(content), Some("enum".to_string()));
    }

    #[test]
    fn test_type_subcategory_composite() {
        let content = "CREATE TYPE point AS (x integer, y integer);";
        assert_eq!(
            TypeHandler::subcategory(content),
            Some("composite".to_string())
        );
    }

    #[test]
    fn test_type_subcategory_range() {
        let content = "CREATE TYPE daterange AS RANGE (SUBTYPE = date);";
        assert_eq!(TypeHandler::subcategory(content), Some("range".to_string()));
    }

    #[test]
    fn test_type_output_path_enum() {
        let obj = RawObject::new(
            ObjectType::Type,
            Some("public".to_string()),
            "status".to_string(),
            "CREATE TYPE status AS ENUM ('active', 'inactive');".to_string(),
        );
        let path = TypeHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/type/enum/status.sql"));
    }

    #[test]
    fn test_type_output_path_composite() {
        let obj = RawObject::new(
            ObjectType::Type,
            Some("myschema".to_string()),
            "point".to_string(),
            "CREATE TYPE point AS (x int, y int);".to_string(),
        );
        let path = TypeHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/myschema/type/composite/point.sql")
        );
    }
}
