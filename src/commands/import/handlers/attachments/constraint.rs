//! Handler for PostgreSQL CONSTRAINT objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL CONSTRAINT objects.
///
/// Constraints are attachment objects that belong to a table. They include
/// CHECK, UNIQUE, PRIMARY KEY, and EXCLUDE constraints. Foreign key
/// constraints are handled separately by [`FkConstraintHandler`].
///
/// Constraints belong to the Append layer and attach to their parent table.
pub struct ConstraintHandler;

impl PatternProvider for ConstraintHandler {
    // Constraints are identified by metadata, not content patterns
}

impl DependencyExtractor for ConstraintHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Table]
    }
}

impl Categorizer for ConstraintHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TableAttachment
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Constraints attach to tables, so this is fallback only
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("constraint")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for ConstraintHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Constraint)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false
    }
}

impl Renderer for ConstraintHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for Constraint objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::Constraint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_handler_layer() {
        assert_eq!(ConstraintHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_constraint_handler_is_not_primary() {
        assert!(!ConstraintHandler::is_primary());
    }

    #[test]
    fn test_constraint_handler_category() {
        assert_eq!(
            ConstraintHandler::category(),
            ObjectCategory::TableAttachment
        );
    }

    #[test]
    fn test_constraint_implicit_deps() {
        let deps = ConstraintHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
    }

    #[test]
    fn test_constraint_default_config() {
        let config = ConstraintHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_constraint_output_path() {
        let obj = RawObject::new(
            ObjectType::Constraint,
            Some("public".to_string()),
            "users_email_check".to_string(),
            "ALTER TABLE users ADD CONSTRAINT users_email_check CHECK (email IS NOT NULL);"
                .to_string(),
        );
        let path = ConstraintHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/constraint/users_email_check.sql")
        );
    }
}
