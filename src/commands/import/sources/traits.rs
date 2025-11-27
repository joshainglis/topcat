//! Traits for import sources.
//!
//! This module defines the [`ImportSource`] trait that all import sources must implement.
//! Sources extract database objects and security statements from their input format.

use super::RawObject;
use crate::commands::import::object_types::ObjectType;
use topcat::exceptions::TopCatError;

/// Trait for import sources that extract database objects.
///
/// Implement this trait to add support for new import source formats
/// (e.g., pg_dump files, database introspection, SQLAlchemy models).
///
/// # Example
///
/// ```ignore
/// struct MyCustomSource {
///     // source-specific state
/// }
///
/// impl ImportSource for MyCustomSource {
///     fn extract_objects(&mut self) -> Result<Vec<RawObject>> {
///         // Parse your source format and produce RawObjects
///         Ok(vec![])
///     }
///
///     fn collect_security(&mut self) -> Result<Vec<SecurityStatement>> {
///         // Extract security-related statements
///         Ok(vec![])
///     }
///
///     fn source_name(&self) -> &'static str {
///         "my-custom-source"
///     }
/// }
/// ```
pub trait ImportSource {
    /// Extract all database objects from the source.
    ///
    /// This method should parse the source format and return a vector of
    /// [`RawObject`] instances representing each database object found.
    ///
    /// The returned objects should have their `obj_type`, `schema`, `name`,
    /// `identity`, and `content` fields populated. The `source_metadata` field
    /// can contain any source-specific information that handlers might need.
    fn extract_objects(&mut self) -> Result<Vec<RawObject>, TopCatError>;

    /// Collect security statements from the source.
    ///
    /// Security statements (GRANT, REVOKE, OWNER) are often scattered throughout
    /// the source and need to be attached to their target objects after extraction.
    ///
    /// This method is called after `extract_objects` to gather all security
    /// statements for post-processing.
    fn collect_security(&mut self) -> Result<Vec<SecurityStatement>, TopCatError>;

    /// Get the name of this import source.
    ///
    /// Used for logging and error messages.
    fn source_name(&self) -> &'static str;
}

/// A security-related statement (GRANT, REVOKE, or OWNER change).
#[derive(Debug, Clone)]
pub struct SecurityStatement {
    /// The kind of security statement.
    pub kind: SecurityKind,

    /// The schema of the target object (if applicable).
    pub target_schema: Option<String>,

    /// The name of the target object.
    pub target_name: String,

    /// The type of the target object (if known).
    pub target_type: Option<ObjectType>,

    /// The SQL content of the security statement.
    pub content: String,
}

/// The kind of security statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecurityKind {
    /// A GRANT statement.
    Grant,
    /// A REVOKE statement.
    Revoke,
    /// An ALTER ... OWNER TO statement.
    Owner,
}

impl SecurityStatement {
    /// Create a new GRANT statement.
    pub fn grant(
        target_schema: Option<String>,
        target_name: String,
        target_type: Option<ObjectType>,
        content: String,
    ) -> Self {
        Self {
            kind: SecurityKind::Grant,
            target_schema,
            target_name,
            target_type,
            content,
        }
    }

    /// Create a new REVOKE statement.
    pub fn revoke(
        target_schema: Option<String>,
        target_name: String,
        target_type: Option<ObjectType>,
        content: String,
    ) -> Self {
        Self {
            kind: SecurityKind::Revoke,
            target_schema,
            target_name,
            target_type,
            content,
        }
    }

    /// Create a new OWNER statement.
    pub fn owner(
        target_schema: Option<String>,
        target_name: String,
        target_type: Option<ObjectType>,
        content: String,
    ) -> Self {
        Self {
            kind: SecurityKind::Owner,
            target_schema,
            target_name,
            target_type,
            content,
        }
    }

    /// Get the fully qualified name of the target.
    pub fn qualified_target(&self) -> String {
        match &self.target_schema {
            Some(schema) => format!("{}.{}", schema, self.target_name),
            None => self.target_name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_statement_grant() {
        let stmt = SecurityStatement::grant(
            Some("public".to_string()),
            "users".to_string(),
            Some(ObjectType::Table),
            "GRANT SELECT ON public.users TO reader;".to_string(),
        );

        assert_eq!(stmt.kind, SecurityKind::Grant);
        assert_eq!(stmt.qualified_target(), "public.users");
    }

    #[test]
    fn test_security_statement_global() {
        let stmt = SecurityStatement::owner(
            None,
            "my_extension".to_string(),
            Some(ObjectType::Extension),
            "ALTER EXTENSION my_extension OWNER TO admin;".to_string(),
        );

        assert_eq!(stmt.kind, SecurityKind::Owner);
        assert_eq!(stmt.qualified_target(), "my_extension");
    }
}
