//! Handler for PostgreSQL EVENT TRIGGER objects.
//!
//! Event triggers fire on database-level events like DDL commands.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;
use crate::commands::import::sources::pg_dump::EVENT_TRIGGER_PATTERN;

/// Handler for PostgreSQL EVENT TRIGGER objects.
///
/// Event triggers fire on database-level events such as DDL commands.
/// They are global objects (no schema) that belong to the Normal layer
/// since they depend on the functions they execute.
///
/// Examples:
/// - `CREATE EVENT TRIGGER "audit_ddl" ON ddl_command_end EXECUTE FUNCTION audit_ddl_func();`
/// - `CREATE EVENT TRIGGER "prevent_drop" ON sql_drop WHEN TAG IN ('DROP TABLE') EXECUTE FUNCTION prevent_drop_func();`
pub struct EventTriggerHandler;

impl PatternProvider for EventTriggerHandler {
    // Event triggers are identified by metadata, not content patterns
}

impl DependencyExtractor for EventTriggerHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Extract function dependency from EXECUTE FUNCTION clause
        if let Some(caps) = EVENT_TRIGGER_PATTERN.captures(content) {
            if let Some(fn_name) = caps.name("fn_name") {
                let qualified = if let Some(schema) = caps.name("fn_schema") {
                    format!("{}.{}", schema.as_str(), fn_name.as_str())
                } else {
                    fn_name.as_str().to_string()
                };
                deps.push((qualified, ObjectType::Function));
            }
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Event triggers depend on functions
        vec![ObjectType::Function]
    }
}

impl Categorizer for EventTriggerHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::EventTrigger
    }

    fn subcategory(_content: &str) -> Option<String> {
        None
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Global object: _global/event_trigger/{name}.sql
        base_dir
            .join("_global")
            .join("event_trigger")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for EventTriggerHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::EventTrigger)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for EventTriggerHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for EventTrigger objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::EventTrigger,
        EventTriggerHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_trigger_handler_layer() {
        assert_eq!(EventTriggerHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_event_trigger_handler_is_primary() {
        assert!(EventTriggerHandler::is_primary());
    }

    #[test]
    fn test_event_trigger_handler_category() {
        assert_eq!(
            EventTriggerHandler::category(),
            ObjectCategory::EventTrigger
        );
    }

    #[test]
    fn test_event_trigger_subcategory() {
        let content =
            r#"CREATE EVENT TRIGGER "audit_ddl" ON ddl_command_end EXECUTE FUNCTION audit();"#;
        assert_eq!(EventTriggerHandler::subcategory(content), None);
    }

    #[test]
    fn test_event_trigger_output_path() {
        let obj = RawObject::new(
            ObjectType::EventTrigger,
            None, // Global object - no schema
            "audit_ddl".to_string(),
            r#"CREATE EVENT TRIGGER "audit_ddl" ON ddl_command_end EXECUTE FUNCTION "public"."audit_func"();"#.to_string(),
        );
        let path = EventTriggerHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/event_trigger/audit_ddl.sql")
        );
    }

    #[test]
    fn test_event_trigger_implicit_dependencies() {
        let deps = EventTriggerHandler::implicit_dependency_types();
        assert_eq!(deps.len(), 1);
        assert!(deps.contains(&ObjectType::Function));
    }

    #[test]
    fn test_event_trigger_pattern_deps_with_schema() {
        let content = r#"CREATE EVENT TRIGGER audit_ddl ON ddl_command_end EXECUTE FUNCTION "public"."audit_func"()"#;
        let deps = EventTriggerHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.audit_func");
        assert_eq!(deps[0].1, ObjectType::Function);
    }

    #[test]
    fn test_event_trigger_pattern_deps_without_schema() {
        let content =
            r#"CREATE EVENT TRIGGER audit_ddl ON ddl_command_end EXECUTE FUNCTION audit_func()"#;
        let deps = EventTriggerHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "audit_func");
        assert_eq!(deps[0].1, ObjectType::Function);
    }

    #[test]
    fn test_event_trigger_pattern_deps_with_when_clause() {
        let content = r#"CREATE EVENT TRIGGER prevent_drop ON sql_drop WHEN TAG IN ('DROP TABLE') EXECUTE FUNCTION "public"."prevent_drop_func"()"#;
        let deps = EventTriggerHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.prevent_drop_func");
        assert_eq!(deps[0].1, ObjectType::Function);
    }
}
