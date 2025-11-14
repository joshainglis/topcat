//! Schema-related operations for analyzing cross-schema dependencies.

use std::collections::{HashMap, HashSet};

use crate::file_node::FileNode;
use crate::schema_utils::SchemaFilter;

use super::core::TCGraph;

impl TCGraph {
    /// Get all schemas with their nodes grouped together.
    ///
    /// Returns a HashMap where keys are schema names and values are vectors of nodes in that schema.
    pub fn get_schemas(&self) -> HashMap<String, Vec<&FileNode>> {
        let mut schemas: HashMap<String, Vec<&FileNode>> = HashMap::new();

        for node in self.name_map.values() {
            let schema_name = node
                .schema
                .clone()
                .unwrap_or_else(|| "no_schema".to_string());
            schemas.entry(schema_name).or_default().push(node);
        }

        schemas
    }

    /// Get all unique schema names.
    pub fn get_schema_names(&self) -> Vec<String> {
        let mut schema_names: HashSet<String> = HashSet::new();

        for node in self.name_map.values() {
            if let Some(schema) = &node.schema {
                schema_names.insert(schema.clone());
            }
        }

        let mut names: Vec<String> = schema_names.into_iter().collect();
        names.sort();
        names
    }

    /// Get internal dependencies (within the same schema) for a given schema.
    pub fn get_internal_dependencies(&self, schema: &str) -> HashSet<(String, String)> {
        let mut internal_deps = HashSet::new();

        for node in self.name_map.values() {
            if node.schema.as_deref() == Some(schema) {
                for dep in &node.deps {
                    if let Some(dep_node) = self.name_map.get(dep) {
                        if dep_node.schema.as_deref() == Some(schema) {
                            internal_deps.insert((node.name.clone(), dep.clone()));
                        }
                    }
                }
            }
        }

        internal_deps
    }

    /// Get external dependencies (to other schemas) for a given schema.
    ///
    /// Returns tuples of (source_node, target_node, target_schema).
    pub fn get_external_dependencies(&self, schema: &str) -> Vec<(String, String, String)> {
        let mut external_deps = Vec::new();

        for node in self.name_map.values() {
            if node.schema.as_deref() == Some(schema) {
                for dep in &node.deps {
                    if let Some(dep_node) = self.name_map.get(dep) {
                        if let Some(dep_schema) = &dep_node.schema {
                            if dep_schema != schema {
                                external_deps.push((
                                    node.name.clone(),
                                    dep.clone(),
                                    dep_schema.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        external_deps
    }

    /// Get schemas that depend on the given schema.
    pub fn get_dependent_schemas(&self, schema: &str) -> HashSet<String> {
        let mut dependent_schemas = HashSet::new();

        for node in self.name_map.values() {
            // Check if this node depends on any node in the target schema
            for dep in &node.deps {
                if let Some(dep_node) = self.name_map.get(dep) {
                    if dep_node.schema.as_deref() == Some(schema) {
                        if let Some(node_schema) = &node.schema {
                            if node_schema != schema {
                                dependent_schemas.insert(node_schema.clone());
                            }
                        }
                    }
                }
            }
        }

        dependent_schemas
    }

    /// Get cross-schema dependency pairs (schema_a -> schema_b).
    pub fn get_cross_schema_dependencies(&self) -> Vec<(String, String)> {
        let mut cross_deps = HashSet::new();

        for node in self.name_map.values() {
            if let Some(source_schema) = &node.schema {
                for dep in &node.deps {
                    if let Some(dep_node) = self.name_map.get(dep) {
                        if let Some(target_schema) = &dep_node.schema {
                            if source_schema != target_schema {
                                cross_deps.insert((source_schema.clone(), target_schema.clone()));
                            }
                        }
                    }
                }
            }
        }

        let mut deps: Vec<_> = cross_deps.into_iter().collect();
        deps.sort();
        deps
    }

    /// Apply schema-based node prefix filtering using SchemaFilter.
    ///
    /// This modifies the include_node_prefixes to filter by the specified schemas.
    /// If schemas is empty, no filtering is applied.
    ///
    /// # Arguments
    ///
    /// * `schemas` - List of schema names to filter by
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use topcat::file_dag::TCGraph;
    /// # use topcat::config::Config;
    /// # let config: Config = unimplemented!();
    /// # let mut graph = TCGraph::new(&config);
    /// let schemas = vec!["auth".to_string(), "billing".to_string()];
    /// graph.apply_schema_filter(&schemas);
    /// ```
    pub fn apply_schema_filter(&mut self, schemas: &[String]) {
        let filter = SchemaFilter::from(schemas);
        if filter.is_empty() {
            return;
        }

        // Convert schemas to node prefixes using SchemaFilter
        let schema_prefixes: HashSet<String> = filter.to_node_prefixes().into_iter().collect();

        // Add to include_node_prefixes
        match &mut self.include_node_prefixes {
            Some(existing) => {
                // Intersect with existing prefixes if they exist
                *existing = existing.intersection(&schema_prefixes).cloned().collect();
            }
            None => {
                // Set new prefixes
                self.include_node_prefixes = Some(schema_prefixes);
            }
        }
    }
}
