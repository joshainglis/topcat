//! Handler for PostgreSQL DEFAULT ACL objects.
//!
//! Default ACL objects represent ALTER DEFAULT PRIVILEGES statements that
//! define default access control for newly created objects.

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
use crate::commands::import::sources::pg_dump::DEFAULT_ACL_PATTERN;

/// Handler for PostgreSQL DEFAULT ACL objects.
///
/// Default ACLs define the default privileges that will be granted
/// on newly created objects. They are standalone objects (not attachments)
/// that belong to the Append layer.
///
/// Example:
/// `ALTER DEFAULT PRIVILEGES FOR ROLE "admin" IN SCHEMA "public" GRANT SELECT ON TABLES TO "reader";`
pub struct DefaultAclHandler;

impl PatternProvider for DefaultAclHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&DEFAULT_ACL_PATTERN]
    }
}

impl DependencyExtractor for DefaultAclHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Default ACLs reference schemas
        if let Some(caps) = DEFAULT_ACL_PATTERN.captures(content)
            && let Some(schema) = caps.name("schema")
        {
            deps.push((schema.as_str().to_string(), ObjectType::Schema));
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Default ACLs only depend on schemas
        vec![ObjectType::Schema]
    }
}

impl Categorizer for DefaultAclHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Security
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("default_acl".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Default ACLs are stored under the schema if specified, or globally
        // {schema}/_acl/default/{name}.sql or _global/_acl/default/{name}.sql
        match &obj.schema {
            Some(schema) => base_dir
                .join(schema)
                .join("_acl")
                .join("default")
                .join(format!("{}.sql", obj.name)),
            None => base_dir
                .join("_global")
                .join("_acl")
                .join("default")
                .join(format!("{}.sql", obj.name)),
        }
    }
}

impl Configurable for DefaultAclHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::DefaultAcl)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false // Default ACLs are not primary objects
    }
}

impl Renderer for DefaultAclHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for DefaultAcl objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::DefaultAcl,
        DefaultAclHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_acl_handler_layer() {
        assert_eq!(DefaultAclHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_default_acl_handler_is_not_primary() {
        assert!(!DefaultAclHandler::is_primary());
    }

    #[test]
    fn test_default_acl_handler_category() {
        assert_eq!(DefaultAclHandler::category(), ObjectCategory::Security);
    }

    #[test]
    fn test_default_acl_subcategory() {
        let content =
            r#"ALTER DEFAULT PRIVILEGES IN SCHEMA "public" GRANT SELECT ON TABLES TO "reader";"#;
        assert_eq!(
            DefaultAclHandler::subcategory(content),
            Some("default_acl".to_string())
        );
    }

    #[test]
    fn test_default_acl_default_config() {
        let config = DefaultAclHandler::default_config();
        // DefaultAcl is not in the attach_to_parent list in ObjectTypeConfig
        assert!(!config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_default_acl_pattern_deps_with_schema() {
        let content =
            r#"ALTER DEFAULT PRIVILEGES IN SCHEMA "public" GRANT SELECT ON TABLES TO "reader";"#;
        let deps = DefaultAclHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public");
        assert_eq!(deps[0].1, ObjectType::Schema);
    }

    #[test]
    fn test_default_acl_pattern_deps_with_role() {
        let content = r#"ALTER DEFAULT PRIVILEGES FOR ROLE "admin" IN SCHEMA "myschema" GRANT ALL ON TABLES TO "admin";"#;
        let deps = DefaultAclHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "myschema");
        assert_eq!(deps[0].1, ObjectType::Schema);
    }

    #[test]
    fn test_default_acl_pattern_deps_no_schema() {
        // Default privileges without schema specification
        let content =
            r#"ALTER DEFAULT PRIVILEGES FOR ROLE "admin" GRANT SELECT ON TABLES TO "reader";"#;
        let deps = DefaultAclHandler::extract_pattern_dependencies(content);
        assert!(deps.is_empty());
    }

    #[test]
    fn test_default_acl_output_path_with_schema() {
        let obj = RawObject::new(
            ObjectType::DefaultAcl,
            Some("public".to_string()),
            "default_tables".to_string(),
            r#"ALTER DEFAULT PRIVILEGES IN SCHEMA "public" GRANT SELECT ON TABLES TO "reader";"#
                .to_string(),
        );
        let path = DefaultAclHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/_acl/default/default_tables.sql")
        );
    }

    #[test]
    fn test_default_acl_output_path_global() {
        let obj = RawObject::new(
            ObjectType::DefaultAcl,
            None,
            "global_defaults".to_string(),
            r#"ALTER DEFAULT PRIVILEGES GRANT SELECT ON TABLES TO "reader";"#.to_string(),
        );
        let path = DefaultAclHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/_acl/default/global_defaults.sql")
        );
    }

    #[test]
    fn test_default_acl_implicit_deps() {
        let deps = DefaultAclHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Schema));
        assert_eq!(deps.len(), 1);
    }
}
