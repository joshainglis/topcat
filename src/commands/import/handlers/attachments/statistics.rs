//! Handler for PostgreSQL STATISTICS objects.

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
use crate::commands::import::sources::pg_dump::STATISTICS_PATTERN;

/// Handler for PostgreSQL STATISTICS objects.
///
/// Statistics objects are extended statistics collected on table columns.
/// These are created with `CREATE STATISTICS` and help the query planner
/// with correlated columns.
///
/// Statistics belong to the Append layer and attach to their parent table.
pub struct StatisticsHandler;

impl PatternProvider for StatisticsHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&STATISTICS_PATTERN]
    }
}

impl DependencyExtractor for StatisticsHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Extract table dependency from CREATE STATISTICS ... FROM table
        if let Some(caps) = STATISTICS_PATTERN.captures(content) {
            if let Some(table_name) = caps.name("table_name") {
                let schema = caps.name("table_schema").map(|m| m.as_str().to_string());
                let qualified = match schema {
                    Some(s) => format!("{}.{}", s, table_name.as_str()),
                    None => table_name.as_str().to_string(),
                };
                deps.push((qualified, ObjectType::Table));
            }
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Table]
    }
}

impl Categorizer for StatisticsHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TableAttachment
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Statistics attach to tables, so this is fallback only
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("statistics")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for StatisticsHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Statistics)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false
    }
}

impl Renderer for StatisticsHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for Statistics objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Statistics,
        StatisticsHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics_handler_layer() {
        assert_eq!(StatisticsHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_statistics_handler_is_not_primary() {
        assert!(!StatisticsHandler::is_primary());
    }

    #[test]
    fn test_statistics_handler_category() {
        assert_eq!(
            StatisticsHandler::category(),
            ObjectCategory::TableAttachment
        );
    }

    #[test]
    fn test_statistics_implicit_deps() {
        let deps = StatisticsHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
    }

    #[test]
    fn test_statistics_default_config() {
        let config = StatisticsHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_statistics_pattern_deps() {
        let content =
            r#"CREATE STATISTICS "public"."users_stats" ON col1, col2 FROM "public"."users";"#;
        let deps = StatisticsHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.users");
        assert_eq!(deps[0].1, ObjectType::Table);
    }

    #[test]
    fn test_statistics_output_path() {
        let obj = RawObject::new(
            ObjectType::Statistics,
            Some("public".to_string()),
            "users_stats".to_string(),
            "CREATE STATISTICS users_stats ON ...".to_string(),
        );
        let path = StatisticsHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/statistics/users_stats.sql")
        );
    }
}
