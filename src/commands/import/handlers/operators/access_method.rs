//! Handler for PostgreSQL ACCESS METHOD objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::patterns::ACCESS_METHOD_PATTERN;
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL ACCESS METHOD objects.
///
/// Access methods define how indexes are stored and accessed.
/// They are global objects (no schema) that can depend on handler functions.
/// They belong to the Prepend layer as foundation objects.
pub struct AccessMethodHandler;

impl PatternProvider for AccessMethodHandler {
    // Access methods are identified by metadata, not content patterns
}

impl DependencyExtractor for AccessMethodHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Access methods may depend on handler functions
        vec![ObjectType::Function]
    }

    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        if let Some(caps) = ACCESS_METHOD_PATTERN.captures(content) {
            // Extract the handler function
            if let Some(handler) = caps.name("handler") {
                let full_handler = if let Some(schema) = caps.name("handler_schema") {
                    format!("{}.{}", schema.as_str(), handler.as_str())
                } else {
                    handler.as_str().to_string()
                };
                deps.push((full_handler, ObjectType::Function));
            }
        }

        deps
    }
}

impl Categorizer for AccessMethodHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::AccessMethod
    }

    fn subcategory(_content: &str) -> Option<String> {
        None
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Global object: _global/access_method/name.sql
        base_dir
            .join("_global")
            .join("access_method")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for AccessMethodHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::AccessMethod)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for AccessMethodHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Extract pattern-based dependencies from AccessMethod content.
fn extract_am_dependencies(content: &str) -> Vec<(String, ObjectType)> {
    AccessMethodHandler::extract_pattern_dependencies(content)
}

/// Create a registered handler for AccessMethod objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(ObjectType::AccessMethod, extract_am_dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_method_handler_layer() {
        assert_eq!(AccessMethodHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_access_method_handler_is_primary() {
        assert!(AccessMethodHandler::is_primary());
    }

    #[test]
    fn test_access_method_handler_category() {
        assert_eq!(
            AccessMethodHandler::category(),
            ObjectCategory::AccessMethod
        );
    }

    #[test]
    fn test_access_method_subcategory() {
        let content = r#"CREATE ACCESS METHOD brin TYPE INDEX HANDLER brinhandler;"#;
        assert_eq!(AccessMethodHandler::subcategory(content), None);
    }

    #[test]
    fn test_access_method_output_path() {
        let obj = RawObject::new(
            ObjectType::AccessMethod,
            None, // Global object - no schema
            "brin".to_string(),
            r#"CREATE ACCESS METHOD brin TYPE INDEX HANDLER brinhandler;"#.to_string(),
        );
        let path = AccessMethodHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/access_method/brin.sql")
        );
    }

    #[test]
    fn test_access_method_implicit_dependencies() {
        let deps = AccessMethodHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Function));
    }

    #[test]
    fn test_access_method_extract_pattern_dependencies() {
        let content = r#"CREATE ACCESS METHOD myam TYPE INDEX HANDLER "pg_catalog"."myhandler";"#;
        let deps = AccessMethodHandler::extract_pattern_dependencies(content);

        // Should extract the handler function dependency
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "pg_catalog.myhandler");
        assert_eq!(deps[0].1, ObjectType::Function);
    }

    #[test]
    fn test_access_method_extract_pattern_dependencies_no_schema() {
        let content = r#"CREATE ACCESS METHOD myam TYPE INDEX HANDLER myhandler;"#;
        let deps = AccessMethodHandler::extract_pattern_dependencies(content);

        // Should extract the handler function dependency
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "myhandler");
        assert_eq!(deps[0].1, ObjectType::Function);
    }
}
