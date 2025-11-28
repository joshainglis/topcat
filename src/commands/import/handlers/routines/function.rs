//! Handler for PostgreSQL FUNCTION objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL FUNCTION objects.
///
/// Functions are callable objects that return a value. They can be written in
/// SQL, PL/pgSQL, or other procedural languages.
///
/// Functions belong to the Normal layer and depend on types, domains, and
/// extensions they use in their parameters, return types, and body.
pub struct FunctionHandler;

impl PatternProvider for FunctionHandler {
    // Functions are identified by metadata, not content patterns
}

impl DependencyExtractor for FunctionHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Functions depend on types for parameters/returns, and may use extensions
        vec![ObjectType::Type, ObjectType::Domain, ObjectType::Extension]
    }
}

impl Categorizer for FunctionHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Function
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("functions")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for FunctionHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Function)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for FunctionHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Function objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::Function)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_handler_layer() {
        assert_eq!(FunctionHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_function_handler_is_primary() {
        assert!(FunctionHandler::is_primary());
    }

    #[test]
    fn test_function_handler_category() {
        assert_eq!(FunctionHandler::category(), ObjectCategory::Function);
    }

    #[test]
    fn test_function_implicit_deps() {
        let deps = FunctionHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::Domain));
        assert!(deps.contains(&ObjectType::Extension));
    }

    #[test]
    fn test_function_output_path() {
        let obj = RawObject::new(
            ObjectType::Function,
            Some("public".to_string()),
            "calculate_total".to_string(),
            "CREATE FUNCTION calculate_total() RETURNS numeric ...".to_string(),
        );
        let path = FunctionHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/functions/calculate_total.sql")
        );
    }

    #[test]
    fn test_function_output_path_no_schema() {
        let obj = RawObject::new(
            ObjectType::Function,
            None,
            "helper_func".to_string(),
            "CREATE FUNCTION helper_func() ...".to_string(),
        );
        let path = FunctionHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/functions/helper_func.sql")
        );
    }
}
