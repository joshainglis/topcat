//! Handler for PostgreSQL COMMENT objects.
//!
//! Comments are attached to database objects to provide documentation.

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
use crate::commands::import::sources::pg_dump::COMMENT_PATTERN;

/// Handler for PostgreSQL COMMENT objects.
///
/// Comments provide documentation for database objects.
/// They belong to the Append layer and attach to their target objects.
///
/// Examples:
/// - `COMMENT ON TABLE "public"."users" IS 'User accounts';`
/// - `COMMENT ON COLUMN "public"."users"."email" IS 'User email address';`
/// - `COMMENT ON FUNCTION "public"."my_func"(int) IS 'Processes integers';`
pub struct CommentHandler;

impl PatternProvider for CommentHandler {
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![&COMMENT_PATTERN]
    }
}

impl DependencyExtractor for CommentHandler {
    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        if let Some(caps) = COMMENT_PATTERN.captures(content) {
            let tgt_name = caps.name("tgt_name").map(|m| m.as_str());
            let tgt_schema = caps.name("tgt_schema").map(|m| m.as_str());
            let tgt_type_str = caps.name("tgt_type").map(|m| m.as_str().trim());

            if let Some(name) = tgt_name {
                let qualified = match tgt_schema {
                    Some(schema) => format!("{}.{}", schema, name),
                    None => name.to_string(),
                };

                // Map the target type string to ObjectType
                let obj_type = match tgt_type_str {
                    Some(t) => match t.to_uppercase().as_str() {
                        "TABLE" => ObjectType::Table,
                        "VIEW" => ObjectType::View,
                        "MATERIALIZED VIEW" => ObjectType::MaterializedView,
                        "COLUMN" => ObjectType::Table, // Columns belong to tables
                        "SEQUENCE" => ObjectType::Sequence,
                        "FUNCTION" => ObjectType::Function,
                        "PROCEDURE" => ObjectType::Procedure,
                        "AGGREGATE" => ObjectType::Aggregate,
                        "SCHEMA" => ObjectType::Schema,
                        "TYPE" => ObjectType::Type,
                        "DOMAIN" => ObjectType::Domain,
                        "COLLATION" => ObjectType::Collation,
                        "CONSTRAINT" => ObjectType::Table, // Constraints belong to tables
                        "INDEX" => ObjectType::Table,      // Indexes belong to tables
                        "TRIGGER" => ObjectType::Table,    // Triggers belong to tables
                        "RULE" => ObjectType::Table,       // Rules belong to tables
                        "POLICY" => ObjectType::Table,     // Policies belong to tables
                        "EXTENSION" => ObjectType::Extension,
                        "OPERATOR" => ObjectType::Operator,
                        "TEXT SEARCH CONFIGURATION" => ObjectType::TextSearchConfiguration,
                        "TEXT SEARCH DICTIONARY" => ObjectType::TextSearchDictionary,
                        "TEXT SEARCH PARSER" => ObjectType::TextSearchParser,
                        "TEXT SEARCH TEMPLATE" => ObjectType::TextSearchTemplate,
                        "SERVER" => ObjectType::Server,
                        "FOREIGN DATA WRAPPER" => ObjectType::ForeignDataWrapper,
                        "FOREIGN TABLE" => ObjectType::ForeignTable,
                        "PUBLICATION" => ObjectType::Publication,
                        "SUBSCRIPTION" => ObjectType::Subscription,
                        "EVENT TRIGGER" => ObjectType::EventTrigger,
                        "LANGUAGE" | "PROCEDURAL LANGUAGE" => ObjectType::Language,
                        "CAST" => ObjectType::Cast,
                        "CONVERSION" => ObjectType::Conversion,
                        "ACCESS METHOD" => ObjectType::AccessMethod,
                        _ => ObjectType::Table, // Default to table
                    },
                    None => ObjectType::Table,
                };

                deps.push((qualified, obj_type));
            }
        }

