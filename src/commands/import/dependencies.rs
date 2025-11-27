//! Dependency analysis for imported PostgreSQL objects.
//!
//! This module analyzes SQL content during import to automatically generate
//! `requires:` headers for topcat dependency tracking.

use std::collections::{HashMap, HashSet};

use topcat::sql_config::SqlDiscoveryConfig;
use topcat::sql_parser::SqlAnalyzer;

use super::object_types::{Layer, ObjectType};
use super::patterns::{
    EVENT_TRIGGER_PATTERN, FOREIGN_TABLE_PATTERN, SERVER_PATTERN, SUBSCRIPTION_PATTERN,
    TRANSFORM_PATTERN, USER_MAPPING_PATTERN,
};

/// Analyzes SQL content to extract dependencies for imported objects.
pub struct DependencyAnalyzer {
    sql_analyzer: SqlAnalyzer,
    /// Known objects that we've seen during this import
    known_objects: HashSet<String>,
    /// Known objects organized by type for implicit dependency analysis
    known_objects_by_type: HashMap<ObjectType, HashSet<String>>,
}

impl DependencyAnalyzer {
    /// Create a new dependency analyzer with the given configuration.
    pub fn new(config: SqlDiscoveryConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let sql_analyzer = SqlAnalyzer::new(config)?;
        Ok(Self {
            sql_analyzer,
            known_objects: HashSet::new(),
            known_objects_by_type: HashMap::new(),
        })
    }

    /// Register an object that we've seen during import.
    ///
    /// This helps filter out dependencies on objects that don't exist in the import.
    fn register_object(&mut self, schema: &str, name: &str) {
        let full_name = format!("{schema}.{name}");
        self.known_objects.insert(full_name.to_lowercase());
        // Also register the schema itself
        self.known_objects.insert(schema.to_lowercase());
    }

    /// Register an object with its type for enhanced dependency analysis.
    ///
    /// This enables using implicit dependency information when analyzing objects
    /// of types that have known dependency patterns.
    pub fn register_object_with_type(&mut self, schema: &str, name: &str, obj_type: ObjectType) {
        // Register in the basic set
        self.register_object(schema, name);

        // Register by type for implicit dependency analysis
        let full_name = format!("{schema}.{name}");
        self.known_objects_by_type
            .entry(obj_type)
            .or_default()
            .insert(full_name.to_lowercase());
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

    /// Analyze dependencies for a specific object type.
    ///
    /// Uses implicit dependency information to prioritize dependencies from
    /// types that the given object type typically depends on.
    pub fn analyze_dependencies_for_type(
        &self,
        content: &str,
        obj_type: ObjectType,
    ) -> HashSet<String> {
        let result = self.sql_analyzer.analyze(content);
        let implicit_types = implicit_dependencies_for_type(obj_type);

        // Build a set of objects from implicit dependency types
        let implicit_objects: HashSet<&String> = implicit_types
            .iter()
            .filter_map(|t| self.known_objects_by_type.get(t))
            .flatten()
            .collect();

        // Start with SQL-parsed dependencies
        let mut deps: HashSet<String> = result
            .dependencies
            .iter()
            .filter(|dep| {
                let dep_lower = dep.to_lowercase();
                // Include if it's in our known objects
                // (either from implicit types or general registration)
                self.known_objects.contains(&dep_lower) || implicit_objects.contains(&dep_lower)
            })
            .cloned()
            .collect();

        // Add pattern-extracted dependencies
        deps.extend(self.extract_pattern_dependencies(content, obj_type));

        deps
    }

    /// Extract dependencies from SQL content using specialized patterns.
    ///
    /// These patterns detect structural dependencies that the general SQL parser
    /// might miss, such as:
    /// - Foreign tables depending on servers
    /// - Servers depending on foreign data wrappers
    /// - Subscriptions depending on publications
    /// - Event triggers depending on functions
    fn extract_pattern_dependencies(&self, content: &str, obj_type: ObjectType) -> HashSet<String> {
        let mut deps = HashSet::new();

        match obj_type {
            ObjectType::ForeignTable => {
                // Foreign tables depend on their server
                if let Some(caps) = FOREIGN_TABLE_PATTERN.captures(content) {
                    if let Some(server) = caps.name("server") {
                        let server_name = server.as_str().to_lowercase();
                        if self.known_objects.contains(&server_name) {
                            deps.insert(server_name);
                        }
                    }
                }
            }
            ObjectType::Server => {
                // Servers depend on their foreign data wrapper
                if let Some(caps) = SERVER_PATTERN.captures(content) {
                    if let Some(fdw) = caps.name("fdw") {
                        let fdw_name = fdw.as_str().to_lowercase();
                        if self.known_objects.contains(&fdw_name) {
                            deps.insert(fdw_name);
                        }
                    }
                }
            }
            ObjectType::UserMapping => {
                // User mappings depend on their server
                if let Some(caps) = USER_MAPPING_PATTERN.captures(content) {
                    if let Some(server) = caps.name("server") {
                        let server_name = server.as_str().to_lowercase();
                        if self.known_objects.contains(&server_name) {
                            deps.insert(server_name);
                        }
                    }
                }
            }
            ObjectType::Subscription => {
                // Subscriptions depend on their publication
                if let Some(caps) = SUBSCRIPTION_PATTERN.captures(content) {
                    if let Some(pub_name) = caps.name("publication") {
                        let pub_lower = pub_name.as_str().to_lowercase();
                        if self.known_objects.contains(&pub_lower) {
                            deps.insert(pub_lower);
                        }
                    }
                }
            }
            ObjectType::EventTrigger => {
                // Event triggers depend on their function
                if let Some(caps) = EVENT_TRIGGER_PATTERN.captures(content) {
                    if let (Some(schema), Some(name)) =
                        (caps.name("fn_schema"), caps.name("fn_name"))
                    {
                        let full_name =
                            format!("{}.{}", schema.as_str(), name.as_str()).to_lowercase();
                        if self.known_objects.contains(&full_name) {
                            deps.insert(full_name);
                        }
                    } else if let Some(name) = caps.name("fn_name") {
                        let fn_name = name.as_str().to_lowercase();
                        if self.known_objects.contains(&fn_name) {
                            deps.insert(fn_name);
                        }
                    }
                }
            }
            ObjectType::Transform => {
                // Transforms depend on their type and language
                if let Some(caps) = TRANSFORM_PATTERN.captures(content) {
                    if let Some(lang) = caps.name("language") {
                        let lang_name = lang.as_str().to_lowercase();
                        if self.known_objects.contains(&lang_name) {
                            deps.insert(lang_name);
                        }
                    }
                    if let (Some(schema), Some(name)) =
                        (caps.name("type_schema"), caps.name("type_name"))
                    {
                        let full_name =
                            format!("{}.{}", schema.as_str(), name.as_str()).to_lowercase();
                        if self.known_objects.contains(&full_name) {
                            deps.insert(full_name);
                        }
                    }
                }
            }
            _ => {}
        }

        deps
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
