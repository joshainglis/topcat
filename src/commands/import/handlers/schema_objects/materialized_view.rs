//! Handler for PostgreSQL MATERIALIZED VIEW objects.

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
use crate::commands::import::sources::pg_dump::MATERIALIZED_VIEW_PATTERN;

/// Handler for PostgreSQL MATERIALIZED VIEW objects.
///
/// Materialized views are views that cache their results. They belong to the
/// Normal layer and depend on the tables, views, and functions they reference.
#[allow(dead_code)] // Marker type for trait implementations
pub struct MaterializedViewHandler;

impl PatternProvider for MaterializedViewHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&MATERIALIZED_VIEW_PATTERN]
    }
}

impl DependencyExtractor for MaterializedViewHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        // Reference pattern to ensure it's used, even if no deps extracted
        let _ = MATERIALIZED_VIEW_PATTERN.is_match(content);
        vec![]
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Table, ObjectType::View, ObjectType::Function]
    }
}

impl Categorizer for MaterializedViewHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::MaterializedView
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("materialized_view")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for MaterializedViewHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::MaterializedView)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for MaterializedViewHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for MaterializedView objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::MaterializedView,
        MaterializedViewHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_materialized_view_pattern() {
        let input = r#"CREATE MATERIALIZED VIEW "public"."mv_users" AS SELECT * FROM users;"#;
        let caps = MATERIALIZED_VIEW_PATTERN.captures(input).unwrap();
        assert_eq!(caps.name("schema").unwrap().as_str(), "public");
        assert_eq!(caps.name("name").unwrap().as_str(), "mv_users");
    }

    #[test]
    fn test_materialized_view_pattern_no_schema() {
        let input = r#"CREATE MATERIALIZED VIEW mv_users AS SELECT * FROM users;"#;
        let caps = MATERIALIZED_VIEW_PATTERN.captures(input).unwrap();
        assert!(caps.name("schema").is_none());
        assert_eq!(caps.name("name").unwrap().as_str(), "mv_users");
    }

    #[test]
    fn test_materialized_view_handler_layer() {
        assert_eq!(MaterializedViewHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_materialized_view_handler_category() {
        assert_eq!(
            MaterializedViewHandler::category(),
            ObjectCategory::MaterializedView
        );
    }

    #[test]
    fn test_materialized_view_implicit_deps() {
        let deps = MaterializedViewHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
        assert!(deps.contains(&ObjectType::View));
        assert!(deps.contains(&ObjectType::Function));
    }

    #[test]
    fn test_materialized_view_output_path() {
        let obj = RawObject::new(
            ObjectType::MaterializedView,
            Some("analytics".to_string()),
            "daily_stats".to_string(),
            "CREATE MATERIALIZED VIEW daily_stats AS ...;".to_string(),
        );
        let path = MaterializedViewHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/analytics/materialized_view/daily_stats.sql")
        );
    }
}
