//! Composable traits for type-specific object handling.
//!
//! These traits define the interface that type-specific handlers implement.
//! By separating concerns into multiple traits, handlers can compose behavior
//! and share common implementations.
//!
//! # Traits Overview
//!
//! - [`PatternProvider`]: Regex patterns for content-based identification
//! - [`DependencyExtractor`]: Extract dependencies from object content
//! - [`Categorizer`]: Determine output organization
//! - [`Renderer`]: Render objects to SQL output
//! - [`Configurable`]: Default configuration for the type
//! - [`ObjectHandler`]: Combined trait for full handling

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;

/// Provides regex patterns for identifying objects in SQL content.
///
/// Implement this trait for object types that can be identified by matching
/// patterns in their SQL content.
///
/// # Example
///
/// ```ignore
/// impl PatternProvider for TriggerHandler {
///     fn content_patterns() -> Vec<&'static Lazy<Regex>> {
///         vec![&TRIGGER_PATTERN]
///     }
/// }
/// ```
pub trait PatternProvider {
    /// Returns regex patterns that identify this object type in content.
    ///
    /// These patterns are used to:
    /// - Validate that content matches expected structure
    /// - Extract structured information from content
    ///
    /// Returns an empty vector if no patterns are applicable.
    fn content_patterns() -> Vec<&'static LazyLock<Regex>> {
        vec![]
    }
}

/// Extracts dependencies from object content.
///
/// Implement this trait to define how dependencies are extracted for a
/// specific object type. This includes both pattern-based extraction
/// (structural dependencies) and implicit dependency types.
pub trait DependencyExtractor {
    /// Extract dependencies using pattern matching on content.
    ///
    /// Returns a set of (dependency_name, dependency_type) pairs representing
    /// structural dependencies that can be determined from the SQL content.
    ///
    /// # Example
    ///
    /// For triggers, this extracts the function name from `EXECUTE FUNCTION func()`:
    /// ```ignore
    /// fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
    ///     // Extract "public.my_func" -> ObjectType::Function
    /// }
    /// ```
    fn extract_pattern_dependencies(_content: &str) -> Vec<(String, ObjectType)> {
        vec![]
    }

    /// Object types this type commonly depends on.
    ///
    /// Returns a list of object types that this type typically depends on.
    /// This is used to filter SQL analysis results to relevant dependency types.
    ///
    /// # Example
    ///
    /// Views typically depend on tables, other views, and functions:
    /// ```ignore
    /// fn implicit_dependency_types() -> Vec<ObjectType> {
    ///     vec![ObjectType::Table, ObjectType::View, ObjectType::Function]
    /// }
    /// ```
    fn implicit_dependency_types() -> Vec<ObjectType> {
        vec![]
    }
}

/// Categorizes objects for output organization.
///
/// Implement this trait to define how objects of this type should be
/// organized in the output directory structure.
pub trait Categorizer {
    /// Primary category for output directory structure.
    ///
    /// This determines the top-level directory within a schema where
    /// objects of this type are placed.
    fn category() -> ObjectCategory;

    /// Optional subcategory based on content analysis.
    ///
    /// Some object types have subcategories (e.g., types can be enum,
    /// composite, domain, range). This method analyzes content to
    /// determine the subcategory.
    ///
    /// Returns `None` if the type has no subcategories.
    fn subcategory(_content: &str) -> Option<String> {
        None
    }

    /// Compute the output path for this object.
    ///
    /// Returns the full path where this object should be written,
    /// relative to the output base directory.
    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf;
}

/// Renders objects to final SQL output.
///
/// Implement this trait to define how objects of this type should be
/// rendered to SQL, including any related objects that should be
/// included in the same file.
pub trait Renderer {
    /// Render the object with its related objects to SQL.
    ///
    /// Returns the complete SQL content for the output file, including
    /// the primary object and any attached related objects.
    ///
    /// # Arguments
    ///
    /// - `obj`: The primary object being rendered
    /// - `related`: Related objects (indexes, constraints, etc.) to include
    /// - `config`: Output configuration settings
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String;
}

/// Default configuration for an object type.
///
/// Implement this trait to define the default behavior for handling
/// objects of this type during import.
pub trait Configurable {
    /// Default ObjectTypeConfig for this type.
    ///
    /// Returns configuration specifying whether to skip, attach to parent, etc.
    fn default_config() -> ObjectTypeConfig;

    /// Dependency layer for this type.
    ///
    /// Returns which layer (Prepend, Normal, Append) objects of this type
    /// belong to for dependency ordering.
    fn layer() -> Layer;

