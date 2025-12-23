//! Handler for PostgreSQL DEFAULT objects.

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
use crate::commands::import::sources::pg_dump::DEFAULT_PATTERN;

/// Handler for PostgreSQL DEFAULT objects.
///
/// Default objects set default values for table columns. These are
/// `ALTER TABLE ... ALTER COLUMN ... SET DEFAULT` statements that
/// appear separately in pg_dump output.
///
/// Defaults belong to the Append layer and attach to their parent table.
#[allow(dead_code)] // Marker type for trait implementations
pub struct DefaultHandler;

impl PatternProvider for DefaultHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&DEFAULT_PATTERN]
    }
}

impl DependencyExtractor for DefaultHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = Vec::new();
        if let Some(caps) = DEFAULT_PATTERN.captures(content)
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
        vec![ObjectType::Table, ObjectType::Sequence]
    }
}

impl Categorizer for DefaultHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TableAttachment
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Defaults attach to tables, so this is fallback only
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("default")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for DefaultHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Default)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false
    }
}

impl Renderer for DefaultHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for Default objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Default,
        DefaultHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_handler_layer() {
        assert_eq!(DefaultHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_default_handler_is_not_primary() {
        assert!(!DefaultHandler::is_primary());
    }

    #[test]
    fn test_default_handler_category() {
        assert_eq!(DefaultHandler::category(), ObjectCategory::TableAttachment);
    }

    #[test]
    fn test_default_implicit_deps() {
        let deps = DefaultHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
        assert!(deps.contains(&ObjectType::Sequence));
    }

    #[test]
    fn test_default_default_config() {
        let config = DefaultHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_default_output_path() {
        let obj = RawObject::new(
            ObjectType::Default,
            Some("public".to_string()),
            "users_id".to_string(),
            "ALTER TABLE users ALTER COLUMN id SET DEFAULT nextval('users_id_seq');".to_string(),
        );
        let path = DefaultHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/default/users_id.sql"));
    }
}
