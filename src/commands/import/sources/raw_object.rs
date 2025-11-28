//! Intermediate representation for imported database objects.
//!
//! [`RawObject`] is the common intermediate representation produced by all
//! import sources. It contains the essential information about a database
//! object that handlers need to process it.

use std::collections::HashMap;

use crate::commands::import::object_types::ObjectType;

/// A dependency extracted by the source using source-specific patterns.
///
/// This allows sources to extract structural dependencies during parsing
/// (e.g., trigger → function relationships) without handlers needing to
/// know about source-specific patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedDep {
    /// The name of the dependency (qualified or unqualified).
    pub name: String,
    /// The type of the dependency.
    pub dep_type: ObjectType,
}

impl ExtractedDep {
    /// Create a new extracted dependency.
    pub fn new(name: impl Into<String>, dep_type: ObjectType) -> Self {
        Self {
            name: name.into(),
            dep_type,
        }
    }
}

/// Intermediate representation of a database object produced by import sources.
///
/// This struct represents a database object in a source-agnostic way. Import sources
/// produce `RawObject` instances, which are then processed by type-specific handlers
/// to generate output files.
///
/// # Fields
///
/// - Core fields (`obj_type`, `schema`, `name`, `content`) are required for all objects
/// - `identity` supports overloaded objects (e.g., functions with different signatures)
/// - `source_metadata` allows source-specific data to flow through to handlers
///
/// # Example
///
/// ```ignore
/// let obj = RawObject {
///     obj_type: ObjectType::Table,
///     schema: Some("public".to_string()),
///     name: "users".to_string(),
///     identity: "users".to_string(),
///     content: "CREATE TABLE public.users (...)".to_string(),
///     owner: Some("postgres".to_string()),
///     target_type: None,
///     source_metadata: HashMap::new(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct RawObject {
    /// The PostgreSQL object type.
    pub obj_type: ObjectType,

    /// Schema name (None for global objects like extensions, languages).
    pub schema: Option<String>,

    /// Object name.
    pub name: String,

    /// Full identity for overloaded objects.
    ///
    /// For functions, this includes the signature (e.g., "my_func(integer, text)").
    /// For non-overloaded objects, this is the same as `name`.
    pub identity: String,

    /// SQL content defining the object.
    pub content: String,

    /// Owner of the object (if specified in source).
    pub owner: Option<String>,

    /// Target type for secondary objects.
    ///
    /// For example, a DEFAULT value has target_type "COLUMN" to indicate
    /// it belongs to a column rather than the table itself.
    pub target_type: Option<String>,

    /// Source-specific metadata.
    ///
    /// This allows import sources to pass additional information to handlers
    /// without modifying the core struct. Keys and values are source-specific.
    ///
    /// Common uses:
    /// - `"pg_dump_oid"`: The OID from pg_dump metadata
    /// - `"line_number"`: Source file line number
    /// - `"pg_dump_type"`: Original pg_dump type string (before normalization)
    pub source_metadata: HashMap<String, String>,

    /// Dependencies extracted by the source using source-specific patterns.
    ///
    /// Sources populate this during parsing by analyzing the SQL content with
    /// their own patterns. This decouples handlers from source-specific pattern
    /// knowledge - handlers just read the pre-extracted dependencies.
    ///
    /// Examples:
    /// - Trigger → Function (extracted from EXECUTE FUNCTION clause)
    /// - Foreign Table → Server (extracted from SERVER clause)
    /// - Subscription → Publication (extracted from PUBLICATION clause)
    pub extracted_deps: Vec<ExtractedDep>,
}

impl RawObject {
    /// Create a new RawObject with required fields.
    pub fn new(
        obj_type: ObjectType,
        schema: Option<String>,
        name: String,
        content: String,
    ) -> Self {
        let identity = name.clone();
        Self {
            obj_type,
            schema,
            name,
            identity,
            content,
            owner: None,
            target_type: None,
            source_metadata: HashMap::new(),
            extracted_deps: Vec::new(),
        }
    }

