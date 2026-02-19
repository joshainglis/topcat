//! Header generation for topcat-compatible SQL files.
//!
//! This module provides utilities for generating topcat metadata headers
//! that include name, layer, and dependency information.

use std::collections::HashSet;

use crate::commands::import::object_types::Layer;

/// Builder for creating topcat-compatible headers.
///
/// Provides a fluent API for constructing headers with various options.
///
/// # Example
///
/// ```ignore
/// let header = HeaderBuilder::new("public", "users")
///     .layer(Layer::Normal)
///     .requires(vec!["public.roles".to_string()])
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct HeaderBuilder {
    /// Schema name (None for global objects).
    schema: Option<String>,

    /// Object name.
    name: String,

    /// Optional layer assignment.
    layer: Option<Layer>,

    /// Dependencies to include in requires.
    requires: HashSet<String>,

    /// Whether to generate layer line.
    generate_layer: bool,
}

impl HeaderBuilder {
    /// Create a new header builder for a schema-qualified object.
    pub fn new(schema: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema: Some(schema.into()),
            name: name.into(),
            layer: None,
            requires: HashSet::new(),
            generate_layer: true,
        }
    }

    /// Create a new header builder for a global (non-schema-qualified) object.
    pub fn global(name: impl Into<String>) -> Self {
        Self {
            schema: None,
            name: name.into(),
            layer: None,
            requires: HashSet::new(),
            generate_layer: true,
        }
    }

    /// Set the layer for this object.
    pub fn layer(mut self, layer: Layer) -> Self {
        self.layer = Some(layer);
        self
    }

    /// Set the layer from an Option.
    pub fn maybe_layer(mut self, layer: Option<Layer>) -> Self {
        self.layer = layer;
        self
    }

    /// Add multiple dependencies.
    pub fn requires(mut self, deps: impl IntoIterator<Item = String>) -> Self {
        self.requires.extend(deps);
        self
    }

    /// Enable or disable layer generation.
    pub fn with_layer_generation(mut self, generate: bool) -> Self {
        self.generate_layer = generate;
        self
    }

    /// Build the header string.
    pub fn build(self) -> String {
        let name_line = match &self.schema {
            Some(schema) => format!("-- name: {}.{}\n", schema, self.name),
            None => format!("-- name: {}\n", self.name),
        };

        let mut header = name_line;

        if self.generate_layer
            && let Some(layer) = self.layer
        {
            header.push_str(&format!("-- layer: {}\n", layer.as_str()));
        }

        if !self.requires.is_empty() {
            let mut sorted_deps: Vec<_> = self.requires.into_iter().collect();
            sorted_deps.sort();
            header.push_str(&format!("-- requires: {}\n", sorted_deps.join(", ")));
        }

        header.push('\n');
        header
    }
}

/// Build a topcat-compatible header with name, layer, and dependencies.
///
/// This is a convenience function that wraps [`HeaderBuilder`].
///
/// # Arguments
///
/// - `schema`: The schema name for the object
/// - `name`: The object name
/// - `layer`: Optional layer assignment
/// - `requires`: Optional set of dependencies
/// - `generate_layers`: Whether to include the layer line
///
/// # Example
///
/// ```ignore
/// let mut deps = HashSet::new();
/// deps.insert("public.users".to_string());
///
/// let header = build_header("public", "orders", Some(Layer::Normal), Some(&deps), true);
/// assert!(header.contains("-- name: public.orders"));
/// assert!(header.contains("-- requires: public.users"));
/// ```
pub fn build_header(
    schema: &str,
    name: &str,
    layer: Option<Layer>,
    requires: Option<&HashSet<String>>,
    generate_layers: bool,
) -> String {
    let mut builder = HeaderBuilder::new(schema, name)
        .with_layer_generation(generate_layers)
        .maybe_layer(layer);

    if let Some(deps) = requires {
        builder = builder.requires(deps.iter().cloned());
    }

    builder.build()
}

