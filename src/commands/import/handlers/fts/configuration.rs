//! Handler for PostgreSQL TEXT SEARCH CONFIGURATION objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{
    FtsSubcategory, Layer, ObjectCategory, ObjectType, ObjectTypeConfig,
};
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL TEXT SEARCH CONFIGURATION objects.
///
/// Text search configurations define how text is processed for full-text search.
/// They specify which parser to use and which dictionaries to apply to each
/// token type produced by the parser.
///
/// Configurations belong to the Prepend layer as they are foundation objects
/// that other objects may depend on.
pub struct ConfigurationHandler;

impl PatternProvider for ConfigurationHandler {
    // Configurations are identified by metadata, not content patterns
}

impl DependencyExtractor for ConfigurationHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Configurations depend on their parser
        vec![ObjectType::TextSearchParser]
    }
}

impl Categorizer for ConfigurationHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TextSearch
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some(FtsSubcategory::Configuration.subdirectory().to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("fts")
            .join(FtsSubcategory::Configuration.subdirectory())
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for ConfigurationHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::TextSearchConfiguration)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for ConfigurationHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for TextSearchConfiguration objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::TextSearchConfiguration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_handler_layer() {
        assert_eq!(ConfigurationHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_configuration_handler_is_primary() {
        assert!(ConfigurationHandler::is_primary());
    }

    #[test]
    fn test_configuration_handler_category() {
        assert_eq!(ConfigurationHandler::category(), ObjectCategory::TextSearch);
    }

    #[test]
    fn test_configuration_subcategory() {
        let content = "CREATE TEXT SEARCH CONFIGURATION my_config (PARSER = default);";
        assert_eq!(
            ConfigurationHandler::subcategory(content),
            Some("configuration".to_string())
        );
    }

    #[test]
    fn test_configuration_output_path() {
        let obj = RawObject::new(
            ObjectType::TextSearchConfiguration,
            Some("public".to_string()),
            "my_config".to_string(),
            "CREATE TEXT SEARCH CONFIGURATION my_config (PARSER = default);".to_string(),
        );
        let path = ConfigurationHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/fts/configuration/my_config.sql")
        );
    }

    #[test]
    fn test_configuration_output_path_custom_schema() {
        let obj = RawObject::new(
            ObjectType::TextSearchConfiguration,
            Some("search".to_string()),
            "english_config".to_string(),
            "CREATE TEXT SEARCH CONFIGURATION english_config (PARSER = default);".to_string(),
        );
        let path = ConfigurationHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/search/fts/configuration/english_config.sql")
        );
    }

    #[test]
    fn test_configuration_implicit_dependencies() {
        let deps = ConfigurationHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::TextSearchParser));
    }
}
