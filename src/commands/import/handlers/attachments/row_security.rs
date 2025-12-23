//! Handler for PostgreSQL ROW SECURITY objects.

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
use crate::commands::import::sources::pg_dump::ROW_SECURITY_PATTERN;

/// Handler for PostgreSQL ROW SECURITY objects.
///
/// Row security objects enable or disable row-level security on tables.
/// These are `ALTER TABLE ... ENABLE/DISABLE ROW LEVEL SECURITY` statements.
///
/// Row security belongs to the Append layer and attaches to the parent table.
/// It must be enabled before policies can be applied.
#[allow(dead_code)] // Marker type for trait implementations
pub struct RowSecurityHandler;

impl PatternProvider for RowSecurityHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&ROW_SECURITY_PATTERN]
    }
}

impl DependencyExtractor for RowSecurityHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = Vec::new();
        if let Some(caps) = ROW_SECURITY_PATTERN.captures(content)
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
        vec![ObjectType::Table]
    }
}

impl Categorizer for RowSecurityHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Security
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Row security attaches to tables, so this is fallback only
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("row_security")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for RowSecurityHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::RowSecurity)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false
    }
}

impl Renderer for RowSecurityHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for RowSecurity objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::RowSecurity,
        RowSecurityHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_security_handler_layer() {
        assert_eq!(RowSecurityHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_row_security_handler_is_not_primary() {
        assert!(!RowSecurityHandler::is_primary());
    }

    #[test]
    fn test_row_security_handler_category() {
        assert_eq!(RowSecurityHandler::category(), ObjectCategory::Security);
    }

    #[test]
    fn test_row_security_implicit_deps() {
        let deps = RowSecurityHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
    }

    #[test]
    fn test_row_security_default_config() {
        let config = RowSecurityHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_row_security_output_path() {
        let obj = RawObject::new(
            ObjectType::RowSecurity,
            Some("public".to_string()),
            "users".to_string(),
            "ALTER TABLE users ENABLE ROW LEVEL SECURITY;".to_string(),
        );
        let path = RowSecurityHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/row_security/users.sql"));
    }
}