    /// Whether this is a primary (standalone) object type.
    ///
    /// Primary objects get their own files. Non-primary objects (attachments)
    /// are included in their parent's file.
    fn is_primary() -> bool {
        true
    }
}

/// Combined trait for full object handling.
///
/// This trait combines all the composable traits and adds the type identifier.
/// Handlers that implement all the individual traits can derive this trait.
///
/// # Object-Safe Considerations
///
/// This trait is not object-safe due to the associated methods. Use the
/// [`HandlerRegistry`] to work with handlers dynamically.
pub trait ObjectHandler:
    PatternProvider + DependencyExtractor + Categorizer + Renderer + Configurable + Send + Sync
{
    /// The PostgreSQL object type this handler handles.
    fn object_type() -> ObjectType
    where
        Self: Sized;
}

/// Container for objects that attach to a primary object.
///
/// When rendering a primary object (like a table), related objects
/// (indexes, constraints, triggers, etc.) are included in the same file.
/// This struct collects all those related objects.
#[derive(Debug, Default, Clone)]
pub struct RelatedObjects {
    /// Index objects attached to this object.
    pub indexes: Vec<RawObject>,

    /// Constraint objects (CHECK, UNIQUE, PRIMARY KEY).
    pub constraints: Vec<RawObject>,

    /// Foreign key constraint objects.
    pub fk_constraints: Vec<RawObject>,

    /// Trigger objects.
    pub triggers: Vec<RawObject>,

    /// Row-level security policy objects.
    pub policies: Vec<RawObject>,

    /// Row security enable/disable statements.
    pub row_security: Vec<RawObject>,

    /// Default value objects for columns.
    pub defaults: Vec<RawObject>,

    /// Statistics objects.
    pub statistics: Vec<RawObject>,

    /// Rule objects.
    pub rules: Vec<RawObject>,

    /// Comment objects.
    pub comments: Vec<RawObject>,

    /// Sequence objects owned by this object.
    pub sequences: Vec<RawObject>,

    /// ACL (GRANT/REVOKE) statements as raw SQL.
    pub acl: Vec<String>,

    /// Owner statement if applicable.
    pub owner: Option<String>,
}

impl RelatedObjects {
    /// Create a new empty RelatedObjects container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are any related objects.
    pub fn is_empty(&self) -> bool {
        self.indexes.is_empty()
            && self.constraints.is_empty()
            && self.fk_constraints.is_empty()
            && self.triggers.is_empty()
            && self.policies.is_empty()
            && self.row_security.is_empty()
            && self.defaults.is_empty()
            && self.statistics.is_empty()
            && self.rules.is_empty()
            && self.comments.is_empty()
            && self.sequences.is_empty()
            && self.acl.is_empty()
            && self.owner.is_none()
    }

    /// Render all related objects in dependency order.
    ///
    /// Returns the concatenated SQL for all related objects, with appropriate
    /// spacing and ordering to ensure correct dependency resolution.
    pub fn render_all(&self, _config: &OutputConfig) -> String {
        let mut parts = Vec::new();

        // Order matters for dependencies:
        // 1. Sequences (may be referenced by defaults)
        for obj in &self.sequences {
            parts.push(obj.content.clone());
        }

        // 2. Indexes
        for obj in &self.indexes {
            parts.push(obj.content.clone());
        }

        // 3. Constraints (non-FK)
        for obj in &self.constraints {
            parts.push(obj.content.clone());
        }

        // 4. FK constraints (after tables/indexes exist)
        for obj in &self.fk_constraints {
            parts.push(obj.content.clone());
        }

        // 5. Defaults
        for obj in &self.defaults {
            parts.push(obj.content.clone());
        }

        // 6. Statistics
        for obj in &self.statistics {
            parts.push(obj.content.clone());
        }

        // 7. Rules
        for obj in &self.rules {
            parts.push(obj.content.clone());
        }

        // 8. Row security
        for obj in &self.row_security {
            parts.push(obj.content.clone());
        }

        // 9. Policies
        for obj in &self.policies {
            parts.push(obj.content.clone());
        }

        // 10. Triggers (after everything else)
        for obj in &self.triggers {
            parts.push(obj.content.clone());
        }

        // 11. Comments
        for obj in &self.comments {
            parts.push(obj.content.clone());
        }

        // 12. Owner
        if let Some(owner_sql) = &self.owner {
            parts.push(owner_sql.clone());
        }

        // 13. ACL (last, after object is fully created)
        for acl_sql in &self.acl {
            parts.push(acl_sql.clone());
        }

        parts
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Get total count of related objects.
    pub fn count(&self) -> usize {
        self.indexes.len()
            + self.constraints.len()
            + self.fk_constraints.len()
            + self.triggers.len()
            + self.policies.len()
            + self.row_security.len()
            + self.defaults.len()
            + self.statistics.len()
            + self.rules.len()
            + self.comments.len()
            + self.sequences.len()
            + self.acl.len()
            + if self.owner.is_some() { 1 } else { 0 }
    }
}

/// Configuration for output generation.
///
/// Controls various aspects of how output files are generated.
#[derive(Debug, Clone)]
pub struct OutputConfig {
    /// Whether to include ACL (GRANT/REVOKE) statements.
    pub include_acl: bool,