/// Build a topcat-compatible header for global objects (no schema prefix).
///
/// This is a convenience function that wraps [`HeaderBuilder`].
///
/// # Arguments
///
/// - `name`: The object name
/// - `layer`: Optional layer assignment
/// - `requires`: Optional set of dependencies
/// - `generate_layers`: Whether to include the layer line
///
/// # Example
///
/// ```ignore
/// let header = build_global_header("plpgsql", Some(Layer::Prepend), None, true);
/// assert!(header.contains("-- name: plpgsql"));
/// assert!(header.contains("-- layer: prepend"));
/// ```
pub fn build_global_header(
    name: &str,
    layer: Option<Layer>,
    requires: Option<&HashSet<String>>,
    generate_layers: bool,
) -> String {
    let mut builder = HeaderBuilder::global(name)
        .with_layer_generation(generate_layers)
        .maybe_layer(layer);

    if let Some(deps) = requires {
        builder = builder.requires(deps.iter().cloned());
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_builder_basic() {
        let header = HeaderBuilder::new("public", "users").build();
        assert_eq!(header, "-- name: public.users\n\n");
    }

    #[test]
    fn test_header_builder_with_layer() {
        let header = HeaderBuilder::new("public", "users")
            .layer(Layer::Normal)
            .build();
        assert!(header.contains("-- name: public.users"));
        assert!(header.contains("-- layer: normal"));
    }

    #[test]
    fn test_header_builder_with_requires() {
        let header = HeaderBuilder::new("public", "orders")
            .requires(vec!["public.users".to_string(), "auth.roles".to_string()])
            .build();

        assert!(header.contains("-- name: public.orders"));
        assert!(header.contains("-- requires:"));
        assert!(header.contains("auth.roles"));
        assert!(header.contains("public.users"));
    }

    #[test]
    fn test_header_builder_global() {
        let header = HeaderBuilder::global("plpgsql")
            .layer(Layer::Prepend)
            .build();

        assert!(header.contains("-- name: plpgsql"));
        assert!(header.contains("-- layer: prepend"));
        assert!(!header.contains(".plpgsql")); // No schema prefix
    }

    #[test]
    fn test_header_builder_no_layer_generation() {
        let header = HeaderBuilder::new("public", "users")
            .layer(Layer::Normal)
            .with_layer_generation(false)
            .build();

        assert!(header.contains("-- name: public.users"));
        assert!(!header.contains("-- layer:"));
    }

    #[test]
    fn test_header_builder_sorted_deps() {
        let header = HeaderBuilder::new("public", "orders")
            .requires(vec![
                "z_table".to_string(),
                "a_table".to_string(),
                "m_table".to_string(),
            ])
            .build();

        // Dependencies should be sorted
        let requires_line = header
            .lines()
            .find(|l| l.starts_with("-- requires:"))
            .unwrap();
        assert!(requires_line.contains("a_table, m_table, z_table"));
    }

    #[test]
    fn test_build_header_function() {
        let mut deps = HashSet::new();
        deps.insert("public.users".to_string());
        deps.insert("auth.roles".to_string());

        let header = build_header("public", "orders", Some(Layer::Normal), Some(&deps), true);

        assert!(header.contains("-- name: public.orders"));
        assert!(header.contains("-- layer: normal"));
        assert!(header.contains("-- requires:"));
        assert!(header.contains("auth.roles"));
        assert!(header.contains("public.users"));
    }

    #[test]
    fn test_build_header_no_deps() {
        let header = build_header("public", "users", Some(Layer::Normal), None, true);

        assert!(header.contains("-- name: public.users"));
        assert!(header.contains("-- layer: normal"));
        assert!(!header.contains("-- requires:"));
    }

    #[test]
    fn test_build_header_no_layer() {
        let header = build_header("public", "users", Some(Layer::Normal), None, false);

        assert!(header.contains("-- name: public.users"));
        assert!(!header.contains("-- layer:"));
    }

    #[test]
    fn test_build_global_header_function() {
        let mut deps = HashSet::new();
        deps.insert("pg_catalog".to_string());

        let header = build_global_header("plpgsql", Some(Layer::Prepend), Some(&deps), true);

        assert!(header.contains("-- name: plpgsql"));
        assert!(header.contains("-- layer: prepend"));
        assert!(header.contains("-- requires: pg_catalog"));
    }
}
