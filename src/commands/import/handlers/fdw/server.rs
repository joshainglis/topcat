//! Handler for PostgreSQL SERVER objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;
use crate::commands::import::sources::pg_dump::SERVER_PATTERN;

/// Handler for PostgreSQL SERVER objects.
///
/// Foreign servers define connection parameters for external data sources.
/// They are global objects (no schema) that depend on a foreign data wrapper
/// and belong to the Prepend layer.
pub struct ServerHandler;

impl PatternProvider for ServerHandler {
    // Servers are identified by metadata, not content patterns
}

impl DependencyExtractor for ServerHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Servers depend on their foreign data wrapper
        vec![ObjectType::ForeignDataWrapper]
    }

    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Extract FDW dependency from CREATE SERVER ... FOREIGN DATA WRAPPER fdw_name
        if let Some(caps) = SERVER_PATTERN.captures(content) {
            if let Some(fdw) = caps.name("fdw") {
                deps.push((fdw.as_str().to_string(), ObjectType::ForeignDataWrapper));
            }
        }

        deps
    }
}

impl Categorizer for ServerHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::ForeignDataWrapper
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("server".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Global object: _global/fdw/server/server_name.sql
        base_dir
            .join("_global")
            .join("fdw")
            .join("server")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for ServerHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Server)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for ServerHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Extract pattern-based dependencies from Server content.
fn extract_fdw_dependency(content: &str) -> Vec<(String, ObjectType)> {
    ServerHandler::extract_pattern_dependencies(content)
}

/// Create a registered handler for Server objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(ObjectType::Server, extract_fdw_dependency)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_handler_layer() {
        assert_eq!(ServerHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_server_handler_is_primary() {
        assert!(ServerHandler::is_primary());
    }

    #[test]
    fn test_server_handler_category() {
        assert_eq!(
            ServerHandler::category(),
            ObjectCategory::ForeignDataWrapper
        );
    }

    #[test]
    fn test_server_subcategory() {
        let content = r#"CREATE SERVER "remote_db" FOREIGN DATA WRAPPER "postgres_fdw";"#;
        assert_eq!(
            ServerHandler::subcategory(content),
            Some("server".to_string())
        );
    }

    #[test]
    fn test_server_output_path() {
        let obj = RawObject::new(
            ObjectType::Server,
            None, // Global object - no schema
            "remote_db".to_string(),
            r#"CREATE SERVER "remote_db" FOREIGN DATA WRAPPER "postgres_fdw";"#.to_string(),
        );
        let path = ServerHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/fdw/server/remote_db.sql")
        );
    }

    #[test]
    fn test_server_implicit_dependencies() {
        let deps = ServerHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::ForeignDataWrapper));
    }

    #[test]
    fn test_server_extract_pattern_dependencies() {
        let content = r#"CREATE SERVER "remote_db" FOREIGN DATA WRAPPER "postgres_fdw" OPTIONS (host 'localhost');"#;
        let deps = ServerHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "postgres_fdw");
        assert_eq!(deps[0].1, ObjectType::ForeignDataWrapper);
    }

    #[test]
    fn test_server_extract_pattern_dependencies_unquoted() {
        let content = r#"CREATE SERVER remote_db FOREIGN DATA WRAPPER postgres_fdw OPTIONS (host 'localhost');"#;
        let deps = ServerHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "postgres_fdw");
        assert_eq!(deps[0].1, ObjectType::ForeignDataWrapper);
    }
}