    /// Whether to include OWNER statements.
    pub include_owner: bool,

    /// Whether to generate layer headers.
    pub generate_layers: bool,

    /// Whether to generate dependency (requires:) headers.
    pub generate_deps: bool,

    /// Whether this is a dry-run (no files written).
    pub dry_run: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            include_acl: true,
            include_owner: true,
            generate_layers: true,
            generate_deps: true,
            dry_run: false,
        }
    }
}

impl OutputConfig {
    /// Create a new OutputConfig with all options enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config for dry-run mode.
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Self::default()
        }
    }
}

/// Helper function to extract dependencies from content using a pattern.
///
/// This is a utility for implementing `extract_pattern_dependencies`.
///
/// # Arguments
///
/// - `content`: SQL content to search
/// - `pattern`: Regex pattern with named capture groups
/// - `captures`: List of (capture_name, object_type) pairs to extract
///
/// # Returns
///
/// Vector of (qualified_name, object_type) pairs for dependencies found.
pub fn extract_deps_with_pattern(
    content: &str,
    pattern: &Regex,
    captures: &[(&str, Option<&str>, ObjectType)],
) -> Vec<(String, ObjectType)> {
    let mut deps = Vec::new();

    if let Some(caps) = pattern.captures(content) {
        for (name_capture, schema_capture, obj_type) in captures {
            if let Some(name_match) = caps.name(name_capture) {
                let name = name_match.as_str().to_string();
                let qualified = if let Some(schema_cap) = schema_capture {
                    if let Some(schema_match) = caps.name(schema_cap) {
                        format!("{}.{}", schema_match.as_str(), name)
                    } else {
                        name
                    }
                } else {
                    name
                };
                deps.push((qualified, *obj_type));
            }
        }
    }

    deps
}

// ============================================================================
// Helper functions that use traits as bounds
// ============================================================================

/// Get content patterns from a handler type.
///
/// This helper provides a generic interface to PatternProvider.
pub fn get_patterns<T: PatternProvider>() -> Vec<&'static LazyLock<Regex>> {
    T::content_patterns()
}

/// Extract dependencies from content using a handler type.
///
/// This helper provides a generic interface to DependencyExtractor.
pub fn extract_dependencies<T: DependencyExtractor>(content: &str) -> Vec<(String, ObjectType)> {
    T::extract_pattern_dependencies(content)
}

/// Get implicit dependency types from a handler type.
///
/// This helper provides a generic interface to DependencyExtractor.
pub fn get_implicit_deps<T: DependencyExtractor>() -> Vec<ObjectType> {
    T::implicit_dependency_types()
}

/// Get category and output path from a handler type.
///
/// This helper provides a generic interface to Categorizer.
pub fn get_category_info<T: Categorizer>(obj: &RawObject, base_dir: &Path) -> (ObjectCategory, PathBuf) {
    (T::category(), T::output_path(obj, base_dir))
}

/// Render an object using a handler type.
///
/// This helper provides a generic interface to Renderer.
pub fn render_object<T: Renderer>(
    obj: &RawObject,
    related: &RelatedObjects,
    config: &OutputConfig,
) -> String {
    T::render(obj, related, config)
}

/// Get configuration from a handler type.
///
/// This helper provides a generic interface to Configurable.
pub fn get_handler_config<T: Configurable>() -> (ObjectTypeConfig, Layer, bool) {
    (T::default_config(), T::layer(), T::is_primary())
}

