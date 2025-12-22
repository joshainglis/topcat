//! Handler for PostgreSQL VIEW objects.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::pg_dump::VIEW_PATTERN;
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL VIEW objects.
///
/// Views are virtual tables defined by a query. They belong to the Normal
/// layer and depend on the tables, views, and functions they reference.
pub struct ViewHandler;

impl PatternProvider for ViewHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&VIEW_PATTERN]
    }
}

impl DependencyExtractor for ViewHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Table, ObjectType::View, ObjectType::Function]
    }
}

impl Categorizer for ViewHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::View
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("view")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for ViewHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::View)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for ViewHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for View objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::View)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_handler_layer() {
        assert_eq!(ViewHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_view_handler_is_primary() {
        assert!(ViewHandler::is_primary());
    }

    #[test]
    fn test_view_handler_category() {
        assert_eq!(ViewHandler::category(), ObjectCategory::View);
    }

    #[test]
    fn test_view_implicit_deps() {
        let deps = ViewHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
        assert!(deps.contains(&ObjectType::View));
        assert!(deps.contains(&ObjectType::Function));
    }

    #[test]
    fn test_view_output_path() {
        let obj = RawObject::new(
            ObjectType::View,
            Some("public".to_string()),
            "active_users".to_string(),
            "CREATE VIEW active_users AS SELECT ...;".to_string(),
        );
        let path = ViewHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/view/active_users.sql"));
    }
}
