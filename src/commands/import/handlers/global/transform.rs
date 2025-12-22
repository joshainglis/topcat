//! Handler for PostgreSQL TRANSFORM objects.
//!
//! Transforms define how to convert between SQL types and procedural
//! language types.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::pg_dump::TRANSFORM_PATTERN;
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL TRANSFORM objects.
///
/// Transforms define how to convert between SQL types and procedural
/// language types. They are global objects (no schema) that belong to
/// the Prepend layer since they may be needed by functions.
///
/// Examples:
/// - `CREATE TRANSFORM FOR hstore LANGUAGE plpython3u (FROM SQL, TO SQL);`
/// - `CREATE TRANSFORM FOR "public"."my_type" LANGUAGE plpgsql (FROM SQL WITH FUNCTION from_sql(), TO SQL WITH FUNCTION to_sql());`
pub struct TransformHandler;

impl PatternProvider for TransformHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&TRANSFORM_PATTERN]
    }
}

impl DependencyExtractor for TransformHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Extract type and language dependencies
        if let Some(caps) = TRANSFORM_PATTERN.captures(content) {
            // Language dependency
            if let Some(lang) = caps.name("language") {
                deps.push((lang.as_str().to_string(), ObjectType::Language));
            }

            // Type dependency
            if let Some(type_name) = caps.name("type_name") {
                let qualified = if let Some(schema) = caps.name("type_schema") {
                    format!("{}.{}", schema.as_str(), type_name.as_str())
                } else {
                    type_name.as_str().to_string()
                };
                deps.push((qualified, ObjectType::Type));
            }
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Transforms depend on types and languages
        vec![ObjectType::Type, ObjectType::Language]
    }
}

impl Categorizer for TransformHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Transform
    }

    fn subcategory(_content: &str) -> Option<String> {
        None
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Global object: _global/transform/{name}.sql
        base_dir
            .join("_global")
            .join("transform")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for TransformHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Transform)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for TransformHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Transform objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Transform,
        TransformHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_handler_layer() {
        assert_eq!(TransformHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_transform_handler_is_primary() {
        assert!(TransformHandler::is_primary());
    }

    #[test]
    fn test_transform_handler_category() {
        assert_eq!(TransformHandler::category(), ObjectCategory::Transform);
    }

    #[test]
    fn test_transform_subcategory() {
        let content = r#"CREATE TRANSFORM FOR hstore LANGUAGE plpython3u (FROM SQL, TO SQL);"#;
        assert_eq!(TransformHandler::subcategory(content), None);
    }

    #[test]
    fn test_transform_output_path() {
        let obj = RawObject::new(
            ObjectType::Transform,
            None, // Global object - no schema
            "hstore_plpython3u".to_string(),
            r#"CREATE TRANSFORM FOR hstore LANGUAGE plpython3u (FROM SQL, TO SQL);"#.to_string(),
        );
        let path = TransformHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/transform/hstore_plpython3u.sql")
        );
    }

    #[test]
    fn test_transform_implicit_dependencies() {
        let deps = TransformHandler::implicit_dependency_types();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::Language));
    }

    #[test]
    fn test_transform_pattern_deps_simple() {
        let content = r#"CREATE TRANSFORM FOR hstore LANGUAGE plpython3u (FROM SQL, TO SQL)"#;
        let deps = TransformHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|(n, t)| n == "plpython3u" && *t == ObjectType::Language)
        );
        assert!(
            deps.iter()
                .any(|(n, t)| n == "hstore" && *t == ObjectType::Type)
        );
    }

    #[test]
    fn test_transform_pattern_deps_with_schema() {
        let content =
            r#"CREATE TRANSFORM FOR "public"."my_type" LANGUAGE plpgsql (FROM SQL, TO SQL)"#;
        let deps = TransformHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|(n, t)| n == "plpgsql" && *t == ObjectType::Language)
        );
        assert!(
            deps.iter()
                .any(|(n, t)| n == "public.my_type" && *t == ObjectType::Type)
        );
    }
}