/// Full object handler interface combining all traits.
///
/// This helper uses the ObjectHandler supertrait.
pub fn process_object<T: ObjectHandler>(
    obj: &RawObject,
    related: &RelatedObjects,
    config: &OutputConfig,
    base_dir: &Path,
) -> (PathBuf, String, Layer) {
    let path = T::output_path(obj, base_dir);
    let output = T::render(obj, related, config);
    let layer = T::layer();
    (path, output, layer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::handlers::attachments::TriggerHandler;
    use std::path::Path;

    #[test]
    fn test_related_objects_is_empty() {
        let related = RelatedObjects::new();
        assert!(related.is_empty());
    }

    #[test]
    fn test_helper_get_patterns() {
        let patterns = get_patterns::<TriggerHandler>();
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_helper_extract_dependencies() {
        let content = r#"CREATE TRIGGER my_trigger AFTER INSERT ON "public"."users" FOR EACH ROW EXECUTE FUNCTION "public"."notify"()"#;
        let deps = extract_dependencies::<TriggerHandler>(content);
        assert!(!deps.is_empty());
    }

    #[test]
    fn test_helper_get_implicit_deps() {
        let deps = get_implicit_deps::<TriggerHandler>();
        assert!(deps.contains(&ObjectType::Function));
    }

    #[test]
    fn test_helper_get_category_info() {
        let obj = RawObject::new(
            ObjectType::Trigger,
            Some("public".to_string()),
            "my_trigger".to_string(),
            "CREATE TRIGGER...".to_string(),
        );
        let (category, path) = get_category_info::<TriggerHandler>(&obj, Path::new("/output"));
        assert_eq!(category, ObjectCategory::TableAttachment);
        assert!(path.to_string_lossy().contains("trigger"));
    }

    #[test]
    fn test_helper_render_object() {
        let obj = RawObject::new(
            ObjectType::Trigger,
            Some("public".to_string()),
            "my_trigger".to_string(),
            "CREATE TRIGGER my_trigger...".to_string(),
        );
        let related = RelatedObjects::new();
        let config = OutputConfig::new();
        let output = render_object::<TriggerHandler>(&obj, &related, &config);
        assert!(output.contains("CREATE TRIGGER"));
    }

    #[test]
    fn test_helper_get_handler_config() {
        let (config, layer, is_primary) = get_handler_config::<TriggerHandler>();
        assert!(!config.skip);
        assert_eq!(layer, Layer::Append);
        assert!(!is_primary);
    }

    #[test]
    fn test_helper_process_object() {
        let obj = RawObject::new(
            ObjectType::Trigger,
            Some("public".to_string()),
            "my_trigger".to_string(),
            "CREATE TRIGGER my_trigger...".to_string(),
        );
        let related = RelatedObjects::new();
        let config = OutputConfig::new();
        let (path, output, layer) = process_object::<TriggerHandler>(&obj, &related, &config, Path::new("/output"));
        assert!(path.to_string_lossy().contains("trigger"));
        assert!(output.contains("CREATE TRIGGER"));
        assert_eq!(layer, Layer::Append);
    }

    #[test]
    fn test_related_objects_count() {
        let mut related = RelatedObjects::new();
        related.indexes.push(RawObject::default());
        related.constraints.push(RawObject::default());
        related.acl.push("GRANT...".to_string());

        assert!(!related.is_empty());
        assert_eq!(related.count(), 3);
    }

    #[test]
    fn test_output_config_default() {
        let config = OutputConfig::default();
        assert!(config.include_acl);
        assert!(config.include_owner);
        assert!(config.generate_layers);
        assert!(config.generate_deps);
        assert!(!config.dry_run);
    }

    #[test]
    fn test_output_config_dry_run() {
        let config = OutputConfig::dry_run();
        assert!(config.dry_run);
        assert!(config.include_acl); // Other options still on
    }

    #[test]
    fn test_related_objects_render_order() {
        let mut related = RelatedObjects::new();

        // Add objects in reverse order
        related.acl.push("-- acl".to_string());
        related.triggers.push(RawObject::new(
            ObjectType::Trigger,
            None,
            "t".to_string(),
            "-- trigger".to_string(),
        ));
        related.indexes.push(RawObject::new(
            ObjectType::Index,
            None,
            "i".to_string(),
            "-- index".to_string(),
        ));

        let output = related.render_all(&OutputConfig::default());

        // Should be rendered in correct order: index, trigger, acl
        let index_pos = output.find("-- index").unwrap();
        let trigger_pos = output.find("-- trigger").unwrap();
        let acl_pos = output.find("-- acl").unwrap();

        assert!(index_pos < trigger_pos, "Index should come before trigger");
        assert!(trigger_pos < acl_pos, "Trigger should come before acl");
    }
}
