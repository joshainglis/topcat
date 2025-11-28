//! Handler for PostgreSQL LANGUAGE objects.
//!
//! Languages define procedural languages available for writing functions
//! and procedures (plpgsql, plpython, etc.).

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL LANGUAGE objects.
///
/// Languages define procedural languages available for writing functions.
/// They are global objects (no schema) that belong to the Prepend layer
/// since functions may depend on them.
///
/// Examples:
/// - `CREATE PROCEDURAL LANGUAGE plpgsql;`
/// - `CREATE TRUSTED LANGUAGE plpython3u;`
pub struct LanguageHandler;

impl PatternProvider for LanguageHandler {
    // Languages are identified by metadata, not content patterns
}

impl DependencyExtractor for LanguageHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Languages typically don't have dependencies on other objects
        // (the handler functions are usually from extensions or pg_catalog)
        vec![]
    }

    fn extract_pattern_dependencies(_content: &str) -> Vec<(String, ObjectType)> {
        // Languages don't have structural dependencies in their CREATE statement
        vec![]
    }
}

impl Categorizer for LanguageHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Language
    }

    fn subcategory(_content: &str) -> Option<String> {
        None
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Global object: _global/language/{name}.sql
        base_dir
            .join("_global")
            .join("language")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for LanguageHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Language)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for LanguageHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Language objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::Language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_handler_layer() {
        assert_eq!(LanguageHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_language_handler_is_primary() {
        assert!(LanguageHandler::is_primary());
    }

    #[test]
    fn test_language_handler_category() {
        assert_eq!(LanguageHandler::category(), ObjectCategory::Language);
    }

    #[test]
    fn test_language_subcategory() {
        let content = r#"CREATE PROCEDURAL LANGUAGE "plpgsql";"#;
        assert_eq!(LanguageHandler::subcategory(content), None);
    }

    #[test]
    fn test_language_output_path() {
        let obj = RawObject::new(
            ObjectType::Language,
            None, // Global object - no schema
            "plpgsql".to_string(),
            r#"CREATE PROCEDURAL LANGUAGE "plpgsql";"#.to_string(),
        );
        let path = LanguageHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/_global/language/plpgsql.sql"));
    }

    #[test]
    fn test_language_implicit_dependencies() {
        let deps = LanguageHandler::implicit_dependency_types();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_language_no_pattern_dependencies() {
        let content = r#"CREATE TRUSTED PROCEDURAL LANGUAGE "plpython3u";"#;
        let deps = LanguageHandler::extract_pattern_dependencies(content);
        assert!(deps.is_empty());
    }
}