        deps
    }

    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Comments can reference many object types
        vec![
            ObjectType::Table,
            ObjectType::View,
            ObjectType::MaterializedView,
            ObjectType::Sequence,
            ObjectType::Function,
            ObjectType::Procedure,
            ObjectType::Aggregate,
            ObjectType::Schema,
            ObjectType::Type,
            ObjectType::Domain,
            ObjectType::Extension,
        ]
    }
}

impl Categorizer for CommentHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Comment
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("comment".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Comments attach to target objects, but if standalone:
        // {schema}/_comment/{name}.sql or _global/_comment/{name}.sql
        match &obj.schema {
            Some(schema) => base_dir
                .join(schema)
                .join("_comment")
                .join(format!("{}.sql", obj.name)),
            None => base_dir
                .join("_global")
                .join("_comment")
                .join(format!("{}.sql", obj.name)),
        }
    }
}

impl Configurable for CommentHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Comment)
    }

    fn layer() -> Layer {
        Layer::Append
    }

    fn is_primary() -> bool {
        false // Comments are attachments
    }
}

impl Renderer for CommentHandler {
    fn render(obj: &RawObject, _related: &RelatedObjects, _config: &OutputConfig) -> String {
        obj.content.clone()
    }
}

/// Create a registered handler for Comment objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Comment,
        CommentHandler::extract_pattern_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_handler_layer() {
        assert_eq!(CommentHandler::layer(), Layer::Append);
    }

    #[test]
    fn test_comment_handler_is_not_primary() {
        assert!(!CommentHandler::is_primary());
    }

    #[test]
    fn test_comment_handler_category() {
        assert_eq!(CommentHandler::category(), ObjectCategory::Comment);
    }

    #[test]
    fn test_comment_subcategory() {
        let content = r#"COMMENT ON TABLE "public"."users" IS 'User accounts';"#;
        assert_eq!(
            CommentHandler::subcategory(content),
            Some("comment".to_string())
        );
    }

    #[test]
    fn test_comment_default_config() {
        let config = CommentHandler::default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }

    #[test]
    fn test_comment_pattern_deps_table() {
        let content = r#"COMMENT ON TABLE "public"."users" IS 'User accounts'"#;
        let deps = CommentHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.users");
        assert_eq!(deps[0].1, ObjectType::Table);
    }

    #[test]
    fn test_comment_pattern_deps_column() {
        let content = r#"COMMENT ON COLUMN "public"."users"."email" IS 'User email'"#;
        let deps = CommentHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        // Column comments depend on the table
        assert_eq!(deps[0].1, ObjectType::Table);
    }

    #[test]
    fn test_comment_pattern_deps_function() {
        let content = r#"COMMENT ON FUNCTION "public"."my_func"(int) IS 'Processes integers'"#;
        let deps = CommentHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public.my_func");
        assert_eq!(deps[0].1, ObjectType::Function);
    }

    #[test]
    fn test_comment_pattern_deps_schema() {
        let content = r#"COMMENT ON SCHEMA "public" IS 'Default schema'"#;
        let deps = CommentHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "public");
        assert_eq!(deps[0].1, ObjectType::Schema);
    }

    #[test]
    fn test_comment_output_path_with_schema() {
        let obj = RawObject::new(
            ObjectType::Comment,
            Some("public".to_string()),
            "users_comment".to_string(),
            r#"COMMENT ON TABLE "public"."users" IS 'User accounts';"#.to_string(),
        );
        let path = CommentHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/public/_comment/users_comment.sql")
        );
    }

    #[test]
    fn test_comment_output_path_global() {
        let obj = RawObject::new(
            ObjectType::Comment,
            None,
            "schema_comment".to_string(),
            r#"COMMENT ON SCHEMA "public" IS 'Default schema';"#.to_string(),
        );
        let path = CommentHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/_comment/schema_comment.sql")
        );
    }

    #[test]
    fn test_comment_implicit_deps() {
        let deps = CommentHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Table));
        assert!(deps.contains(&ObjectType::Function));
        assert!(deps.contains(&ObjectType::Schema));
    }
}
