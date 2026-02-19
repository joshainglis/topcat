//! Handler for PostgreSQL POLICY objects.

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
use crate::commands::import::sources::pg_dump::POLICY_PATTERN;

/// Handler for PostgreSQL POLICY objects.
///
/// Policies define row-level security rules for tables. They determine
/// which rows can be accessed by different users or roles.
///
/// Policies belong to the Append layer and attach to their parent table.
/// They are created after row security is enabled on the table.
pub struct PolicyHandler;

impl PatternProvider for PolicyHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&POLICY_PATTERN]
    }
}

impl DependencyExtractor for PolicyHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = Vec::new();
        if let Some(caps) = POLICY_PATTERN.captures(content)
            && let (Some(schema), Some(table)) = (caps.name("schema"), caps.name("table"))
        {
            deps.push((
                format!("{}.{}", schema.as_str(), table.as_str()),
                ObjectType::Table,
            ));
        }
        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Table, ObjectType::Function]
    }
}

impl Categorizer for PolicyHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Security
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Policies attach to tables, so this is fallback only
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("policy")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for PolicyHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Policy)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false
    }
}

impl Renderer for PolicyHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for Policy objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Policy,
        PolicyHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_handler_layer() {
        assert_eq!(PolicyHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_policy_handler_is_not_primary() {
        assert!(!PolicyHandler::is_primary());
    }

    #[test]
    fn test_policy_handler_category() {
        assert_eq!(PolicyHandler::category(), ObjectCategory::Security);
    }

    #[test]
    fn test_policy_implicit_deps() {
        let deps = PolicyHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
        assert!(deps.contains(&ObjectType::Function));
    }

    #[test]
    fn test_policy_default_config() {
        let config = PolicyHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_policy_output_path() {
        let obj = RawObject::new(
            ObjectType::Policy,
            Some("public".to_string()),
            "users_access_policy".to_string(),
            "CREATE POLICY users_access_policy ON users ...".to_string(),
        );
        let path = PolicyHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/policy/users_access_policy.sql")
        );
    }
}
