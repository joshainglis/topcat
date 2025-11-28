//! Handler for PostgreSQL OPERATOR CLASS objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::patterns::OPERATOR_CLASS_PATTERN;
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL OPERATOR CLASS objects.
///
/// Operator classes define how an index access method can be used with a
/// particular operator family for a particular data type.
/// They are schema-qualified objects that depend on types and access methods.
/// They belong to the Prepend layer as foundation objects.
pub struct OperatorClassHandler;

impl PatternProvider for OperatorClassHandler {
    // Operator classes are identified by metadata, not content patterns
}

impl DependencyExtractor for OperatorClassHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Operator classes depend on their data type and access method
        vec![ObjectType::Type, ObjectType::AccessMethod]
    }

    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        if let Some(caps) = OPERATOR_CLASS_PATTERN.captures(content) {
            // Extract the data type this operator class is for
            if let Some(type_name) = caps.name("type_name") {
                let full_type = if let Some(type_schema) = caps.name("type_schema") {
                    format!("{}.{}", type_schema.as_str(), type_name.as_str())
                } else {
                    type_name.as_str().to_string()
                };
                deps.push((full_type, ObjectType::Type));
            }

            // Extract the access method (e.g., btree, hash, gist)
            if let Some(method) = caps.name("method") {
                deps.push((method.as_str().to_string(), ObjectType::AccessMethod));
            }
        }

        deps
    }
}

impl Categorizer for OperatorClassHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Operator
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("class".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("pg_catalog");
        base_dir
            .join("_global")
            .join("operator")
            .join("class")
            .join(schema)
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for OperatorClassHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::OperatorClass)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for OperatorClassHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Extract pattern-based dependencies from OperatorClass content.
fn extract_opclass_dependencies(content: &str) -> Vec<(String, ObjectType)> {
    OperatorClassHandler::extract_pattern_dependencies(content)
}

/// Create a registered handler for OperatorClass objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(ObjectType::OperatorClass, extract_opclass_dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_class_handler_layer() {
        assert_eq!(OperatorClassHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_operator_class_handler_is_primary() {
        assert!(OperatorClassHandler::is_primary());
    }

    #[test]
    fn test_operator_class_handler_category() {
        assert_eq!(OperatorClassHandler::category(), ObjectCategory::Operator);
    }

    #[test]
    fn test_operator_class_subcategory() {
        let content = r#"CREATE OPERATOR CLASS text_pattern_ops FOR TYPE text USING btree AS ...;"#;
        assert_eq!(
            OperatorClassHandler::subcategory(content),
            Some("class".to_string())
        );
    }

    #[test]
    fn test_operator_class_output_path() {
        let obj = RawObject::new(
            ObjectType::OperatorClass,
            Some("pg_catalog".to_string()),
            "text_pattern_ops".to_string(),
            r#"CREATE OPERATOR CLASS text_pattern_ops FOR TYPE text USING btree;"#.to_string(),
        );
        let path = OperatorClassHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/operator/class/pg_catalog/text_pattern_ops.sql")
        );
    }

    #[test]
    fn test_operator_class_output_path_custom_schema() {
        let obj = RawObject::new(
            ObjectType::OperatorClass,
            Some("myapp".to_string()),
            "my_ops".to_string(),
            r#"CREATE OPERATOR CLASS my_ops FOR TYPE mytype USING hash;"#.to_string(),
        );
        let path = OperatorClassHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/operator/class/myapp/my_ops.sql")
        );
    }

    #[test]
    fn test_operator_class_implicit_dependencies() {
        let deps = OperatorClassHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::AccessMethod));
    }

    #[test]
    fn test_operator_class_extract_pattern_dependencies() {
        let content = r#"CREATE OPERATOR CLASS "pg_catalog"."text_pattern_ops" FOR TYPE "pg_catalog"."text" USING btree AS ...;"#;
        let deps = OperatorClassHandler::extract_pattern_dependencies(content);

        // Should extract the type dependency
        let has_type_dep = deps.iter().any(|(_, t)| *t == ObjectType::Type);
        assert!(has_type_dep, "Should extract type dependency");

        // Should extract the access method dependency
        let has_method_dep = deps
            .iter()
            .any(|(name, t)| *t == ObjectType::AccessMethod && name == "btree");
        assert!(has_method_dep, "Should extract access method dependency");
    }
}
