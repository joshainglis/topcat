//! Handler for PostgreSQL TEXT SEARCH TEMPLATE objects.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{
    FtsSubcategory, Layer, ObjectCategory, ObjectType, ObjectTypeConfig,
};
use crate::commands::import::sources::RawObject;
use crate::commands::import::sources::pg_dump::FTS_TEMPLATE_PATTERN;

/// Handler for PostgreSQL TEXT SEARCH TEMPLATE objects.
///
/// Text search templates define the implementation for dictionaries.
/// They specify the functions used for initialization and lexizing.
///
/// Templates belong to the Prepend layer as they are the most fundamental FTS objects.
/// Dictionaries depend on templates.
#[allow(dead_code)] // Marker type for trait implementations
pub struct TemplateHandler;

impl PatternProvider for TemplateHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&FTS_TEMPLATE_PATTERN]
    }
}

impl DependencyExtractor for TemplateHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        // Reference pattern to ensure it's used, even if no deps extracted
        let _ = FTS_TEMPLATE_PATTERN.is_match(content);
        vec![]
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Templates are foundational and don't typically depend on other FTS objects
        // They may depend on functions for init/lexize
        vec![ObjectType::Schema, ObjectType::Function]
    }
}

impl Categorizer for TemplateHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TextSearch
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some(FtsSubcategory::Template.subdirectory().to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("fts")
            .join(FtsSubcategory::Template.subdirectory())
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for TemplateHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::TextSearchTemplate)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for TemplateHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for TextSearchTemplate objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::TextSearchTemplate,
        TemplateHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_handler_layer() {
        assert_eq!(TemplateHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_template_handler_is_primary() {
        assert!(TemplateHandler::is_primary());
    }

    #[test]
    fn test_template_handler_category() {
        assert_eq!(TemplateHandler::category(), ObjectCategory::TextSearch);
    }

    #[test]
    fn test_template_subcategory() {
        let content = "CREATE TEXT SEARCH TEMPLATE my_template (INIT = init_fn);";
        assert_eq!(
            TemplateHandler::subcategory(content),
            Some("template".to_string())
        );
    }

    #[test]
    fn test_template_output_path() {
        let obj = RawObject::new(
            ObjectType::TextSearchTemplate,
            Some("public".to_string()),
            "my_template".to_string(),
            "CREATE TEXT SEARCH TEMPLATE my_template (INIT = init_fn);".to_string(),
        );
        let path = TemplateHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/fts/template/my_template.sql")
        );
    }

    #[test]
    fn test_template_output_path_custom_schema() {
        let obj = RawObject::new(
            ObjectType::TextSearchTemplate,
            Some("search".to_string()),
            "snowball".to_string(),
            "CREATE TEXT SEARCH TEMPLATE snowball (INIT = init_fn);".to_string(),
        );
        let path = TemplateHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/search/fts/template/snowball.sql")
        );
    }

    #[test]
    fn test_template_implicit_dependencies() {
        let deps = TemplateHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Schema));
        assert!(deps.contains(&ObjectType::Function));
    }
}
