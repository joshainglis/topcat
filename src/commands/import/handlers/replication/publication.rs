//! Handler for PostgreSQL PUBLICATION objects.

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
use crate::commands::import::sources::pg_dump::PUBLICATION_PATTERN;

/// Handler for PostgreSQL PUBLICATION objects.
///
/// Publications define which tables are available for logical replication.
/// They are global objects (no schema) that belong to the Normal layer.
/// Publications don't have dependencies on other objects.
pub struct PublicationHandler;

impl PatternProvider for PublicationHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&PUBLICATION_PATTERN]
    }
}

impl DependencyExtractor for PublicationHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Publications don't have implicit dependencies
        // (tables referenced are soft dependencies, not hard requirements)
        vec![]
    }

    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        // Reference pattern to ensure it's used, even if no deps extracted
        let _ = PUBLICATION_PATTERN.is_match(content);
        vec![]
    }
}

impl Categorizer for PublicationHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Replication
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("publication".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Global object: _global/replication/publication/{name}.sql
        base_dir
            .join("_global")
            .join("replication")
            .join("publication")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for PublicationHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Publication)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for PublicationHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Publication objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Publication,
        PublicationHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publication_handler_layer() {
        assert_eq!(PublicationHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_publication_handler_is_primary() {
        assert!(PublicationHandler::is_primary());
    }

    #[test]
    fn test_publication_handler_category() {
        assert_eq!(PublicationHandler::category(), ObjectCategory::Replication);
    }

    #[test]
    fn test_publication_subcategory() {
        let content = r#"CREATE PUBLICATION "my_pub" FOR ALL TABLES;"#;
        assert_eq!(
            PublicationHandler::subcategory(content),
            Some("publication".to_string())
        );
    }

    #[test]
    fn test_publication_output_path() {
        let obj = RawObject::new(
            ObjectType::Publication,
            None, // Global object - no schema
            "my_pub".to_string(),
            r#"CREATE PUBLICATION "my_pub" FOR ALL TABLES;"#.to_string(),
        );
        let path = PublicationHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/replication/publication/my_pub.sql")
        );
    }

    #[test]
    fn test_publication_implicit_dependencies() {
        let deps = PublicationHandler::implicit_dependency_types();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_publication_no_pattern_dependencies() {
        let content = r#"CREATE PUBLICATION "my_pub" FOR TABLE "public"."users";"#;
        let deps = PublicationHandler::extract_pattern_dependencies(content);
        assert!(deps.is_empty());
    }
}
