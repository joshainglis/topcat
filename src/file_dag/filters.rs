//! Node filtering and graph traversal operations.

use std::collections::{HashMap, HashSet};

use petgraph::graph::DiGraph;

use crate::exceptions::TopCatError;

use super::core::TCGraph;

impl TCGraph {
    /// Get all transitive dependencies of a node (nodes that this node depends on).
    pub fn get_transitive_dependencies(&self, node_name: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut to_visit = vec![node_name.to_string()];

        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }

            if let Some(node) = self.name_map.get(&current) {
                visited.insert(current.clone());

                for dep in &node.deps {
                    if !visited.contains(dep) {
                        to_visit.push(dep.clone());
                    }
                }
            }
        }

        // Remove the starting node itself
        visited.remove(node_name);
        visited
    }

    /// Get all transitive dependents of a node (nodes that depend on this node).
    pub fn get_transitive_dependents(&self, node_name: &str) -> HashSet<String> {
        let dependents_map = self.build_dependents_map();
        let mut visited = HashSet::new();
        let mut to_visit = vec![node_name.to_string()];

        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }

            visited.insert(current.clone());

            if let Some(dependents) = dependents_map.get(&current) {
                for dependent in dependents {
                    if !visited.contains(dependent) {
                        to_visit.push(dependent.clone());
                    }
                }
            }
        }

        // Remove the starting node itself
        visited.remove(node_name);
        visited
    }

    /// Get direct neighbors of a node (both dependencies and dependents).
    ///
    /// Returns (dependencies, dependents).
    pub fn get_direct_neighbors(&self, node_name: &str) -> (HashSet<String>, HashSet<String>) {
        let dependencies = if let Some(node) = self.name_map.get(node_name) {
            node.deps.iter().cloned().collect()
        } else {
            HashSet::new()
        };

        let dependents_map = self.build_dependents_map();
        let dependents = dependents_map
            .get(node_name)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        (dependencies, dependents)
    }

    /// Create a filtered subgraph containing only the specified nodes.
    pub fn filter_nodes(&self, nodes_to_keep: &HashSet<String>) -> Result<TCGraph, TopCatError> {
        // Create a new graph with the same configuration
        let mut new_graph = TCGraph {
            comment_str: self.comment_str.clone(),
            file_dirs: self.file_dirs.clone(),
            exclude_globs: self.exclude_globs.clone(),
            include_globs: self.include_globs.clone(),
            include_extensions: self.include_extensions.clone(),
            exclude_extensions: self.exclude_extensions.clone(),
            include_node_prefixes: self.include_node_prefixes.clone(),
            exclude_node_prefixes: self.exclude_node_prefixes.clone(),
            layer_graphs: HashMap::new(),
            layer_index_maps: HashMap::new(),
            layers: self.layers.clone(),
            fallback_layer: self.fallback_layer.clone(),
            path_map: HashMap::new(),
            name_map: HashMap::new(),
            include_hidden: self.include_hidden,
            graph_is_built: false,
            subdir_filter: self.subdir_filter.clone(),
            sql_discovery: self.sql_discovery.clone(),
        };

        // Add filtered nodes to name_map and path_map
        for node_name in nodes_to_keep {
            if let Some(node) = self.name_map.get(node_name) {
                new_graph.name_map.insert(node_name.clone(), node.clone());
                new_graph.path_map.insert(node.path.clone(), node.clone());
            }
        }

        // Initialize layer graphs for filtered nodes
        for layer in &new_graph.layers {
            new_graph.layer_graphs.insert(layer.clone(), DiGraph::new());
            new_graph
                .layer_index_maps
                .insert(layer.clone(), HashMap::new());
        }

        // Add nodes to appropriate layer graphs
        for (node_name, node) in &new_graph.name_map {
            let layer_graph = new_graph
                .layer_graphs
                .get_mut(&node.layer)
                .expect("Layer graph should exist for node layer");
            let layer_map = new_graph
                .layer_index_maps
                .get_mut(&node.layer)
                .expect("Layer index map should exist for node layer");
            let idx = layer_graph.add_node(node.clone());
            layer_map.insert(node_name.clone(), idx);
        }

        // Add edges (only if both endpoints are in the filtered set)
        for (node_name, node) in &new_graph.name_map {
            let source_idx = new_graph
                .layer_index_maps
                .get(&node.layer)
                .and_then(|m| m.get(node_name))
                .copied();

            if let Some(source_idx) = source_idx {
                let layer_graph = new_graph
                    .layer_graphs
                    .get_mut(&node.layer)
                    .expect("Layer graph should exist for node layer");
                for dep in &node.deps {
                    // Only add edge if dependency is in the same layer and in the filtered set
                    if nodes_to_keep.contains(dep)
                        && let Some(dep_node) = new_graph.name_map.get(dep)
                        && dep_node.layer == node.layer
                        && let Some(&target_idx) = new_graph
                            .layer_index_maps
                            .get(&node.layer)
                            .and_then(|m| m.get(dep))
                    {
                        layer_graph.add_edge(source_idx, target_idx, ());
                    }
                }
            }
        }

        new_graph.graph_is_built = true;
        Ok(new_graph)
    }
}
