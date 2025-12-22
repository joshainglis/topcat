//! Handler for PostgreSQL OPERATOR objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;
use crate::commands::import::sources::pg_dump::{DEFAULT_SCHEMA_PATTERN, build_operator_pattern};

/// Handler for PostgreSQL OPERATOR objects.
///
/// Operators define custom operations between data types.
/// They are schema-qualified objects that depend on types and functions.
/// They belong to the Prepend layer as foundation objects.
pub struct OperatorHandler;

impl PatternProvider for OperatorHandler {
    // Operators are identified by metadata, not content patterns
}

impl DependencyExtractor for OperatorHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Operators depend on their types and function
        vec![
            ObjectType::Type,
            ObjectType::Function,
            ObjectType::OperatorFamily,
        ]
    }

    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];
        let pattern = build_operator_pattern(DEFAULT_SCHEMA_PATTERN);

        if let Some(caps) = pattern.captures(content) {
            // Extract function/procedure dependency
            if let Some(proc_name) = caps.name("procedure_name") {
                deps.push((proc_name.as_str().to_string(), ObjectType::Function));
            }

            // Extract left arg type dependency
            if let Some(left_arg) = caps.name("left_arg_name") {
                deps.push((left_arg.as_str().to_string(), ObjectType::Type));
            }

            // Extract right arg type dependency
            if let Some(right_arg) = caps.name("right_arg_name") {
                deps.push((right_arg.as_str().to_string(), ObjectType::Type));
            }
        }

        deps
    }
}

impl Categorizer for OperatorHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Operator
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("operator".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("pg_catalog");
        // Sanitize operator name for filesystem (operators can have special chars)
        let safe_name = sanitize_operator_name(&obj.name);
        base_dir
            .join("_global")
            .join("operator")
            .join(schema)
            .join(format!("{safe_name}.sql"))
    }
}

impl Configurable for OperatorHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Operator)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for OperatorHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Sanitize operator name for use as filename.
///
/// Operators can contain special characters like ||, &&, @>, etc.
/// These need to be converted to safe filename characters.
fn sanitize_operator_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '|' => "pipe".to_string(),
            '&' => "amp".to_string(),
            '@' => "at".to_string(),
            '>' => "gt".to_string(),
            '<' => "lt".to_string(),
            '=' => "eq".to_string(),
            '!' => "not".to_string(),
            '+' => "plus".to_string(),
            '-' => "minus".to_string(),
            '*' => "star".to_string(),
            '/' => "slash".to_string(),
            '%' => "pct".to_string(),
            '^' => "caret".to_string(),
            '~' => "tilde".to_string(),
            '#' => "hash".to_string(),
            '?' => "qmark".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// Extract pattern-based dependencies from Operator content.
fn extract_operator_dependencies(content: &str) -> Vec<(String, ObjectType)> {
    OperatorHandler::extract_pattern_dependencies(content)
}

/// Create a registered handler for Operator objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(ObjectType::Operator, extract_operator_dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_handler_layer() {
        assert_eq!(OperatorHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_operator_handler_is_primary() {
        assert!(OperatorHandler::is_primary());
    }

    #[test]
    fn test_operator_handler_category() {
        assert_eq!(OperatorHandler::category(), ObjectCategory::Operator);
    }

    #[test]
    fn test_operator_subcategory() {
        let content =
            r#"CREATE OPERATOR || (FUNCTION = textcat, LEFTARG = text, RIGHTARG = text);"#;
        assert_eq!(
            OperatorHandler::subcategory(content),
            Some("operator".to_string())
        );
    }

    #[test]
    fn test_operator_output_path() {
        let obj = RawObject::new(
            ObjectType::Operator,
            Some("pg_catalog".to_string()),
            "||".to_string(),
            r#"CREATE OPERATOR || (FUNCTION = textcat, LEFTARG = text, RIGHTARG = text);"#
                .to_string(),
        );
        let path = OperatorHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/operator/pg_catalog/pipepipe.sql")
        );
    }

    #[test]
    fn test_operator_output_path_custom_schema() {
        let obj = RawObject::new(
            ObjectType::Operator,
            Some("myapp".to_string()),
            "@>".to_string(),
            r#"CREATE OPERATOR @> (FUNCTION = contains, LEFTARG = jsonb, RIGHTARG = jsonb);"#
                .to_string(),
        );
        let path = OperatorHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/operator/myapp/atgt.sql")
        );
    }

    #[test]
    fn test_operator_implicit_dependencies() {
        let deps = OperatorHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::Function));
        assert!(deps.contains(&ObjectType::OperatorFamily));
    }

    #[test]
    fn test_sanitize_operator_name() {
        assert_eq!(sanitize_operator_name("||"), "pipepipe");
        assert_eq!(sanitize_operator_name("@>"), "atgt");
        assert_eq!(sanitize_operator_name("&&"), "ampamp");
        assert_eq!(sanitize_operator_name("="), "eq");
        assert_eq!(sanitize_operator_name("<>"), "ltgt");
        assert_eq!(sanitize_operator_name("my_op"), "my_op");
    }
}
