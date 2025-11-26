//! Tree data structures and building for dependency visualization.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Represents a node in a tree structure for display purposes.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: Option<PathBuf>,
    pub children: Vec<TreeNode>,
    pub additional_parents: Vec<String>,
}

impl TreeNode {
    /// Create a new tree node.
    pub fn new(name: String, path: Option<PathBuf>) -> Self {
        Self {
            name,
            path,
            children: Vec::new(),
            additional_parents: Vec::new(),
        }
    }

    /// Add a child to this node.
    pub fn add_child(&mut self, child: TreeNode) {
        self.children.push(child);
    }

    /// Add an additional parent reference.
    pub fn add_additional_parent(&mut self, parent: String) {
        self.additional_parents.push(parent);
    }
}

/// Build a forest of trees from dependency relationships with stable ordering.
///
/// Converts a DAG represented as dependency maps into a forest of trees for display.
/// Uses stable sorting based on node names for consistent output across runs.
///
/// Trees are built from root nodes (no dependencies) downward through their dependents.
/// Handles nodes with multiple dependencies by selecting a primary parent and
/// marking additional dependencies.
///
/// # Arguments
///
/// * `nodes` - Set of node names to include in the forest
/// * `deps_map` - Map of node -> dependencies (used to find roots)
/// * `dependents_map` - Map of node -> dependents (used to traverse tree)
/// * `node_paths` - Map of node names to their file paths
///
/// # Returns
///
/// A vector of root `TreeNode` objects, one for each disconnected tree
pub fn build_tree_forest(
    nodes: &HashSet<String>,
    deps_map: &HashMap<String, HashSet<String>>,
    dependents_map: &HashMap<String, HashSet<String>>,
    node_paths: &HashMap<String, PathBuf>,
) -> Vec<TreeNode> {
    let mut visited = HashSet::new();
    let mut forest = Vec::new();

    // Find root nodes (nodes with no dependencies within the set)
    let mut roots: Vec<_> = nodes
        .iter()
        .filter(|node| {
            let deps = deps_map.get(*node).cloned().unwrap_or_default();
            let internal_deps: HashSet<_> = deps.intersection(nodes).collect();
            internal_deps.is_empty()
        })
        .cloned()
        .collect();

    // Stable sort for consistent ordering
    roots.sort();

    // Build a tree for each root node
    for root in roots {
        if !visited.contains(&root) {
            let tree = build_tree_recursive(
                &root,
                nodes,
                deps_map,
                dependents_map,
                node_paths,
                &mut visited,
                None,
            );
            forest.push(tree);
        }
    }

    forest
}

/// Recursively build a tree from a starting node.
///
/// Builds a tree from a root node downward through its dependents.
/// Uses stable sorting for consistent output across runs.
fn build_tree_recursive(
    node_name: &str,
    all_nodes: &HashSet<String>,
    deps_map: &HashMap<String, HashSet<String>>,
    dependents_map: &HashMap<String, HashSet<String>>,
    node_paths: &HashMap<String, PathBuf>,
    visited: &mut HashSet<String>,
    primary_parent: Option<&str>,
) -> TreeNode {
    let mut tree_node = TreeNode::new(node_name.to_string(), node_paths.get(node_name).cloned());

    visited.insert(node_name.to_string());

    // Find all dependencies (parents) within the set - nodes this one depends on
    let mut all_dependencies: Vec<_> = deps_map
        .get(node_name)
        .map(|deps| {
            deps.iter()
                .filter(|p| all_nodes.contains(*p))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    // Stable sort for consistent ordering
    all_dependencies.sort();

    // Mark additional dependencies (those other than the primary parent)
    if let Some(primary) = primary_parent {
        for dep in all_dependencies.iter() {
            if dep != primary {
                tree_node.add_additional_parent(dep.clone());
            }
        }
    }

    // Get children (dependents within the set) - nodes that depend on this one
    let mut children: Vec<_> = dependents_map
        .get(node_name)
        .map(|deps| {
            deps.iter()
                .filter(|d| all_nodes.contains(*d))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    // Stable sort for consistent ordering
    children.sort();

    // Build child trees (traverse dependents)
    for child in children {
        if visited.contains(&child) {
            // Node already shown elsewhere, add a reference
            let mut ref_node = TreeNode::new(child.clone(), None);
            ref_node
                .additional_parents
                .push("[shown above]".to_string());
            tree_node.add_child(ref_node);
        } else {
            let child_tree = build_tree_recursive(
                &child,
                all_nodes,
                deps_map,
                dependents_map,
                node_paths,
                visited,
                Some(node_name),
            );
            tree_node.add_child(child_tree);
        }
    }

    tree_node
}
