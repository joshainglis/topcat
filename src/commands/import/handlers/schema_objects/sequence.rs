//! Handler for PostgreSQL SEQUENCE objects.

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
use crate::commands::import::sources::pg_dump::SEQUENCE_PATTERN;

/// Handler for PostgreSQL SEQUENCE objects.
///
/// Sequences are auto-incrementing number generators. They belong to the
/// Normal layer and are often owned by tables (for SERIAL columns).
#[allow(dead_code)] // Marker type for trait implementations
pub struct SequenceHandler;

impl PatternProvider for SequenceHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&SEQUENCE_PATTERN]
    }
}

impl DependencyExtractor for SequenceHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        // Reference pattern to ensure it's used, even if no deps extracted
        let _ = SEQUENCE_PATTERN.is_match(content);
        vec![]
    }
    // Sequences don't have implicit dependencies
    // (ownership by tables is handled separately)
}

impl Categorizer for SequenceHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Sequence
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("sequence")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for SequenceHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Sequence)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for SequenceHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Sequence objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Sequence,
        SequenceHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence_pattern() {
        let input = r#"CREATE SEQUENCE "public"."users_id_seq" START 1;"#;
        let caps = SEQUENCE_PATTERN.captures(input).unwrap();
        assert_eq!(caps.name("schema").unwrap().as_str(), "public");
        assert_eq!(caps.name("name").unwrap().as_str(), "users_id_seq");
    }

    #[test]
    fn test_sequence_pattern_no_schema() {
        let input = r#"CREATE SEQUENCE users_id_seq;"#;
        let caps = SEQUENCE_PATTERN.captures(input).unwrap();
        assert!(caps.name("schema").is_none());
        assert_eq!(caps.name("name").unwrap().as_str(), "users_id_seq");
    }

    #[test]
    fn test_sequence_handler_layer() {
        assert_eq!(SequenceHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_sequence_handler_is_primary() {
        assert!(SequenceHandler::is_primary());
    }

    #[test]
    fn test_sequence_handler_category() {
        assert_eq!(SequenceHandler::category(), ObjectCategory::Sequence);
    }

    #[test]
    fn test_sequence_output_path() {
        let obj = RawObject::new(
            ObjectType::Sequence,
            Some("public".to_string()),
            "users_id_seq".to_string(),
            "CREATE SEQUENCE users_id_seq;".to_string(),
        );
        let path = SequenceHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/sequence/users_id_seq.sql")
        );
    }
}
