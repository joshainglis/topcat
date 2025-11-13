//! Schema filtering utilities for dependency graph operations.
//!
//! This module provides the `SchemaFilter` type for working with schema-based
//! filtering of nodes in dependency graphs. Schema filtering is commonly used
//! to focus analysis, cleanup, and export operations on specific schemas within
//! a multi-schema codebase.
//!
//! # Examples
//!
//! ```
//! use topcat::schema_utils::SchemaFilter;
//!
//! let filter = SchemaFilter::new(vec!["auth".to_string(), "billing".to_string()]);
//!
//! // Convert to node prefixes for graph filtering
//! let prefixes = filter.to_node_prefixes();
//! assert!(prefixes.contains(&"auth.".to_string()));
//! assert!(prefixes.contains(&"billing.".to_string()));
//!
//! // Check if a node matches
//! assert!(filter.matches_node("auth.users"));
//! assert!(filter.matches_node("auth"));
//! assert!(!filter.matches_node("public.data"));
//! ```

use crate::file_node::FileNode;

/// Schema-based filter for dependency graph nodes.
///
/// A `SchemaFilter` represents a set of schemas to include when filtering nodes.
/// It provides methods to:
/// - Convert schemas to node name prefixes for graph construction
/// - Check if individual nodes match the filter
/// - Filter collections of nodes
///
/// # Schema Naming Convention
///
/// Topcat assumes a naming convention where:
/// - Schema definitions are named exactly as the schema (e.g., "auth")
/// - Schema members are prefixed with the schema name (e.g., "auth.users", "auth.sessions")
///
/// This allows the filter to match both the schema itself and all its members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaFilter {
    schemas: Vec<String>,
}

impl SchemaFilter {
    /// Create a new schema filter.
    ///
    /// # Arguments
    ///
    /// * `schemas` - List of schema names to filter by
    ///
    /// # Examples
    ///
    /// ```
    /// use topcat::schema_utils::SchemaFilter;
    ///
    /// let filter = SchemaFilter::new(vec!["auth".to_string()]);
    /// ```
    pub fn new(schemas: Vec<String>) -> Self {
        Self { schemas }
    }

    /// Create an empty schema filter (matches nothing).
    ///
    /// This is useful as a default value or when no filtering is needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcat::schema_utils::SchemaFilter;
    ///
    /// let filter = SchemaFilter::empty();
    /// assert!(filter.is_empty());
    /// ```
    pub fn empty() -> Self {
        Self {
            schemas: Vec::new(),
        }
    }

    /// Check if this filter is empty (no schemas specified).
    ///
    /// An empty filter typically means "include all schemas".
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    /// Get the list of schemas in this filter.
    pub fn schemas(&self) -> &[String] {
        &self.schemas
    }

    /// Convert schema names to node name prefixes for graph filtering.
    ///
    /// For each schema, generates:
    /// - The exact schema name (for the schema definition itself)
    /// - The schema name with a dot suffix (for schema members)
    ///
    /// # Returns
    ///
    /// A vector of node name prefixes. If the filter is empty, returns an empty vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcat::schema_utils::SchemaFilter;
    ///
    /// let filter = SchemaFilter::new(vec!["auth".to_string()]);
    /// let prefixes = filter.to_node_prefixes();
    ///
    /// assert_eq!(prefixes, vec!["auth".to_string(), "auth.".to_string()]);
    /// ```
    pub fn to_node_prefixes(&self) -> Vec<String> {
        if self.is_empty() {
            return Vec::new();
        }

        let mut prefixes = Vec::with_capacity(self.schemas.len() * 2);
        for schema in &self.schemas {
            prefixes.push(schema.clone()); // Exact match (e.g., "auth")
            prefixes.push(format!("{schema}.")); // Prefix match (e.g., "auth.")
        }
        prefixes
    }

    /// Check if a node name matches this schema filter.
    ///
    /// A node matches if:
    /// - Its name exactly equals one of the schema names, OR
    /// - Its name starts with a schema name followed by a dot
    ///
    /// If the filter is empty, no nodes match (empty filter = no schemas selected).
    ///
    /// # Arguments
    ///
    /// * `node_name` - The name of the node to check
    ///
    /// # Examples
    ///
    /// ```
    /// use topcat::schema_utils::SchemaFilter;
    ///
    /// let filter = SchemaFilter::new(vec!["auth".to_string()]);
    ///
    /// assert!(filter.matches_node("auth"));
    /// assert!(filter.matches_node("auth.users"));
    /// assert!(filter.matches_node("auth.sessions"));
    /// assert!(!filter.matches_node("public"));
    /// assert!(!filter.matches_node("public.data"));
    /// ```
    pub fn matches_node(&self, node_name: &str) -> bool {
        if self.is_empty() {
            return false;
        }

        self.schemas
            .iter()
            .any(|schema| node_name == schema || node_name.starts_with(&format!("{schema}.")))
    }

