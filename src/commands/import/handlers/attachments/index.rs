//! Handler for PostgreSQL INDEX objects.

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
use crate::commands::import::sources::pg_dump::INDEX_PATTERN;

/// Handler for PostgreSQL INDEX objects.
///
/// Indexes are attachment objects that belong to a table. They are created
/// after the table and provide optimized access paths for queries.
///
/// Indexes belong to the Append layer and attach to their parent table.
pub struct IndexHandler;

impl PatternProvider for IndexHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&INDEX_PATTERN]
    }
}

impl DependencyExtractor for IndexHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = Vec::new();
        if let Some(caps) = INDEX_PATTERN.captures(content)
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
        vec![ObjectType::Table, ObjectType::OperatorClass]
    }
}

impl Categorizer for IndexHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TableAttachment
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Indexes attach to tables, so this is fallback only
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("index")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for IndexHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Index)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false
    }
}

impl Renderer for IndexHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for Index objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Index,
        IndexHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_handler_layer() {
        assert_eq!(IndexHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_index_handler_is_not_primary() {
        assert!(!IndexHandler::is_primary());
    }

    #[test]
    fn test_index_handler_category() {
        assert_eq!(IndexHandler::category(), ObjectCategory::TableAttachment);
    }

    #[test]
    fn test_index_implicit_deps() {
        let deps = IndexHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
        assert!(deps.contains(&ObjectType::OperatorClass));
    }

    #[test]
    fn test_index_default_config() {
        let config = IndexHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_index_output_path() {
        let obj = RawObject::new(
            ObjectType::Index,
            Some("public".to_string()),
            "users_pkey".to_string(),
            "CREATE INDEX users_pkey ON users (id);".to_string(),
        );
        let path = IndexHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/index/users_pkey.sql"));
    }
}
