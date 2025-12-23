//! Handler for PostgreSQL DOMAIN objects.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;
use crate::commands::import::sources::pg_dump::DOMAIN_PATTERN;

/// Handler for PostgreSQL DOMAIN objects.
///
/// Domains are data types with optional constraints. They are built on top of
/// existing types and add validation rules. For example:
/// `CREATE DOMAIN positive_int AS integer CHECK (VALUE > 0);`
///
/// Domains belong to the Prepend layer as they are foundational type definitions
/// that tables and functions depend on.
#[allow(dead_code)] // Marker type for trait implementations
pub struct DomainHandler;

impl PatternProvider for DomainHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&DOMAIN_PATTERN]
    }
}

impl DependencyExtractor for DomainHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        // Reference pattern to ensure it's used, even if no deps extracted
        let _ = DOMAIN_PATTERN.is_match(content);
        vec![]
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Domains depend on their underlying type
        vec![ObjectType::Type, ObjectType::Schema]
    }
}

impl Categorizer for DomainHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Type
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("domain".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("type")
            .join("domain")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for DomainHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Domain)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for DomainHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Domain objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Domain,
        DomainHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_handler_layer() {
        assert_eq!(DomainHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_domain_handler_is_primary() {
        assert!(DomainHandler::is_primary());
    }

    #[test]
    fn test_domain_handler_category() {
        assert_eq!(DomainHandler::category(), ObjectCategory::Type);
    }

    #[test]
    fn test_domain_implicit_deps() {
        let deps = DomainHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::Schema));
    }

    #[test]
    fn test_domain_subcategory() {
        assert_eq!(
            DomainHandler::subcategory("anything"),
            Some("domain".to_string())
        );
    }

    #[test]
    fn test_domain_output_path() {
        let obj = RawObject::new(
            ObjectType::Domain,
            Some("public".to_string()),
            "email_address".to_string(),
            "CREATE DOMAIN email_address AS text CHECK (...);".to_string(),
        );
        let path = DomainHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/type/domain/email_address.sql")
        );
    }

    #[test]
    fn test_domain_output_path_no_schema() {
        let obj = RawObject::new(
            ObjectType::Domain,
            None,
            "positive_int".to_string(),
            "CREATE DOMAIN positive_int AS integer CHECK (VALUE > 0);".to_string(),
        );
        let path = DomainHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/type/domain/positive_int.sql")
        );
    }
}
