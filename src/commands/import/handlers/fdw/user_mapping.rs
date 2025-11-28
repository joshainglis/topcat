//! Handler for PostgreSQL USER MAPPING objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;
use crate::commands::import::sources::pg_dump::USER_MAPPING_PATTERN;

/// Handler for PostgreSQL USER MAPPING objects.
///
/// User mappings define how local users authenticate to foreign servers.
/// They are global objects (no schema) that depend on a server and belong
/// to the Prepend layer.
pub struct UserMappingHandler;

impl PatternProvider for UserMappingHandler {
    // User mappings are identified by metadata, not content patterns
}

impl DependencyExtractor for UserMappingHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // User mappings depend on their server
        vec![ObjectType::Server]
    }

    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Extract server dependency from CREATE USER MAPPING FOR user SERVER server_name
        if let Some(caps) = USER_MAPPING_PATTERN.captures(content) {
            if let Some(server) = caps.name("server") {
                deps.push((server.as_str().to_string(), ObjectType::Server));
            }
        }

        deps
    }
}

impl Categorizer for UserMappingHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::ForeignDataWrapper
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("user_mapping".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Global object: _global/fdw/user_mapping/mapping_name.sql
        // Note: obj.name typically includes the user name, e.g., "user_name"
        base_dir
            .join("_global")
            .join("fdw")
            .join("user_mapping")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for UserMappingHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::UserMapping)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for UserMappingHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Extract pattern-based dependencies from UserMapping content.
fn extract_server_dependency(content: &str) -> Vec<(String, ObjectType)> {
    UserMappingHandler::extract_pattern_dependencies(content)
}

/// Create a registered handler for UserMapping objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(ObjectType::UserMapping, extract_server_dependency)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_mapping_handler_layer() {
        assert_eq!(UserMappingHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_user_mapping_handler_is_primary() {
        assert!(UserMappingHandler::is_primary());
    }

    #[test]
    fn test_user_mapping_handler_category() {
        assert_eq!(
            UserMappingHandler::category(),
            ObjectCategory::ForeignDataWrapper
        );
    }

    #[test]
    fn test_user_mapping_subcategory() {
        let content = r#"CREATE USER MAPPING FOR "postgres" SERVER "remote_db";"#;
        assert_eq!(
            UserMappingHandler::subcategory(content),
            Some("user_mapping".to_string())
        );
    }

    #[test]
    fn test_user_mapping_output_path() {
        let obj = RawObject::new(
            ObjectType::UserMapping,
            None, // Global object - no schema
            "postgres".to_string(),
            r#"CREATE USER MAPPING FOR "postgres" SERVER "remote_db";"#.to_string(),
        );
        let path = UserMappingHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/fdw/user_mapping/postgres.sql")
        );
    }

    #[test]
    fn test_user_mapping_implicit_dependencies() {
        let deps = UserMappingHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Server));
    }

    #[test]
    fn test_user_mapping_extract_pattern_dependencies() {
        let content =
            r#"CREATE USER MAPPING FOR "postgres" SERVER "remote_db" OPTIONS (user 'admin');"#;
        let deps = UserMappingHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "remote_db");
        assert_eq!(deps[0].1, ObjectType::Server);
    }

    #[test]
    fn test_user_mapping_extract_pattern_dependencies_public() {
        let content = r#"CREATE USER MAPPING FOR PUBLIC SERVER remote_db;"#;
        let deps = UserMappingHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "remote_db");
        assert_eq!(deps[0].1, ObjectType::Server);
    }
}
