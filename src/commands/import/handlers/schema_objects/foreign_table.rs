//! Handler for PostgreSQL FOREIGN TABLE objects.

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

/// Pattern for matching CREATE FOREIGN TABLE statements.
pub static FOREIGN_TABLE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+FOREIGN\s+TABLE\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        \([^)]*\)\s*
        SERVER\s+
        "?(?P<server>[^"\s;]+)"?
        "#,
    )
    .expect("Invalid FOREIGN_TABLE_PATTERN regex")
});

/// Handler for PostgreSQL FOREIGN TABLE objects.
///
/// Foreign tables are tables whose data resides on external servers via
/// Foreign Data Wrappers. They belong to the Normal layer and depend on
/// their server as well as types used in column definitions.
pub struct ForeignTableHandler;

impl PatternProvider for ForeignTableHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&FOREIGN_TABLE_PATTERN]
    }
}

impl DependencyExtractor for ForeignTableHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = Vec::new();

        // Extract server dependency from FOREIGN TABLE definition
        if let Some(caps) = FOREIGN_TABLE_PATTERN.captures(content)
            && let Some(server) = caps.name("server")
        {
            deps.push((server.as_str().to_string(), ObjectType::Server));
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Type, ObjectType::Domain, ObjectType::Schema]
    }
}

impl Categorizer for ForeignTableHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::ForeignTable
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("foreign_table")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for ForeignTableHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::ForeignTable)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for ForeignTableHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Extract server dependency from foreign table content.
fn extract_server_dependency(content: &str) -> Vec<(String, ObjectType)> {
    ForeignTableHandler::extract_pattern_dependencies(content)
}

/// Create a registered handler for ForeignTable objects.
///
/// This handler includes pattern-based dependency extraction for the server.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(ObjectType::ForeignTable, extract_server_dependency)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foreign_table_pattern() {
        let input = r#"CREATE FOREIGN TABLE "public"."remote_users" (id int, name text) SERVER "remote_server" OPTIONS (...);"#;
        let caps = FOREIGN_TABLE_PATTERN.captures(input).unwrap();
        assert_eq!(caps.name("schema").unwrap().as_str(), "public");
        assert_eq!(caps.name("name").unwrap().as_str(), "remote_users");
        assert_eq!(caps.name("server").unwrap().as_str(), "remote_server");
    }

    #[test]
    fn test_foreign_table_pattern_no_schema() {
        let input = r#"CREATE FOREIGN TABLE remote_users (id int) SERVER my_server;"#;
        let caps = FOREIGN_TABLE_PATTERN.captures(input).unwrap();
        assert!(caps.name("schema").is_none());
        assert_eq!(caps.name("name").unwrap().as_str(), "remote_users");
        assert_eq!(caps.name("server").unwrap().as_str(), "my_server");
    }

    #[test]
    fn test_foreign_table_handler_layer() {
        assert_eq!(ForeignTableHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_foreign_table_handler_is_primary() {
        assert!(ForeignTableHandler::is_primary());
    }

    #[test]
    fn test_foreign_table_handler_category() {
        assert_eq!(
            ForeignTableHandler::category(),
            ObjectCategory::ForeignTable
        );
    }

    #[test]
    fn test_foreign_table_implicit_deps() {
        let deps = ForeignTableHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::Domain));
        assert!(deps.contains(&ObjectType::Schema));
    }

    #[test]
    fn test_foreign_table_pattern_deps() {
        let content =
            r#"CREATE FOREIGN TABLE "public"."remote_users" (id int) SERVER "my_server";"#;
        let deps = ForeignTableHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], ("my_server".to_string(), ObjectType::Server));
    }

    #[test]
    fn test_foreign_table_output_path() {
        let obj = RawObject::new(
            ObjectType::ForeignTable,
            Some("public".to_string()),
            "remote_users".to_string(),
            "CREATE FOREIGN TABLE remote_users ...;".to_string(),
        );
        let path = ForeignTableHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/foreign_table/remote_users.sql")
        );
    }

    #[test]
    fn test_create_handler_has_pattern_deps() {
        let handler = create_handler();
        let content = r#"CREATE FOREIGN TABLE ft (id int) SERVER srv;"#;
        let deps = handler.extract_pattern_deps(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "srv");
    }
}
