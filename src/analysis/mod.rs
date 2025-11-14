// Analysis module for dependency graph analysis
// This module will contain algorithms for finding orphans, unrequired files, dead branches, etc.

pub mod external_usage;
pub mod root_matcher;

use self::root_matcher::RootNodeMatcher;
use crate::file_dag::TCGraph;
use std::collections::{HashMap, HashSet};

/// Trait for analyzing dependency graphs
#[allow(dead_code)]
pub trait GraphAnalyzer {
    /// Find files with no dependencies and no dependents (orphans)
    fn find_orphans(&self) -> HashSet<String>;

    /// Find files that are not required by any other files
    fn find_unrequired(&self) -> HashSet<String>;

    /// Find files with dependencies but no dependents (leaf nodes)
    fn find_leaf_nodes(&self) -> HashSet<String>;

    /// Find files with dependents but no dependencies (root nodes)
    fn find_root_nodes(&self) -> HashSet<String>;

    /// Find complete dead branches (subtrees that can be trimmed together)
    /// Optionally accepts a RootNodeMatcher to protect certain nodes from being marked as dead
    fn find_dead_branches(&self, root_matcher: Option<&RootNodeMatcher>) -> HashSet<String>;
}

// Implementation of GraphAnalyzer for TCGraph
impl GraphAnalyzer for TCGraph {
    fn find_orphans(&self) -> HashSet<String> {
        let dependents_map = self.build_dependents_map();

        self.nodes()
            .filter(|node| {
                // Has no dependencies and no dependents
                node.deps.is_empty() && !dependents_map.contains_key(&node.name)
            })
            .map(|node| node.name.clone())
            .collect()
    }

    fn find_unrequired(&self) -> HashSet<String> {
        let dependents_map = self.build_dependents_map();

        self.nodes()
            .filter(|node| {
                // Has no dependents (nothing depends on it)
                !dependents_map.contains_key(&node.name)
            })
            .map(|node| node.name.clone())
            .collect()
    }

    fn find_leaf_nodes(&self) -> HashSet<String> {
        let dependents_map = self.build_dependents_map();

        self.nodes()
            .filter(|node| {
                // Has dependencies but no dependents
                !node.deps.is_empty() && !dependents_map.contains_key(&node.name)
            })
            .map(|node| node.name.clone())
            .collect()
    }

    fn find_root_nodes(&self) -> HashSet<String> {
        let dependents_map = self.build_dependents_map();

        self.nodes()
            .filter(|node| {
                // Has dependents but no dependencies
                node.deps.is_empty() && dependents_map.contains_key(&node.name)
            })
            .map(|node| node.name.clone())
            .collect()
    }

    fn find_dead_branches(&self, root_matcher: Option<&RootNodeMatcher>) -> HashSet<String> {
        // Start with leaf nodes (files with dependencies but no dependents)
        let mut dead_nodes = self.find_leaf_nodes();

        // Remove root nodes from initial leaf nodes
        if let Some(matcher) = root_matcher {
            let node_to_path = self.build_node_to_path_map();
            dead_nodes = matcher.filter_non_roots(&dead_nodes, &node_to_path);
        }

        if dead_nodes.is_empty() {
            return dead_nodes;
        }

        // Build dependents map for efficient lookup
        let dependents_map = self.build_dependents_map();

        // Iteratively add nodes whose only dependents are already in dead_nodes
        // This creates the transitive closure of "dead" nodes
        let mut changed = true;
        while changed {
            changed = false;

            for node in self.nodes() {
                // Skip nodes already identified as dead
                if dead_nodes.contains(&node.name) {
                    continue;
                }

                // Skip root nodes (protected from deletion)
                if let Some(matcher) = root_matcher {
                    if matcher.is_root(&node.name, &node.path) {
                        continue;
                    }
                }

                // Get dependents of this node
                // If this node has no dependents, skip (it's handled by other methods)
                let Some(dependents) = dependents_map.get(&node.name) else {
                    continue;
                };

                // Check if ALL dependents are in dead_nodes
                // If yes, this node will also become a leaf node once dead_nodes are deleted
                if !dependents.is_empty() && dependents.iter().all(|d| dead_nodes.contains(d)) {
                    dead_nodes.insert(node.name.clone());
                    changed = true;
                }
            }
        }

        dead_nodes
    }
}

// Helper methods for TCGraph
impl TCGraph {
    /// Build a reverse dependency map: for each node, list all nodes that depend on it
    /// This is the "dependents" relationship (opposite of "dependencies")
    pub fn build_dependents_map(&self) -> HashMap<String, HashSet<String>> {
        let mut dependents: HashMap<String, HashSet<String>> = HashMap::new();

        for node in self.nodes() {
            for dep in &node.deps {
                dependents
                    .entry(dep.clone())
                    .or_default()
                    .insert(node.name.clone());
            }
        }

        dependents
    }

    /// Build a map from node names to their file paths
    /// Used by RootNodeMatcher to check path-based patterns
    pub fn build_node_to_path_map(&self) -> HashMap<String, std::path::PathBuf> {
        self.nodes()
            .map(|node| (node.name.clone(), node.path.clone()))
            .collect()
    }
}
