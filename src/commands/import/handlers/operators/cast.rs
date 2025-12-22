//! Handler for PostgreSQL CAST objects.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::pg_dump::{build_cast_pattern, CAST_PATTERN, DEFAULT_SCHEMA_PATTERN};
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL CAST objects.
///
/// Casts define how to convert between data types.
/// They are global objects that depend on types and optionally on cast functions.
/// They belong to the Prepend layer as foundation objects.
pub struct CastHandler;

impl PatternProvider for CastHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&CAST_PATTERN]
    }
}

impl DependencyExtractor for CastHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Casts depend on source and target types, and optionally a function
        vec![ObjectType::Type, ObjectType::Function]
    }

    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];
        let pattern = build_cast_pattern(DEFAULT_SCHEMA_PATTERN);

        if let Some(caps) = pattern.captures(content) {
            // Extract source type dependency
            if let Some(from_name) = caps.name("from_name") {
                deps.push((from_name.as_str().to_string(), ObjectType::Type));
            }

            // Extract target type dependency
            if let Some(to_name) = caps.name("to_name") {
                deps.push((to_name.as_str().to_string(), ObjectType::Type));
            }

            // Extract cast function dependency (if not INOUT cast)
            if let Some(func_name) = caps.name("func_name") {
                deps.push((func_name.as_str().to_string(), ObjectType::Function));
            }
        }

        deps
    }
}

impl Categorizer for CastHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Cast
    }

    fn subcategory(_content: &str) -> Option<String> {
        None
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Global object: _global/cast/name.sql
        // The name typically includes the cast signature like "integer_to_text"
        let safe_name = sanitize_cast_name(&obj.name);
        base_dir
            .join("_global")
            .join("cast")
            .join(format!("{safe_name}.sql"))
    }
}

impl Configurable for CastHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Cast)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for CastHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Sanitize cast name for use as filename.
///
/// Cast names from pg_dump can include special characters like parentheses and spaces
/// (e.g., "CAST (integer AS text)"). These need to be converted to safe filename characters.
fn sanitize_cast_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '(' | ')' => '_'.to_string(),
            ' ' => "_".to_string(),
            '.' => "_".to_string(),
            _ => c.to_string(),
        })
        .collect::<String>()
        .replace("__", "_")
        .trim_matches('_')
        .to_string()
}

/// Extract pattern-based dependencies from Cast content.
fn extract_cast_dependencies(content: &str) -> Vec<(String, ObjectType)> {
    CastHandler::extract_pattern_dependencies(content)
}

/// Create a registered handler for Cast objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(ObjectType::Cast, extract_cast_dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cast_handler_layer() {
        assert_eq!(CastHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_cast_handler_is_primary() {
        assert!(CastHandler::is_primary());
    }

    #[test]
    fn test_cast_handler_category() {
        assert_eq!(CastHandler::category(), ObjectCategory::Cast);
    }

    #[test]
    fn test_cast_subcategory() {
        let content = r#"CREATE CAST (integer AS text) WITH FUNCTION int4_to_text;"#;
        assert_eq!(CastHandler::subcategory(content), None);
    }

    #[test]
    fn test_cast_output_path() {
        let obj = RawObject::new(
            ObjectType::Cast,
            None, // Global object - no schema
            "CAST (integer AS text)".to_string(),
            r#"CREATE CAST (integer AS text) WITH FUNCTION int4_to_text;"#.to_string(),
        );
        let path = CastHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/cast/CAST_integer_AS_text.sql")
        );
    }

    #[test]
    fn test_cast_implicit_dependencies() {
        let deps = CastHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::Function));
    }

    #[test]
    fn test_sanitize_cast_name() {
        assert_eq!(
            sanitize_cast_name("CAST (integer AS text)"),
            "CAST_integer_AS_text"
        );
        assert_eq!(sanitize_cast_name("int_to_text"), "int_to_text");
        assert_eq!(
            sanitize_cast_name("CAST (myschema.mytype AS text)"),
            "CAST_myschema_mytype_AS_text"
        );
    }

    #[test]
    fn test_cast_extract_pattern_dependencies_with_function() {
        let content = r#"CREATE CAST (integer AS text) WITH FUNCTION int4_to_text;"#;
        let deps = CastHandler::extract_pattern_dependencies(content);

        // Should extract type dependencies
        let has_from_type = deps
            .iter()
            .any(|(name, t)| *t == ObjectType::Type && name == "integer");
        let has_to_type = deps
            .iter()
            .any(|(name, t)| *t == ObjectType::Type && name == "text");

        assert!(has_from_type, "Should extract source type dependency");
        assert!(has_to_type, "Should extract target type dependency");
    }

    #[test]
    fn test_cast_extract_pattern_dependencies_inout() {
        let content = r#"CREATE CAST (integer AS text) WITH INOUT;"#;
        let deps = CastHandler::extract_pattern_dependencies(content);

        // INOUT cast should not extract function dependency
        let has_function = deps.iter().any(|(_, t)| *t == ObjectType::Function);
        assert!(
            !has_function,
            "INOUT cast should not have function dependency"
        );
    }
}
