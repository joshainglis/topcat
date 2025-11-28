//! Handler for PostgreSQL OPERATOR FAMILY objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::patterns::OPERATOR_FAMILY_PATTERN;
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL OPERATOR FAMILY objects.
///
/// Operator families define collections of related operator classes.
/// They are schema-qualified objects that depend on access methods.
/// They belong to the Prepend layer as foundation objects.
pub struct OperatorFamilyHandler;

impl PatternProvider for OperatorFamilyHandler {
    // Operator families are identified by metadata, not content patterns
}

impl DependencyExtractor for OperatorFamilyHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Operator families depend on their access method
        vec![ObjectType::Type, ObjectType::AccessMethod]
    }

    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        if let Some(caps) = OPERATOR_FAMILY_PATTERN.captures(content) {
            // Extract the access method (e.g., btree, hash, gist)
            if let Some(method) = caps.name("method") {
                deps.push((method.as_str().to_string(), ObjectType::AccessMethod));
            }
        }

        deps
    }
}

impl Categorizer for OperatorFamilyHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Operator
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("family".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        let schema = obj.schema.as_deref().unwrap_or("pg_catalog");
        base_dir
            .join("_global")
            .join("operator")
            .join("family")
            .join(schema)
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for OperatorFamilyHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::OperatorFamily)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for OperatorFamilyHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Extract pattern-based dependencies from OperatorFamily content.
fn extract_opfamily_dependencies(content: &str) -> Vec<(String, ObjectType)> {
    OperatorFamilyHandler::extract_pattern_dependencies(content)
}

/// Create a registered handler for OperatorFamily objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(ObjectType::OperatorFamily, extract_opfamily_dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_family_handler_layer() {
        assert_eq!(OperatorFamilyHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_operator_family_handler_is_primary() {
        assert!(OperatorFamilyHandler::is_primary());
    }

    #[test]
    fn test_operator_family_handler_category() {
        assert_eq!(OperatorFamilyHandler::category(), ObjectCategory::Operator);
    }

    #[test]
    fn test_operator_family_subcategory() {
        let content = r#"CREATE OPERATOR FAMILY text_ops USING btree;"#;
        assert_eq!(
            OperatorFamilyHandler::subcategory(content),
            Some("family".to_string())
        );
    }

    #[test]
    fn test_operator_family_output_path() {
        let obj = RawObject::new(
            ObjectType::OperatorFamily,
            Some("pg_catalog".to_string()),
            "text_ops".to_string(),
            r#"CREATE OPERATOR FAMILY text_ops USING btree;"#.to_string(),
        );
        let path = OperatorFamilyHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/operator/family/pg_catalog/text_ops.sql")
        );
    }

    #[test]
    fn test_operator_family_output_path_custom_schema() {
        let obj = RawObject::new(
            ObjectType::OperatorFamily,
            Some("myapp".to_string()),
            "my_family".to_string(),
            r#"CREATE OPERATOR FAMILY my_family USING hash;"#.to_string(),
        );
        let path = OperatorFamilyHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/operator/family/myapp/my_family.sql")
        );
    }

    #[test]
    fn test_operator_family_implicit_dependencies() {
        let deps = OperatorFamilyHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Type));
        assert!(deps.contains(&ObjectType::AccessMethod));
    }

    #[test]
    fn test_operator_family_extract_pattern_dependencies() {
        let content = r#"CREATE OPERATOR FAMILY "pg_catalog"."integer_ops" USING btree;"#;
        let deps = OperatorFamilyHandler::extract_pattern_dependencies(content);

        // Should extract the access method dependency
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "btree");
        assert_eq!(deps[0].1, ObjectType::AccessMethod);
    }
}
