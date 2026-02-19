//! Handler for PostgreSQL AGGREGATE objects.

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
use crate::commands::import::sources::pg_dump::AGGREGATE_PATTERN;

/// Handler for PostgreSQL AGGREGATE objects.
///
/// Aggregates are special functions that compute a result from a set of input
/// values, like `SUM()`, `AVG()`, or custom aggregates.
///
/// Aggregates belong to the Normal layer and depend on types (for parameters/
/// return type) and other functions (for their state transition functions).
pub struct AggregateHandler;

impl PatternProvider for AggregateHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&AGGREGATE_PATTERN]
    }
}

impl DependencyExtractor for AggregateHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        // Reference pattern to ensure it's used, even if no deps extracted
        let _ = AGGREGATE_PATTERN.is_match(content);
        vec![]
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Aggregates depend on types and typically on helper functions
        vec![ObjectType::Type, ObjectType::Domain, ObjectType::Extension]
    }
}

impl Categorizer for AggregateHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Function
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("functions")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for AggregateHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Aggregate)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for AggregateHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Aggregate objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Aggregate,
        AggregateHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_handler_layer() {
        assert_eq!(AggregateHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_aggregate_handler_is_primary() {
        assert!(AggregateHandler::is_primary());
    }

    #[test]
    fn test_aggregate_handler_category() {
        assert_eq!(AggregateHandler::category(), ObjectCategory::Function);
    }

    #[test]
    fn test_aggregate_implicit_deps() {
        let deps = AggregateHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::Domain));
        assert!(deps.contains(&ObjectType::Extension));
    }

    #[test]
    fn test_aggregate_output_path() {
        let obj = RawObject::new(
            ObjectType::Aggregate,
            Some("public".to_string()),
            "array_agg_custom".to_string(),
            "CREATE AGGREGATE array_agg_custom (anyelement) (...)".to_string(),
        );
        let path = AggregateHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/functions/array_agg_custom.sql")
        );
    }

    #[test]
    fn test_aggregate_output_path_no_schema() {
        let obj = RawObject::new(
            ObjectType::Aggregate,
            None,
            "my_agg".to_string(),
            "CREATE AGGREGATE my_agg (integer) (...)".to_string(),
        );
        let path = AggregateHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/functions/my_agg.sql"));
    }
}