    /// Filter a collection of nodes, keeping only those that match the schema filter.
    ///
    /// If the filter is empty, returns an empty vector.
    ///
    /// # Arguments
    ///
    /// * `nodes` - Slice of file nodes to filter
    ///
    /// # Returns
    ///
    /// A vector of references to nodes that match the filter
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use topcat::schema_utils::SchemaFilter;
    /// use topcat::file_node::FileNode;
    ///
    /// let filter = SchemaFilter::new(vec!["auth".to_string()]);
    /// let nodes: Vec<FileNode> = vec![]; // Your nodes here
    /// let filtered = filter.filter_nodes(&nodes);
    /// ```
    pub fn filter_nodes<'a>(&self, nodes: &'a [FileNode]) -> Vec<&'a FileNode> {
        if self.is_empty() {
            return Vec::new();
        }

        nodes
            .iter()
            .filter(|node| self.matches_node(&node.name))
            .collect()
    }

    /// Convert this filter to an Option, returning None if empty.
    ///
    /// This is useful when calling APIs that accept `Option<Vec<String>>` for schema filtering.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcat::schema_utils::SchemaFilter;
    ///
    /// let empty = SchemaFilter::empty();
    /// assert_eq!(empty.to_option(), None);
    ///
    /// let filter = SchemaFilter::new(vec!["auth".to_string()]);
    /// assert_eq!(filter.to_option(), Some(vec!["auth".to_string(), "auth.".to_string()]));
    /// ```
    pub fn to_option(&self) -> Option<Vec<String>> {
        if self.is_empty() {
            None
        } else {
            Some(self.to_node_prefixes())
        }
    }
}

impl Default for SchemaFilter {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<Vec<String>> for SchemaFilter {
    fn from(schemas: Vec<String>) -> Self {
        Self::new(schemas)
    }
}

impl From<&[String]> for SchemaFilter {
    fn from(schemas: &[String]) -> Self {
        Self::new(schemas.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_filter() {
        let filter = SchemaFilter::empty();
        assert!(filter.is_empty());
        assert_eq!(filter.schemas(), &[] as &[String]);
        assert_eq!(filter.to_node_prefixes(), Vec::<String>::new());
        assert_eq!(filter.to_option(), None);
    }

    #[test]
    fn test_single_schema() {
        let filter = SchemaFilter::new(vec!["auth".to_string()]);
        assert!(!filter.is_empty());
        assert_eq!(filter.schemas(), &["auth"]);
        assert_eq!(
            filter.to_node_prefixes(),
            vec!["auth".to_string(), "auth.".to_string()]
        );
        assert_eq!(
            filter.to_option(),
            Some(vec!["auth".to_string(), "auth.".to_string()])
        );
    }

    #[test]
    fn test_multiple_schemas() {
        let filter = SchemaFilter::new(vec!["auth".to_string(), "billing".to_string()]);
        assert!(!filter.is_empty());
        assert_eq!(filter.schemas(), &["auth", "billing"]);

        let prefixes = filter.to_node_prefixes();
        assert_eq!(prefixes.len(), 4);
        assert!(prefixes.contains(&"auth".to_string()));
        assert!(prefixes.contains(&"auth.".to_string()));
        assert!(prefixes.contains(&"billing".to_string()));
        assert!(prefixes.contains(&"billing.".to_string()));
    }

    #[test]
    fn test_matches_node_empty_filter() {
        let filter = SchemaFilter::empty();
        assert!(!filter.matches_node("auth"));
        assert!(!filter.matches_node("auth.users"));
        assert!(!filter.matches_node("public"));
    }

    #[test]
    fn test_matches_node_exact_match() {
        let filter = SchemaFilter::new(vec!["auth".to_string()]);
        assert!(filter.matches_node("auth"));
        assert!(!filter.matches_node("authentication")); // Not exact
        assert!(!filter.matches_node("authz")); // Not exact
    }

    #[test]
    fn test_matches_node_prefix_match() {
        let filter = SchemaFilter::new(vec!["auth".to_string()]);
        assert!(filter.matches_node("auth.users"));
        assert!(filter.matches_node("auth.sessions"));
        assert!(filter.matches_node("auth.permissions.roles")); // Nested
        assert!(!filter.matches_node("authusers")); // No dot separator
    }

    #[test]
    fn test_matches_node_multiple_schemas() {
        let filter = SchemaFilter::new(vec!["auth".to_string(), "billing".to_string()]);
        assert!(filter.matches_node("auth"));
        assert!(filter.matches_node("auth.users"));
        assert!(filter.matches_node("billing"));
        assert!(filter.matches_node("billing.invoices"));
        assert!(!filter.matches_node("public"));
        assert!(!filter.matches_node("public.data"));
    }

    #[test]
    fn test_from_vec() {
        let filter: SchemaFilter = vec!["auth".to_string()].into();
        assert_eq!(filter.schemas(), &["auth"]);
    }

    #[test]
    fn test_from_slice() {
        let schemas = vec!["auth".to_string(), "billing".to_string()];
        let filter: SchemaFilter = schemas.as_slice().into();
        assert_eq!(filter.schemas(), &["auth", "billing"]);
    }

    #[test]
    fn test_default() {
        let filter = SchemaFilter::default();
        assert!(filter.is_empty());
    }
}
