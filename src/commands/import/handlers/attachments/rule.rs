//! Handler for PostgreSQL RULE objects.

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
use crate::commands::import::sources::pg_dump::RULE_PATTERN;

/// Handler for PostgreSQL RULE objects.
///
/// Rules define actions to be performed when queries are executed on tables
/// or views. They can transform or redirect queries.
///
/// Rules belong to the Append layer and attach to their parent table or view.
pub struct RuleHandler;

impl PatternProvider for RuleHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&RULE_PATTERN]
    }
}

impl DependencyExtractor for RuleHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Extract table/view dependency from CREATE RULE ... ON table
        if let Some(caps) = RULE_PATTERN.captures(content)
            && let Some(table_name) = caps.name("table_name")
        {
            let schema = caps.name("schema").map(|m| m.as_str().to_string());
            let qualified = match schema {
                Some(s) => format!("{}.{}", s, table_name.as_str()),
                None => table_name.as_str().to_string(),
            };
            // Rules can apply to tables or views
            deps.push((qualified, ObjectType::Table));
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![ObjectType::Table, ObjectType::View]
    }
}

impl Categorizer for RuleHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::TableAttachment
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Rules attach to tables/views, so this is fallback only
        let schema = obj.schema.as_deref().unwrap_or("public");
        base_dir
            .join(schema)
            .join("rule")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for RuleHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Rule)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false
    }
}

impl Renderer for RuleHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for Rule objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Rule,
        RuleHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_handler_layer() {
        assert_eq!(RuleHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_rule_handler_is_not_primary() {
        assert!(!RuleHandler::is_primary());
    }

    #[test]
    fn test_rule_handler_category() {
        assert_eq!(RuleHandler::category(), ObjectCategory::TableAttachment);
    }

    #[test]
    fn test_rule_implicit_deps() {
        let deps = RuleHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
        assert!(deps.contains(&ObjectType::View));
    }

    #[test]
    fn test_rule_default_config() {
        let config = RuleHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_rule_pattern_deps() {
        let content = r#"CREATE RULE my_rule AS ON INSERT TO "public"."users" DO INSTEAD NOTHING;"#;
        let deps = RuleHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.users");
        assert_eq!(deps[0].1, ObjectType::Table);
    }

    #[test]
    fn test_rule_output_path() {
        let obj = RawObject::new(
            ObjectType::Rule,
            Some("public".to_string()),
            "users_insert_rule".to_string(),
            "CREATE RULE users_insert_rule AS ON INSERT TO users ...".to_string(),
        );
        let path = RuleHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/rule/users_insert_rule.sql")
        );
    }
}
