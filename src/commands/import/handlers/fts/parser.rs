//! Handler for PostgreSQL TEXT SEARCH PARSER objects.

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

/// Handler for PostgreSQL TEXT SEARCH PARSER objects.
///
/// Text search parsers break text into tokens for full-text search indexing.
/// They define how to split text and what token types are produced.
///
/// Parsers belong to the Prepend layer as they are the most fundamental FTS objects.
/// Configurations depend on parsers.
pub struct ParserHandler;

impl PatternProvider for ParserHandler {
    // Parsers are identified by metadata, not content patterns
}

impl DependencyExtractor for ParserHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Parsers are foundational and don't typically depend on other FTS objects
        vec![ObjectType::Schema]
    }
}

impl Categorizer for ParserHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TextSearch
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some(FtsSubcategory::Parser.subdirectory().to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("fts")
            .join(FtsSubcategory::Parser.subdirectory())
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for ParserHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::TextSearchParser)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for ParserHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for TextSearchParser objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::TextSearchParser)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_handler_layer() {
        assert_eq!(ParserHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_parser_handler_is_primary() {
        assert!(ParserHandler::is_primary());
    }

    #[test]
    fn test_parser_handler_category() {
        assert_eq!(ParserHandler::category(), ObjectCategory::TextSearch);
    }

    #[test]
    fn test_parser_subcategory() {
        let content = "CREATE TEXT SEARCH PARSER my_parser (START = start_fn);";
        assert_eq!(
            ParserHandler::subcategory(content),
            Some("parser".to_string())
        );
    }

    #[test]
    fn test_parser_output_path() {
        let obj = RawObject::new(
            ObjectType::TextSearchParser,
            Some("public".to_string()),
            "my_parser".to_string(),
            "CREATE TEXT SEARCH PARSER my_parser (START = start_fn);".to_string(),
        );
        let path = ParserHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/fts/parser/my_parser.sql")
        );
    }

    #[test]
    fn test_parser_output_path_custom_schema() {
        let obj = RawObject::new(
            ObjectType::TextSearchParser,
            Some("search".to_string()),
            "custom_parser".to_string(),
            "CREATE TEXT SEARCH PARSER custom_parser (START = start_fn);".to_string(),
        );
        let path = ParserHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/search/fts/parser/custom_parser.sql")
        );
    }

    #[test]
    fn test_parser_implicit_dependencies() {
        let deps = ParserHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Schema));
    }
}
