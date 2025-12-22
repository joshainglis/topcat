//! Handler for PostgreSQL FUNCTION objects.

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

/// Pattern for identifying API functions.
///
/// Matches function names like `api_users_get_v1`, `api_orders_create_v2`, etc.
static API_FUNCTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^api_(?P<name>[\w_]+)_(?P<method>create|delete|list|update|get)_(?P<version>v\d+)")
        .expect("Invalid API function regex")
});

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

    /// Determine subcategory based on function naming conventions.
    ///
    /// Note: This implementation uses the function name (passed as `name_or_content`)
    /// rather than content analysis, since function categorization is based on naming
    /// patterns rather than SQL syntax.
    ///
    /// Supported patterns:
    /// - `api_{name}_{method}_v{n}` → `api/{name}/v{n}`
    /// - `cast_*` → `casting`
    /// - `new_*` → `builder`
    /// - `util_*` → `utility`
    /// - `trigger_*` or `*_trigger` → `trigger`
    fn subcategory(name_or_content: &str) -> Option<String> {
        // Check for API function pattern
        if let Some(caps) = API_FUNCTION_PATTERN.captures(name_or_content) {
            let api_name = caps.name("name").map(|m| m.as_str()).unwrap_or("");
            let version = caps.name("version").map(|m| m.as_str()).unwrap_or("");
            return Some(format!("api/{api_name}/{version}"));
        }

        // Check for prefix-based categories
        if name_or_content.starts_with("cast_") {
            Some("casting".to_string())
        } else if name_or_content.starts_with("new_") {
            Some("builder".to_string())
        } else if name_or_content.starts_with("util_") {
            Some("utility".to_string())
        } else if name_or_content.starts_with("trigger_") || name_or_content.ends_with("_trigger") {
            Some("trigger".to_string())
        } else {
            None
        }
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");

        // Use subcategory based on function name
        let dir = match Self::subcategory(&obj.name) {
            Some(subcat) => base_dir.join(schema).join("functions").join(subcat),
            None => base_dir.join(schema).join("functions"),
        };

        dir.join(format!("{}.sql", obj.name))
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

    // Subcategory tests

    #[test]
    fn test_subcategory_none_for_regular_function() {
        assert_eq!(FunctionHandler::subcategory("calculate_total"), None);
        assert_eq!(FunctionHandler::subcategory("do_something"), None);
    }

    #[test]
    fn test_subcategory_api_function() {
        assert_eq!(
            FunctionHandler::subcategory("api_users_get_v1"),
            Some("api/users/v1".to_string())
        );
        assert_eq!(
            FunctionHandler::subcategory("api_orders_create_v2"),
            Some("api/orders/v2".to_string())
        );
        assert_eq!(
            FunctionHandler::subcategory("api_products_delete_v3"),
            Some("api/products/v3".to_string())
        );
    }

    #[test]
    fn test_subcategory_casting_function() {
        assert_eq!(
            FunctionHandler::subcategory("cast_to_text"),
            Some("casting".to_string())
        );
        assert_eq!(
            FunctionHandler::subcategory("cast_int_to_string"),
            Some("casting".to_string())
        );
    }

    #[test]
    fn test_subcategory_builder_function() {
        assert_eq!(
            FunctionHandler::subcategory("new_user"),
            Some("builder".to_string())
        );
        assert_eq!(
            FunctionHandler::subcategory("new_order"),
            Some("builder".to_string())
        );
    }

    #[test]
    fn test_subcategory_utility_function() {
        assert_eq!(
            FunctionHandler::subcategory("util_format_date"),
            Some("utility".to_string())
        );
    }

    #[test]
    fn test_subcategory_trigger_function() {
        assert_eq!(
            FunctionHandler::subcategory("trigger_audit"),
            Some("trigger".to_string())
        );
        assert_eq!(
            FunctionHandler::subcategory("user_audit_trigger"),
            Some("trigger".to_string())
        );
    }

    #[test]
    fn test_output_path_with_subcategory() {
        let obj = RawObject::new(
            ObjectType::Function,
            Some("public".to_string()),
            "api_users_get_v1".to_string(),
            "CREATE FUNCTION api_users_get_v1() ...".to_string(),
        );
        let path = FunctionHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/functions/api/users/v1/api_users_get_v1.sql")
        );
    }

    #[test]
    fn test_output_path_trigger_function() {
        let obj = RawObject::new(
            ObjectType::Function,
            Some("public".to_string()),
            "trigger_audit_log".to_string(),
            "CREATE FUNCTION trigger_audit_log() ...".to_string(),
        );
        let path = FunctionHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/functions/trigger/trigger_audit_log.sql")
        );
    }
}
