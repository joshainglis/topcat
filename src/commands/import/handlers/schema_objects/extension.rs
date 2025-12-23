//! Handler for PostgreSQL EXTENSION objects.

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

/// Pattern for matching CREATE EXTENSION statements.
pub static EXTENSION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        CREATE\s+EXTENSION(?:\s+IF\s+NOT\s+EXISTS)?\s+
        "(?P<ext_name>[^"]+)"
        (?:\s+WITH\s+SCHEMA\s+"(?P<ext_schema>[^"]+)")?"#,
    )
    .expect("Invalid EXTENSION_PATTERN regex")
});

/// Handler for PostgreSQL EXTENSION objects.
///
/// Extensions are pre-packaged modules that add functionality to PostgreSQL.
/// They are foundation objects in the Prepend layer.
#[allow(dead_code)] // Marker type for trait implementations
pub struct ExtensionHandler;

impl PatternProvider for ExtensionHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&EXTENSION_PATTERN]
    }
}

impl DependencyExtractor for ExtensionHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        // Reference pattern to ensure it's used, even if no deps extracted
        let _ = EXTENSION_PATTERN.is_match(content);
        vec![]
    }
    // Extensions don't have implicit dependencies from SQL content
    // (they may depend on each other but this is handled by PostgreSQL)
}

impl Categorizer for ExtensionHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Extension
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Extensions go in a global _extensions directory
        base_dir
            .join("_extensions")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for ExtensionHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Extension)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for ExtensionHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Extension objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Extension,
        ExtensionHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_pattern() {
        let input = r#"CREATE EXTENSION IF NOT EXISTS "pgcrypto" WITH SCHEMA "public";"#;
        let caps = EXTENSION_PATTERN.captures(input).unwrap();
        assert_eq!(caps.name("ext_name").unwrap().as_str(), "pgcrypto");
        assert_eq!(caps.name("ext_schema").unwrap().as_str(), "public");
    }

    #[test]
    fn test_extension_pattern_no_schema() {
        let input = r#"CREATE EXTENSION "uuid-ossp";"#;
        let caps = EXTENSION_PATTERN.captures(input).unwrap();
        assert_eq!(caps.name("ext_name").unwrap().as_str(), "uuid-ossp");
        assert!(caps.name("ext_schema").is_none());
    }

    #[test]
    fn test_extension_handler_layer() {
        assert_eq!(ExtensionHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_extension_handler_category() {
        assert_eq!(ExtensionHandler::category(), ObjectCategory::Extension);
    }

    #[test]
    fn test_extension_output_path() {
        let obj = RawObject::new(
            ObjectType::Extension,
            None,
            "pgcrypto".to_string(),
            "CREATE EXTENSION pgcrypto;".to_string(),
        );
        let path = ExtensionHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/_extensions/pgcrypto.sql"));
    }
}