    /// Get the fully qualified name (schema.name or just name for global objects).
    pub fn qualified_name(&self) -> String {
        match &self.schema {
            Some(s) => format!("{}.{}", s, self.name),
            None => self.name.clone(),
        }
    }

    /// Check if this is a global (non-schema-qualified) object.
    pub fn is_global(&self) -> bool {
        self.schema.is_none()
    }

    /// Check if this object type is a primary (standalone) object.
    pub fn is_primary(&self) -> bool {
        self.obj_type.is_primary()
    }

    /// Check if this object type is a table attachment.
    pub fn is_table_attachment(&self) -> bool {
        self.obj_type.is_table_attachment()
    }

    /// Get a metadata value by key.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.source_metadata.get(key).map(|s| s.as_str())
    }

    /// Set a metadata value.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.source_metadata.insert(key.into(), value.into());
    }

    /// Builder method to set identity.
    pub fn with_identity(mut self, identity: impl Into<String>) -> Self {
        self.identity = identity.into();
        self
    }

    /// Builder method to set owner.
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(owner.into());
        self
    }

    /// Builder method to set target type.
    pub fn with_target_type(mut self, target_type: impl Into<String>) -> Self {
        self.target_type = Some(target_type.into());
        self
    }

    /// Builder method to add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.set_metadata(key, value);
        self
    }

    /// Add an extracted dependency.
    pub fn add_extracted_dep(&mut self, name: impl Into<String>, dep_type: ObjectType) {
        self.extracted_deps.push(ExtractedDep::new(name, dep_type));
    }

    /// Builder method to add an extracted dependency.
    pub fn with_extracted_dep(mut self, name: impl Into<String>, dep_type: ObjectType) -> Self {
        self.add_extracted_dep(name, dep_type);
        self
    }

    /// Get extracted dependencies.
    pub fn extracted_deps(&self) -> &[ExtractedDep] {
        &self.extracted_deps
    }
}

impl Default for RawObject {
    fn default() -> Self {
        Self {
            obj_type: ObjectType::Unknown,
            schema: None,
            name: String::new(),
            identity: String::new(),
            content: String::new(),
            owner: None,
            target_type: None,
            source_metadata: HashMap::new(),
            extracted_deps: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_object_qualified_name() {
        let obj = RawObject::new(
            ObjectType::Table,
            Some("public".to_string()),
            "users".to_string(),
            "CREATE TABLE...".to_string(),
        );
        assert_eq!(obj.qualified_name(), "public.users");

        let global_obj = RawObject::new(
            ObjectType::Extension,
            None,
            "uuid-ossp".to_string(),
            "CREATE EXTENSION...".to_string(),
        );
        assert_eq!(global_obj.qualified_name(), "uuid-ossp");
    }

    #[test]
    fn test_raw_object_is_global() {
        let schema_obj = RawObject::new(
            ObjectType::Table,
            Some("public".to_string()),
            "users".to_string(),
            "".to_string(),
        );
        assert!(!schema_obj.is_global());

        let global_obj = RawObject::new(
            ObjectType::Extension,
            None,
            "plpgsql".to_string(),
            "".to_string(),
        );
        assert!(global_obj.is_global());
    }

    #[test]
    fn test_raw_object_builder_methods() {
        let obj = RawObject::new(
            ObjectType::Function,
            Some("public".to_string()),
            "my_func".to_string(),
            "CREATE FUNCTION...".to_string(),
        )
        .with_identity("my_func(integer, text)")
        .with_owner("app_owner")
        .with_metadata("line_number", "42");

        assert_eq!(obj.identity, "my_func(integer, text)");
        assert_eq!(obj.owner, Some("app_owner".to_string()));
        assert_eq!(obj.get_metadata("line_number"), Some("42"));
    }

    #[test]
    fn test_raw_object_metadata() {
        let mut obj = RawObject::default();
        obj.set_metadata("key1", "value1");
        obj.set_metadata("key2", "value2");

        assert_eq!(obj.get_metadata("key1"), Some("value1"));
        assert_eq!(obj.get_metadata("key2"), Some("value2"));
        assert_eq!(obj.get_metadata("nonexistent"), None);
    }
}
