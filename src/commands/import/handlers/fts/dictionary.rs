//! Handler for PostgreSQL TEXT SEARCH DICTIONARY objects.

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

/// Handler for PostgreSQL TEXT SEARCH DICTIONARY objects.
///
/// Text search dictionaries define how tokens are normalized during full-text search.
/// They are based on templates and can have custom options (like stopwords, language).
///
/// Dictionaries belong to the Prepend layer as they are foundation objects
/// that configurations depend on.
pub struct DictionaryHandler;

impl PatternProvider for DictionaryHandler {
    // Dictionaries are identified by metadata, not content patterns
}

impl DependencyExtractor for DictionaryHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Dictionaries depend on their template
        vec![ObjectType::TextSearchTemplate]
    }
}

impl Categorizer for DictionaryHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TextSearch
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some(FtsSubcategory::Dictionary.subdirectory().to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("fts")
            .join(FtsSubcategory::Dictionary.subdirectory())
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for DictionaryHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::TextSearchDictionary)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for DictionaryHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for TextSearchDictionary objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::TextSearchDictionary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_handler_layer() {
        assert_eq!(DictionaryHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_dictionary_handler_is_primary() {
        assert!(DictionaryHandler::is_primary());
    }

    #[test]
    fn test_dictionary_handler_category() {
        assert_eq!(DictionaryHandler::category(), ObjectCategory::TextSearch);
    }

    #[test]
    fn test_dictionary_subcategory() {
        let content = "CREATE TEXT SEARCH DICTIONARY my_dict (TEMPLATE = snowball);";
        assert_eq!(
            DictionaryHandler::subcategory(content),
            Some("dictionary".to_string())
        );
    }

    #[test]
    fn test_dictionary_output_path() {
        let obj = RawObject::new(
            ObjectType::TextSearchDictionary,
            Some("public".to_string()),
            "english_stem".to_string(),
            "CREATE TEXT SEARCH DICTIONARY english_stem (TEMPLATE = snowball);".to_string(),
        );
        let path = DictionaryHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/fts/dictionary/english_stem.sql")
        );
    }

    #[test]
    fn test_dictionary_output_path_custom_schema() {
        let obj = RawObject::new(
            ObjectType::TextSearchDictionary,
            Some("search".to_string()),
            "french_dict".to_string(),
            "CREATE TEXT SEARCH DICTIONARY french_dict (TEMPLATE = snowball);".to_string(),
        );
        let path = DictionaryHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/search/fts/dictionary/french_dict.sql")
        );
    }

    #[test]
    fn test_dictionary_implicit_dependencies() {
        let deps = DictionaryHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::TextSearchTemplate));
    }
}
