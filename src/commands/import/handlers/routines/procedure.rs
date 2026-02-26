//! Handler for PostgreSQL PROCEDURE objects.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::{routine_implicit_dependency_types, routine_output_path, routine_render};
use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;
use crate::commands::import::sources::pg_dump::PROCEDURE_PATTERN;

/// Handler for PostgreSQL PROCEDURE objects.
///
/// Procedures are callable objects that do not return a value (introduced in
/// PostgreSQL 11). They support transaction control (COMMIT/ROLLBACK) within
/// their body, unlike functions.
///
/// Procedures belong to the Normal layer and share the same dependencies as
/// functions.
pub struct ProcedureHandler;

impl PatternProvider for ProcedureHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&PROCEDURE_PATTERN]
    }
}

impl DependencyExtractor for ProcedureHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        routine_implicit_dependency_types()
    }
}

impl Categorizer for ProcedureHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Function
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        routine_output_path(obj, base_dir)
    }
}

impl Configurable for ProcedureHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Procedure)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for ProcedureHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        routine_render(obj, related, config)
    }
}

/// Create a registered handler for Procedure objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::Procedure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedure_handler_layer() {
        assert_eq!(ProcedureHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_procedure_handler_is_primary() {
        assert!(ProcedureHandler::is_primary());
    }

    #[test]
    fn test_procedure_handler_category() {
        assert_eq!(ProcedureHandler::category(), ObjectCategory::Function);
    }

    #[test]
    fn test_procedure_implicit_deps() {
        let deps = ProcedureHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::Domain));
        assert!(deps.contains(&ObjectType::Extension));
    }

    #[test]
    fn test_procedure_output_path() {
        let obj = RawObject::new(
            ObjectType::Procedure,
            Some("public".to_string()),
            "process_batch".to_string(),
            "CREATE PROCEDURE process_batch() LANGUAGE plpgsql AS $$...".to_string(),
        );
        let path = ProcedureHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/functions/process_batch.sql")
        );
    }

    #[test]
    fn test_procedure_output_path_no_schema() {
        let obj = RawObject::new(
            ObjectType::Procedure,
            None,
            "cleanup".to_string(),
            "CREATE PROCEDURE cleanup() ...".to_string(),
        );
        let path = ProcedureHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/functions/cleanup.sql"));
    }
}
