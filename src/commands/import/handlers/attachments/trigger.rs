//! Handler for PostgreSQL TRIGGER objects.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, ObjectHandler, OutputConfig, PatternProvider,
    RelatedObjects, Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;
use crate::commands::import::sources::pg_dump::TRIGGER_PATTERN;

/// Handler for PostgreSQL TRIGGER objects.
///
/// Triggers are attachment objects that fire in response to table events.
/// They execute a function when specific conditions are met.
///
/// Triggers belong to the Append layer and primarily attach to their
/// trigger function, with a secondary reference to the table.
pub struct TriggerHandler;

impl PatternProvider for TriggerHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&TRIGGER_PATTERN]
    }
}

impl DependencyExtractor for TriggerHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Extract function dependency from EXECUTE FUNCTION/PROCEDURE
        if let Some(caps) = TRIGGER_PATTERN.captures(content)
            && let (Some(fn_schema), Some(fn_name)) = (caps.name("fn_schema"), caps.name("fn_name"))
        {
            let qualified = format!("{}.{}", fn_schema.as_str(), fn_name.as_str());
            deps.push((qualified, ObjectType::Function));
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Function, ObjectType::Table]
    }
}

impl Categorizer for TriggerHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TableAttachment
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Triggers attach to functions, so this is fallback only
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("trigger")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for TriggerHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Trigger)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false
    }
}

impl Renderer for TriggerHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

impl ObjectHandler for TriggerHandler {
    fn object_type() -> ObjectType {
        ObjectType::Trigger
    }
}

/// Create a registered handler for Trigger objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Trigger,
        TriggerHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_handler_layer() {
        assert_eq!(TriggerHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_trigger_handler_is_not_primary() {
        assert!(!TriggerHandler::is_primary());
    }

    #[test]
    fn test_trigger_handler_category() {
        assert_eq!(TriggerHandler::category(), ObjectCategory::TableAttachment);
    }

    #[test]
    fn test_trigger_implicit_deps() {
        let deps = TriggerHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Function));
        assert!(deps.contains(&ObjectType::Table));
    }

    #[test]
    fn test_trigger_default_config() {
        let config = TriggerHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_trigger_pattern_deps() {
        let content = r#"CREATE TRIGGER my_trigger AFTER INSERT ON "public"."users" FOR EACH ROW EXECUTE FUNCTION "public"."notify_insert"()"#;
        let deps = TriggerHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.notify_insert");
        assert_eq!(deps[0].1, ObjectType::Function);
    }

    #[test]
    fn test_trigger_output_path() {
        let obj = RawObject::new(
            ObjectType::Trigger,
            Some("public".to_string()),
            "user_audit_trigger".to_string(),
            "CREATE TRIGGER user_audit_trigger ...".to_string(),
        );
        let path = TriggerHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/trigger/user_audit_trigger.sql")
        );
    }
}
