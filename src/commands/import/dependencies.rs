//! Dependency analysis for imported PostgreSQL objects.
//!
//! This module analyzes SQL content during import to automatically generate
//! `requires:` headers for topcat dependency tracking.

use std::collections::HashSet;

use topcat::sql_config::SqlDiscoveryConfig;
use topcat::sql_parser::SqlAnalyzer;

use super::object_types::{Layer, ObjectType};

/// Analyzes SQL content to extract dependencies for imported objects.
pub struct DependencyAnalyzer {
    sql_analyzer: SqlAnalyzer,
    /// Known objects that we've seen during this import
    known_objects: HashSet<String>,
}

impl DependencyAnalyzer {
    /// Create a new dependency analyzer with the given configuration.
    pub fn new(config: SqlDiscoveryConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let sql_analyzer = SqlAnalyzer::new(config)?;
        Ok(Self {
            sql_analyzer,
            known_objects: HashSet::new(),
        })
    }

    /// Register an object that we've seen during import.
    ///
    /// This helps filter out dependencies on objects that don't exist in the import.
    pub fn register_object(&mut self, schema: &str, name: &str) {
        let full_name = format!("{schema}.{name}");
        self.known_objects.insert(full_name.to_lowercase());
        // Also register the schema itself
        self.known_objects.insert(schema.to_lowercase());
    }

    /// Analyze SQL content and extract dependencies.
    ///
    /// Returns a set of dependency names that should be added to the `requires:` header.
    pub fn analyze_dependencies(&self, content: &str) -> HashSet<String> {
        let result = self.sql_analyzer.analyze(content);

        // Filter dependencies to only those that exist in our known objects
        result
            .dependencies
            .iter()
            .filter(|dep| {
                let dep_lower = dep.to_lowercase();
                // Include if it's a known object (schema or schema.name)
                self.known_objects.contains(&dep_lower)
            })
            .cloned()
            .collect()
    }

    /// Analyze dependencies and return formatted header lines.
    ///
    /// Returns the dependencies as a `requires:` header line, or None if no dependencies found.
    #[allow(dead_code)]
    pub fn generate_requires_header(&self, content: &str) -> Option<String> {
        let deps = self.analyze_dependencies(content);

        if deps.is_empty() {
            return None;
        }

        // Sort for deterministic output
        let mut sorted_deps: Vec<_> = deps.into_iter().collect();
        sorted_deps.sort();

        Some(format!("-- requires: {}", sorted_deps.join(", ")))
    }
}

/// Build a topcat-compatible header with name, layer, and dependencies.
pub fn build_header(
    schema: &str,
    name: &str,
    layer: Option<Layer>,
    requires: Option<&HashSet<String>>,
    generate_layers: bool,
) -> String {
    let mut header = format!("-- name: {schema}.{name}\n");

    if generate_layers {
        if let Some(l) = layer {
            header.push_str(&format!("-- layer: {}\n", l.as_str()));
        }
    }

    if let Some(deps) = requires {
        if !deps.is_empty() {
            let mut sorted_deps: Vec<_> = deps.iter().cloned().collect();
            sorted_deps.sort();
            header.push_str(&format!("-- requires: {}\n", sorted_deps.join(", ")));
        }
    }

    header.push('\n');
    header
}

/// Build a topcat-compatible header for global objects (no schema prefix).
pub fn build_global_header(
    name: &str,
    layer: Option<Layer>,
    requires: Option<&HashSet<String>>,
    generate_layers: bool,
) -> String {
    let mut header = format!("-- name: {name}\n");

    if generate_layers {
        if let Some(l) = layer {
            header.push_str(&format!("-- layer: {}\n", l.as_str()));
        }
    }

    if let Some(deps) = requires {
        if !deps.is_empty() {
            let mut sorted_deps: Vec<_> = deps.iter().cloned().collect();
            sorted_deps.sort();
            header.push_str(&format!("-- requires: {}\n", sorted_deps.join(", ")));
        }
    }

    header.push('\n');
    header
}

/// Determine implicit dependencies based on object type.
///
/// Some object types have inherent dependencies:
/// - Functions depend on their parameter and return types
/// - Tables depend on their column types
/// - Views depend on the tables they reference
/// - Triggers depend on the tables they're attached to
#[allow(dead_code)]
pub fn implicit_dependencies_for_type(obj_type: ObjectType) -> Vec<ObjectType> {
    match obj_type {
        ObjectType::Function | ObjectType::Procedure | ObjectType::Aggregate => {
            vec![ObjectType::Type, ObjectType::Domain, ObjectType::Extension]
        }
        ObjectType::Table | ObjectType::ForeignTable => {
            vec![ObjectType::Type, ObjectType::Domain, ObjectType::Schema]
        }
        ObjectType::View | ObjectType::MaterializedView => {
            vec![ObjectType::Table, ObjectType::View, ObjectType::Function]
        }
        ObjectType::Trigger => vec![ObjectType::Function, ObjectType::Table],
        ObjectType::Index => vec![ObjectType::Table, ObjectType::OperatorClass],
        ObjectType::Constraint | ObjectType::FkConstraint => vec![ObjectType::Table],
        ObjectType::Policy => vec![ObjectType::Table, ObjectType::Function],
        ObjectType::Cast => vec![ObjectType::Type, ObjectType::Function],
        ObjectType::Operator => {
            vec![
                ObjectType::Type,
                ObjectType::Function,
                ObjectType::OperatorFamily,
            ]
        }
        ObjectType::OperatorClass | ObjectType::OperatorFamily => {
            vec![ObjectType::Type, ObjectType::AccessMethod]
        }
        ObjectType::TextSearchConfiguration => vec![ObjectType::TextSearchParser],
        ObjectType::TextSearchDictionary => vec![ObjectType::TextSearchTemplate],
        ObjectType::Server => vec![ObjectType::ForeignDataWrapper],
        ObjectType::UserMapping => vec![ObjectType::Server],
        ObjectType::Subscription => vec![ObjectType::Publication],
        ObjectType::EventTrigger => vec![ObjectType::Function],
        ObjectType::Transform => vec![ObjectType::Type, ObjectType::Language],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_header_full() {
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
    fn test_build_global_header() {
        let mut deps = HashSet::new();
        deps.insert("pg_catalog".to_string());

        let header = build_global_header("plpgsql", Some(Layer::Prepend), Some(&deps), true);

        assert!(header.contains("-- name: plpgsql"));
        assert!(header.contains("-- layer: prepend"));
        assert!(header.contains("-- requires: pg_catalog"));
    }

    #[test]
    fn test_implicit_dependencies() {
        assert!(!implicit_dependencies_for_type(ObjectType::Function).is_empty());
        assert!(!implicit_dependencies_for_type(ObjectType::View).is_empty());
        assert!(implicit_dependencies_for_type(ObjectType::Schema).is_empty());
    }
}
