//! Handler for PostgreSQL FK CONSTRAINT objects.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::pg_dump::FK_CONSTRAINT_PATTERN;
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL FK CONSTRAINT objects.
///
/// Foreign key constraints are attachment objects that belong to a table.
/// They reference columns in other tables and enforce referential integrity.
///
/// FK constraints belong to the Append layer and attach to their parent table.
/// They are created after regular constraints to ensure referenced tables exist.
pub struct FkConstraintHandler;

impl PatternProvider for FkConstraintHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&FK_CONSTRAINT_PATTERN]
    }
}

impl DependencyExtractor for FkConstraintHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Table]
    }
}

impl Categorizer for FkConstraintHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TableAttachment
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // FK constraints attach to tables, so this is fallback only
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("fk_constraint")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for FkConstraintHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::FkConstraint)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false
    }
}

impl Renderer for FkConstraintHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for FkConstraint objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::FkConstraint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fk_constraint_handler_layer() {
        assert_eq!(FkConstraintHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_fk_constraint_handler_is_not_primary() {
        assert!(!FkConstraintHandler::is_primary());
    }

    #[test]
    fn test_fk_constraint_handler_category() {
        assert_eq!(
            FkConstraintHandler::category(),
            ObjectCategory::TableAttachment
        );
    }

    #[test]
    fn test_fk_constraint_implicit_deps() {
        let deps = FkConstraintHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
    }

    #[test]
    fn test_fk_constraint_default_config() {
        let config = FkConstraintHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_fk_constraint_output_path() {
        let obj = RawObject::new(
            ObjectType::FkConstraint,
            Some("public".to_string()),
            "orders_user_fk".to_string(),
            "ALTER TABLE orders ADD CONSTRAINT orders_user_fk FOREIGN KEY (user_id) REFERENCES users(id);".to_string(),
        );
        let path = FkConstraintHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/fk_constraint/orders_user_fk.sql")
        );
    }
}
