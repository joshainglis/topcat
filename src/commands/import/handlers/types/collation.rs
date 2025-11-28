//! Handler for PostgreSQL COLLATION objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL COLLATION objects.
///
/// Collations define string comparison and sorting rules. For example:
/// `CREATE COLLATION french (LOCALE = 'fr_FR.utf8');`
///
/// Collations belong to the Prepend layer as they are foundational objects
/// that columns and indexes may depend on.
pub struct CollationHandler;

impl PatternProvider for CollationHandler {
    // Collations are identified by metadata, not content patterns
}

impl DependencyExtractor for CollationHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Schema]
    }
}

impl Categorizer for CollationHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Collation
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("collation")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for CollationHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Collation)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for CollationHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Collation objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::Collation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collation_handler_layer() {
        assert_eq!(CollationHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_collation_handler_is_primary() {
        assert!(CollationHandler::is_primary());
    }

    #[test]
    fn test_collation_handler_category() {
        assert_eq!(CollationHandler::category(), ObjectCategory::Collation);
    }

    #[test]
    fn test_collation_implicit_deps() {
        let deps = CollationHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Schema));
    }

    #[test]
    fn test_collation_output_path() {
        let obj = RawObject::new(
            ObjectType::Collation,
            Some("public".to_string()),
            "french".to_string(),
            "CREATE COLLATION french (LOCALE = 'fr_FR.utf8');".to_string(),
        );
        let path = CollationHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/collation/french.sql"));
    }

    #[test]
    fn test_collation_output_path_no_schema() {
        let obj = RawObject::new(
            ObjectType::Collation,
            None,
            "custom_coll".to_string(),
            "CREATE COLLATION custom_coll (...);".to_string(),
        );
        let path = CollationHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/collation/custom_coll.sql")
        );
    }
}
