//! Handler for PostgreSQL FOREIGN DATA WRAPPER objects.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::pg_dump::FDW_PATTERN;
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL FOREIGN DATA WRAPPER objects.
///
/// Foreign data wrappers provide the mechanism for accessing external data sources.
/// They are global objects (no schema) and belong to the Prepend layer as they are
/// foundation objects that servers and foreign tables depend on.
pub struct WrapperHandler;

impl PatternProvider for WrapperHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&FDW_PATTERN]
    }
}

impl DependencyExtractor for WrapperHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // FDW is a foundation type with no implicit dependencies
        vec![]
    }
}

impl Categorizer for WrapperHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::ForeignDataWrapper
    }

    fn subcategory(_content: &str) -> Option<String> {
        None
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Global object: _global/fdw/wrapper_name.sql
        base_dir
            .join("_global")
            .join("fdw")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for WrapperHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::ForeignDataWrapper)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for WrapperHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for ForeignDataWrapper objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::ForeignDataWrapper)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrapper_handler_layer() {
        assert_eq!(WrapperHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_wrapper_handler_is_primary() {
        assert!(WrapperHandler::is_primary());
    }

    #[test]
    fn test_wrapper_handler_category() {
        assert_eq!(
            WrapperHandler::category(),
            ObjectCategory::ForeignDataWrapper
        );
    }

    #[test]
    fn test_wrapper_subcategory() {
        let content = r#"CREATE FOREIGN DATA WRAPPER "postgres_fdw" HANDLER postgres_fdw_handler;"#;
        assert_eq!(WrapperHandler::subcategory(content), None);
    }

    #[test]
    fn test_wrapper_output_path() {
        let obj = RawObject::new(
            ObjectType::ForeignDataWrapper,
            None, // Global object - no schema
            "postgres_fdw".to_string(),
            r#"CREATE FOREIGN DATA WRAPPER "postgres_fdw" HANDLER postgres_fdw_handler;"#
                .to_string(),
        );
        let path = WrapperHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/_global/fdw/postgres_fdw.sql"));
    }

    #[test]
    fn test_wrapper_implicit_dependencies() {
        let deps = WrapperHandler::implicit_dependency_types();
        assert!(deps.is_empty()); // FDW has no implicit deps
    }
}
