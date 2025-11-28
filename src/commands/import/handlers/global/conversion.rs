//! Handler for PostgreSQL CONVERSION objects.
//!
//! Conversions define how to convert character encodings.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;

/// Handler for PostgreSQL CONVERSION objects.
///
/// Conversions define how to convert between character encodings.
/// Unlike most global objects, conversions are schema-qualified.
/// They belong to the Prepend layer.
///
/// Examples:
/// - `CREATE CONVERSION "public"."myconv" FOR 'UTF8' TO 'LATIN1' FROM utf8_to_iso8859_1;`
/// - `CREATE DEFAULT CONVERSION "public"."mydefconv" FOR 'UTF8' TO 'SJIS' FROM utf8_to_sjis;`
pub struct ConversionHandler;

impl PatternProvider for ConversionHandler {
    // Conversions are identified by metadata, not content patterns
}

impl DependencyExtractor for ConversionHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Conversions don't have typical object dependencies
        // (the conversion functions are usually built-in)
        vec![]
    }

    fn extract_pattern_dependencies(_content: &str) -> Vec<(String, ObjectType)> {
        // Conversions reference built-in conversion functions
        // We don't track these as dependencies
        vec![]
    }
}

impl Categorizer for ConversionHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Conversion
    }

    fn subcategory(_content: &str) -> Option<String> {
        None
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Schema-qualified: {schema}/conversion/{name}.sql
        match &obj.schema {
            Some(schema) => base_dir
                .join(schema)
                .join("conversion")
                .join(format!("{}.sql", obj.name)),
            None => base_dir
                .join("_global")
                .join("conversion")
                .join(format!("{}.sql", obj.name)),
        }
    }
}

impl Configurable for ConversionHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Conversion)
    }

    fn layer() -> Layer {
        Layer::Prepend
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for ConversionHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Create a registered handler for Conversion objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::new(ObjectType::Conversion)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_handler_layer() {
        assert_eq!(ConversionHandler::layer(), Layer::Prepend);
    }

    #[test]
    fn test_conversion_handler_is_primary() {
        assert!(ConversionHandler::is_primary());
    }

    #[test]
    fn test_conversion_handler_category() {
        assert_eq!(ConversionHandler::category(), ObjectCategory::Conversion);
    }

    #[test]
    fn test_conversion_subcategory() {
        let content =
            r#"CREATE CONVERSION "myconv" FOR 'UTF8' TO 'LATIN1' FROM utf8_to_iso8859_1;"#;
        assert_eq!(ConversionHandler::subcategory(content), None);
    }

    #[test]
    fn test_conversion_output_path_with_schema() {
        let obj = RawObject::new(
            ObjectType::Conversion,
            Some("public".to_string()),
            "myconv".to_string(),
            r#"CREATE CONVERSION "public"."myconv" FOR 'UTF8' TO 'LATIN1' FROM utf8_to_iso8859_1;"#
                .to_string(),
        );
        let path = ConversionHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/public/conversion/myconv.sql"));
    }

    #[test]
    fn test_conversion_output_path_global() {
        let obj = RawObject::new(
            ObjectType::Conversion,
            None,
            "myconv".to_string(),
            r#"CREATE CONVERSION "myconv" FOR 'UTF8' TO 'LATIN1' FROM utf8_to_iso8859_1;"#
                .to_string(),
        );
        let path = ConversionHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(path, PathBuf::from("/output/_global/conversion/myconv.sql"));
    }

    #[test]
    fn test_conversion_implicit_dependencies() {
        let deps = ConversionHandler::implicit_dependency_types();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_conversion_no_pattern_dependencies() {
        let content = r#"CREATE DEFAULT CONVERSION "public"."myconv" FOR 'UTF8' TO 'SJIS' FROM utf8_to_sjis;"#;
        let deps = ConversionHandler::extract_pattern_dependencies(content);
        assert!(deps.is_empty());
    }
}
