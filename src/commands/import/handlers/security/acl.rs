//! Handler for PostgreSQL ACL (Access Control List) objects.
//!
//! ACL objects represent GRANT and REVOKE statements that control access
//! to database objects.

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
use crate::commands::import::sources::pg_dump::{GRANT_PATTERN, REVOKE_PATTERN};

/// Handler for PostgreSQL ACL (GRANT/REVOKE) objects.
///
/// ACL statements control access privileges on database objects.
/// They belong to the Append layer and attach to their target objects.
///
/// Examples:
/// - `GRANT SELECT ON TABLE "schema"."table" TO "role";`
/// - `REVOKE ALL ON FUNCTION "schema"."func"() FROM PUBLIC;`
pub struct AclHandler;

impl PatternProvider for AclHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&GRANT_PATTERN, &REVOKE_PATTERN]
    }
}

impl DependencyExtractor for AclHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Try GRANT pattern first
        if let Some(caps) = GRANT_PATTERN.captures(content) {
            if let Some(dep) = extract_target_dependency(&caps) {
                deps.push(dep);
            }
        }
        // Try REVOKE pattern
        else if let Some(caps) = REVOKE_PATTERN.captures(content)
            && let Some(dep) = extract_target_dependency(&caps)
        {
            deps.push(dep);
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        // ACLs can reference many object types
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
            ObjectType::ForeignDataWrapper,
            ObjectType::Server,
            ObjectType::Language,
        ]
    }
}

/// Extract the target object dependency from GRANT/REVOKE captures.
fn extract_target_dependency(caps: &regex::Captures) -> Option<(String, ObjectType)> {
    let obj_name = caps.name("obj_name")?.as_str();
    let obj_schema = caps.name("obj_schema").map(|m| m.as_str());
    let obj_type_str = caps.name("obj_type").map(|m| m.as_str().trim());

    let qualified = match obj_schema {
        Some(schema) => format!("{schema}.{obj_name}"),
        None => obj_name.to_string(),
    };

    // Map the object type string to ObjectType
    let obj_type = match obj_type_str {
        Some(t) => match t.to_uppercase().as_str() {
            "TABLE" => ObjectType::Table,
            "SEQUENCE" => ObjectType::Sequence,
            "FUNCTION" => ObjectType::Function,
            "PROCEDURE" => ObjectType::Procedure,
            "SCHEMA" => ObjectType::Schema,
            "TYPE" => ObjectType::Type,
            "DOMAIN" => ObjectType::Domain,
            "FOREIGN DATA WRAPPER" => ObjectType::ForeignDataWrapper,
            "FOREIGN SERVER" => ObjectType::Server,
            "LANGUAGE" => ObjectType::Language,
            _ => ObjectType::Table, // Default to table
        },
        None => ObjectType::Table, // Default to table
    };

    Some((qualified, obj_type))
}

impl Categorizer for AclHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Security
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("acl".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // ACLs attach to their target objects, but if standalone:
        // {schema}/_acl/{name}.sql or _global/_acl/{name}.sql
        match &obj.schema {
            Some(schema) => base_dir
                .join(schema)
                .join("_acl")
                .join(format!("{}.sql", obj.name)),
            None => base_dir
                .join("_global")
                .join("_acl")
                .join(format!("{}.sql", obj.name)),
        }
    }
}

impl Configurable for AclHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Acl)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false // ACLs are attachments
    }
}

impl Renderer for AclHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for ACL objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(ObjectType::Acl, AclHandler::extract_pattern_dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acl_handler_layer() {
        assert_eq!(AclHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_acl_handler_is_not_primary() {
        assert!(!AclHandler::is_primary());
    }

    #[test]
    fn test_acl_handler_category() {
        assert_eq!(AclHandler::category(), ObjectCategory::Security);
    }

    #[test]
    fn test_acl_subcategory() {
        let content = r#"GRANT SELECT ON TABLE "public"."users" TO "app_role";"#;
        assert_eq!(AclHandler::subcategory(content), Some("acl".to_string()));
    }

    #[test]
    fn test_acl_default_config() {
        let config = AclHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_grant_pattern_deps() {
        let content = r#"GRANT SELECT ON TABLE "public"."users" TO "app_role";"#;
        let deps = AclHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.users");
        assert_eq!(deps[0].1, ObjectType::Table);
    }

    #[test]
    fn test_grant_function_deps() {
        let content = r#"GRANT EXECUTE ON FUNCTION "public"."my_func"() TO "app_role";"#;
        let deps = AclHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.my_func");
        assert_eq!(deps[0].1, ObjectType::Function);
    }

    #[test]
    fn test_revoke_pattern_deps() {
        let content = r#"REVOKE ALL ON TABLE "public"."secrets" FROM PUBLIC;"#;
        let deps = AclHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.secrets");
        assert_eq!(deps[0].1, ObjectType::Table);
    }

    #[test]
    fn test_acl_output_path_with_schema() {
        let obj = RawObject::new(
            ObjectType::Acl,
            Some("public".to_string()),
            "users_acl".to_string(),
            r#"GRANT SELECT ON TABLE "public"."users" TO "reader";"#.to_string(),
        );
        let path = AclHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/_acl/users_acl.sql"));
    }

    #[test]
    fn test_acl_output_path_global() {
        let obj = RawObject::new(
            ObjectType::Acl,
            None,
            "schema_acl".to_string(),
            r#"GRANT USAGE ON SCHEMA "public" TO "app_role";"#.to_string(),
        );
        let path = AclHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/_global/_acl/schema_acl.sql"));
    }

    #[test]
    fn test_acl_implicit_deps() {
        let deps = AclHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
        assert!(deps.contains(&ObjectType::Function));
        assert!(deps.contains(&ObjectType::Schema));
    }
}
