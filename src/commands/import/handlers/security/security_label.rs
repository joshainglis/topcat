//! Handler for PostgreSQL SECURITY LABEL objects.
//!
//! Security labels are used by label-based mandatory access control (MAC)
//! systems like SELinux.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::patterns::SECURITY_LABEL_PATTERN;
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL SECURITY LABEL objects.
///
/// Security labels are used by label-based mandatory access control systems
/// like SELinux to apply security contexts to database objects.
/// They belong to the Append layer and attach to their target objects.
///
/// Example:
/// `SECURITY LABEL FOR "selinux" ON TABLE "public"."users" IS 'system_u:object_r:sepgsql_table_t:s0';`
pub struct SecurityLabelHandler;

impl PatternProvider for SecurityLabelHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&SECURITY_LABEL_PATTERN]
    }
}

impl DependencyExtractor for SecurityLabelHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        if let Some(caps) = SECURITY_LABEL_PATTERN.captures(content) {
            let obj_name = caps.name("obj_name").map(|m| m.as_str());
            let obj_schema = caps.name("obj_schema").map(|m| m.as_str());
            let obj_type_str = caps.name("obj_type").map(|m| m.as_str().trim());

            if let Some(name) = obj_name {
                let qualified = match obj_schema {
                    Some(schema) => format!("{}.{}", schema, name),
                    None => name.to_string(),
                };

                // Map the object type string to ObjectType
                let obj_type = match obj_type_str {
                    Some(t) => match t.to_uppercase().as_str() {
                        "TABLE" => ObjectType::Table,
                        "VIEW" => ObjectType::View,
                        "MATERIALIZED VIEW" => ObjectType::MaterializedView,
                        "SEQUENCE" => ObjectType::Sequence,
                        "FUNCTION" => ObjectType::Function,
                        "PROCEDURE" => ObjectType::Procedure,
                        "SCHEMA" => ObjectType::Schema,
                        "TYPE" => ObjectType::Type,
                        "DOMAIN" => ObjectType::Domain,
                        "COLUMN" => ObjectType::Table, // Columns belong to tables
                        _ => ObjectType::Table,        // Default to table
                    },
                    None => ObjectType::Table,
                };

                deps.push((qualified, obj_type));
            }
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Security labels can reference many object types
        vec![
            ObjectType::Table,
            ObjectType::View,
            ObjectType::MaterializedView,
            ObjectType::Sequence,
            ObjectType::Function,
            ObjectType::Procedure,
            ObjectType::Schema,
            ObjectType::Type,
            ObjectType::Domain,
        ]
    }
}

impl Categorizer for SecurityLabelHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Security
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("security_label".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Security labels attach to target objects, but if standalone:
        // {schema}/_security_label/{name}.sql or _global/_security_label/{name}.sql
        match &obj.schema {
            Some(schema) => base_dir
                .join(schema)
                .join("_security_label")
                .join(format!("{}.sql", obj.name)),
            None => base_dir
                .join("_global")
                .join("_security_label")
                .join(format!("{}.sql", obj.name)),
        }
    }
}

impl Configurable for SecurityLabelHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::SecurityLabel)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false // Security labels are attachments
    }
}

impl Renderer for SecurityLabelHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for SecurityLabel objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::SecurityLabel,
        SecurityLabelHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_label_handler_layer() {
        assert_eq!(SecurityLabelHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_security_label_handler_is_not_primary() {
        assert!(!SecurityLabelHandler::is_primary());
    }

    #[test]
    fn test_security_label_handler_category() {
        assert_eq!(SecurityLabelHandler::category(), ObjectCategory::Security);
    }

    #[test]
    fn test_security_label_subcategory() {
        let content = r#"SECURITY LABEL FOR "selinux" ON TABLE "public"."users" IS 'system_u:object_r:sepgsql_table_t:s0';"#;
        assert_eq!(
            SecurityLabelHandler::subcategory(content),
            Some("security_label".to_string())
        );
    }

    #[test]
    fn test_security_label_default_config() {
        let config = SecurityLabelHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_security_label_pattern_deps_table() {
        let content = r#"SECURITY LABEL FOR "selinux" ON TABLE "public"."users" IS 'system_u:object_r:sepgsql_table_t:s0'"#;
        let deps = SecurityLabelHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.users");
        assert_eq!(deps[0].1, ObjectType::Table);
    }

    #[test]
    fn test_security_label_pattern_deps_function() {
        let content = r#"SECURITY LABEL FOR "selinux" ON FUNCTION "public"."my_func" IS 'system_u:object_r:sepgsql_proc_exec_t:s0'"#;
        let deps = SecurityLabelHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.my_func");
        assert_eq!(deps[0].1, ObjectType::Function);
    }

    #[test]
    fn test_security_label_pattern_deps_schema() {
        let content = r#"SECURITY LABEL FOR "selinux" ON SCHEMA public IS 'system_u:object_r:sepgsql_schema_t:s0'"#;
        let deps = SecurityLabelHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public");
        assert_eq!(deps[0].1, ObjectType::Schema);
    }

    #[test]
    fn test_security_label_output_path_with_schema() {
        let obj = RawObject::new(
            ObjectType::SecurityLabel,
            Some("public".to_string()),
            "users_label".to_string(),
            r#"SECURITY LABEL FOR "selinux" ON TABLE "public"."users" IS 'label';"#.to_string(),
        );
        let path = SecurityLabelHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/_security_label/users_label.sql")
        );
    }

    #[test]
    fn test_security_label_output_path_global() {
        let obj = RawObject::new(
            ObjectType::SecurityLabel,
            None,
            "schema_label".to_string(),
            r#"SECURITY LABEL FOR "selinux" ON SCHEMA "public" IS 'label';"#.to_string(),
        );
        let path = SecurityLabelHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/_security_label/schema_label.sql")
        );
    }

    #[test]
    fn test_security_label_implicit_deps() {
        let deps = SecurityLabelHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
        assert!(deps.contains(&ObjectType::Function));
        assert!(deps.contains(&ObjectType::Schema));
    }
}
