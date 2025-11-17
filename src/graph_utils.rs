//! Graph utility functions for working with dependency graphs.
//!
//! This module provides helper functions and traits for common operations
//! on `TCGraph` instances, particularly for efficient node lookups and mappings.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::file_node::FileNode;

/// Build a HashMap for O(1) node lookups by name.
///
/// This helper improves performance from O(n²) to O(n) for analyses that need
/// to look up node details repeatedly. Instead of linear searching through all
/// nodes for each result, build a hash map once and use O(1) lookups.
///
/// # Arguments
///
/// * `nodes` - Slice of all file nodes in the graph
///
/// # Returns
///
/// HashMap mapping node names to node references for fast lookup
///
/// # Examples
///
/// ```no_run
/// use topcat::file_dag::TCGraph;
/// use topcat::graph_utils::build_name_to_node_map;
/// use topcat::config::Config;
///
/// # let config: Config = unimplemented!();
/// # let graph = TCGraph::new(&config);
/// let nodes = graph.get_all_nodes();
/// let node_map = build_name_to_node_map(&nodes);
///
/// // Fast O(1) lookup by node name
/// if let Some(node) = node_map.get("my_node") {
///     println!("Found node: {}", node.name);
/// }
/// ```
pub fn build_name_to_node_map(nodes: &[FileNode]) -> HashMap<&str, &FileNode> {
    nodes.iter().map(|n| (n.name.as_str(), n)).collect()
}

/// Build a HashMap for O(1) lookups of file paths by node name.
///
/// This is useful when you need to quickly find the file path associated
/// with a node name without scanning through the entire node list.
///
/// # Arguments
///
/// * `nodes` - Slice of all file nodes in the graph
///
/// # Returns
///
/// HashMap mapping node names (owned String) to their file paths
///
/// # Examples
///
/// ```no_run
/// use topcat::file_dag::TCGraph;
/// use topcat::graph_utils::build_name_to_path_map;
/// use topcat::config::Config;
///
/// # let config: Config = unimplemented!();
/// # let graph = TCGraph::new(&config);
/// let nodes = graph.get_all_nodes();
/// let path_map = build_name_to_path_map(&nodes);
///
/// // Fast O(1) lookup of file path by node name
/// if let Some(path) = path_map.get("my_node") {
///     println!("Node file: {:?}", path);
/// }
/// ```
pub fn build_name_to_path_map(nodes: &[FileNode]) -> HashMap<String, PathBuf> {
    nodes
        .iter()
        .map(|n| (n.name.clone(), n.path.clone()))
        .collect()
}

/// Build a HashMap for O(1) lookups of nodes by their file path.
///
/// This is useful when you have a file path and need to find the corresponding
/// node in the graph quickly.
///
/// # Arguments
///
/// * `nodes` - Slice of all file nodes in the graph
///
/// # Returns
///
/// HashMap mapping file paths to node references
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use topcat::file_dag::TCGraph;
/// use topcat::graph_utils::build_path_to_node_map;
/// use topcat::config::Config;
///
/// # let config: Config = unimplemented!();
/// # let graph = TCGraph::new(&config);
/// let nodes = graph.get_all_nodes();
/// let path_map = build_path_to_node_map(&nodes);
///
/// // Fast O(1) lookup of node by file path
/// let path = PathBuf::from("my_file.sql");
/// if let Some(node) = path_map.get(&path) {
///     println!("Found node: {}", node.name);
/// }
/// ```
pub fn build_path_to_node_map(nodes: &[FileNode]) -> HashMap<PathBuf, &FileNode> {
    nodes.iter().map(|n| (n.path.clone(), n)).collect()
}

/// Count nodes by their layer.
///
/// Returns a HashMap showing how many nodes are in each layer of the graph.
/// This is useful for understanding the distribution of files across layers.
///
/// # Arguments
///
/// * `nodes` - Slice of all file nodes in the graph
///
/// # Returns
///
/// HashMap mapping layer names to the count of nodes in that layer
///
/// # Examples
///
/// ```no_run
/// use topcat::file_dag::TCGraph;
/// use topcat::graph_utils::count_nodes_by_layer;
/// use topcat::config::Config;
///
/// # let config: Config = unimplemented!();
/// # let graph = TCGraph::new(&config);
/// let nodes = graph.get_all_nodes();
/// let layer_counts = count_nodes_by_layer(&nodes);
///
/// for (layer, count) in layer_counts {
///     println!("{}: {} nodes", layer, count);
/// }
/// ```
pub fn count_nodes_by_layer(nodes: &[FileNode]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for node in nodes {
        *counts.entry(node.layer.clone()).or_insert(0) += 1;
    }
    counts
}

/// Find all nodes in a specific layer.
///
/// Returns a vector of references to nodes that belong to the specified layer.
///
/// # Arguments
///
/// * `nodes` - Slice of all file nodes in the graph
/// * `layer` - Name of the layer to filter by
///
/// # Returns
///
/// Vector of references to nodes in the specified layer
///
/// # Examples
///
/// ```no_run
/// use topcat::file_dag::TCGraph;
/// use topcat::graph_utils::filter_nodes_by_layer;
/// use topcat::config::Config;
///
/// # let config: Config = unimplemented!();
/// # let graph = TCGraph::new(&config);
/// let nodes = graph.get_all_nodes();
/// let normal_nodes = filter_nodes_by_layer(&nodes, "normal");
///
/// println!("Found {} nodes in 'normal' layer", normal_nodes.len());
/// ```
pub fn filter_nodes_by_layer<'a>(nodes: &'a [FileNode], layer: &str) -> Vec<&'a FileNode> {
    nodes.iter().filter(|n| n.layer == layer).collect()
}

/// Group nodes by their schema.
///
/// Returns a HashMap grouping nodes by their schema name. Nodes without
/// a schema are grouped under an empty string key.
///
/// # Arguments
///
/// * `nodes` - Slice of all file nodes in the graph
///
/// # Returns
///
/// HashMap mapping schema names to vectors of nodes in that schema
///
/// # Examples
///
/// ```no_run
/// use topcat::file_dag::TCGraph;
/// use topcat::graph_utils::group_nodes_by_schema;
/// use topcat::config::Config;
///
/// # let config: Config = unimplemented!();
/// # let graph = TCGraph::new(&config);
/// let nodes = graph.get_all_nodes();
/// let schema_groups = group_nodes_by_schema(&nodes);
///
/// for (schema, nodes) in schema_groups {
///     println!("Schema '{}': {} nodes", schema, nodes.len());
/// }
/// ```
pub fn group_nodes_by_schema(nodes: &[FileNode]) -> HashMap<String, Vec<&FileNode>> {
    let mut groups: HashMap<String, Vec<&FileNode>> = HashMap::new();
    for node in nodes {
        let schema = node.schema.clone().unwrap_or_default();
        groups.entry(schema).or_default().push(node);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_node::NameSource;
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn create_test_nodes() -> Vec<FileNode> {
        let node1_deps = HashSet::new();
        let mut node2_deps = HashSet::new();
        node2_deps.insert("node1".to_string());
        let node3_deps = HashSet::new();

        vec![
            FileNode {
                name: "node1".to_string(),
                path: PathBuf::from("/path/to/node1.sql"),
                deps: node1_deps.clone(),
                schema: Some("auth".to_string()),
                layer: "normal".to_string(),
                ensure_exists: HashSet::new(),
                discovered_deps: None,
                override_deps: HashSet::new(),
                name_source: NameSource::Header,
                implicit: false,
            },
            FileNode {
                name: "node2".to_string(),
                path: PathBuf::from("/path/to/node2.sql"),
                deps: node2_deps,
                schema: Some("auth".to_string()),
                layer: "normal".to_string(),
                ensure_exists: HashSet::new(),
                discovered_deps: None,
                override_deps: HashSet::new(),
                name_source: NameSource::Header,
                implicit: false,
            },
            FileNode {
                name: "node3".to_string(),
                path: PathBuf::from("/path/to/node3.sql"),
                deps: node3_deps,
                schema: Some("billing".to_string()),
                layer: "prepend".to_string(),
                ensure_exists: HashSet::new(),
                discovered_deps: None,
                override_deps: HashSet::new(),
                name_source: NameSource::Header,
                implicit: false,
            },
        ]
    }

    #[test]
    fn test_build_name_to_node_map() {
        let nodes = create_test_nodes();
        let map = build_name_to_node_map(&nodes);

        assert_eq!(map.len(), 3);
        assert!(map.contains_key("node1"));
        assert!(map.contains_key("node2"));
        assert!(map.contains_key("node3"));

        let node1 = map.get("node1").unwrap();
        assert_eq!(node1.name, "node1");
    }

    #[test]
    fn test_build_name_to_path_map() {
        let nodes = create_test_nodes();
        let map = build_name_to_path_map(&nodes);

        assert_eq!(map.len(), 3);
        assert_eq!(map.get("node1"), Some(&PathBuf::from("/path/to/node1.sql")));
        assert_eq!(map.get("node2"), Some(&PathBuf::from("/path/to/node2.sql")));
        assert_eq!(map.get("node3"), Some(&PathBuf::from("/path/to/node3.sql")));
    }

    #[test]
    fn test_build_path_to_node_map() {
        let nodes = create_test_nodes();
        let map = build_path_to_node_map(&nodes);

        assert_eq!(map.len(), 3);

        let path1 = PathBuf::from("/path/to/node1.sql");
        assert!(map.contains_key(&path1));
        let node1 = map.get(&path1).unwrap();
        assert_eq!(node1.name, "node1");
    }

    #[test]
    fn test_count_nodes_by_layer() {
        let nodes = create_test_nodes();
        let counts = count_nodes_by_layer(&nodes);

        assert_eq!(counts.get("normal"), Some(&2));
        assert_eq!(counts.get("prepend"), Some(&1));
        assert_eq!(counts.get("append"), None);
    }

    #[test]
    fn test_filter_nodes_by_layer() {
        let nodes = create_test_nodes();

        let normal_nodes = filter_nodes_by_layer(&nodes, "normal");
        assert_eq!(normal_nodes.len(), 2);
        assert!(normal_nodes.iter().any(|n| n.name == "node1"));
        assert!(normal_nodes.iter().any(|n| n.name == "node2"));

        let prepend_nodes = filter_nodes_by_layer(&nodes, "prepend");
        assert_eq!(prepend_nodes.len(), 1);
        assert_eq!(prepend_nodes[0].name, "node3");

        let append_nodes = filter_nodes_by_layer(&nodes, "append");
        assert_eq!(append_nodes.len(), 0);
    }

    #[test]
    fn test_group_nodes_by_schema() {
        let nodes = create_test_nodes();
        let groups = group_nodes_by_schema(&nodes);

        assert_eq!(groups.len(), 2);

        let auth_nodes = groups.get("auth").unwrap();
        assert_eq!(auth_nodes.len(), 2);

        let billing_nodes = groups.get("billing").unwrap();
        assert_eq!(billing_nodes.len(), 1);
        assert_eq!(billing_nodes[0].name, "node3");
    }

    #[test]
    fn test_empty_nodes() {
        let nodes: Vec<FileNode> = vec![];

        assert_eq!(build_name_to_node_map(&nodes).len(), 0);
        assert_eq!(build_name_to_path_map(&nodes).len(), 0);
        assert_eq!(build_path_to_node_map(&nodes).len(), 0);
        assert_eq!(count_nodes_by_layer(&nodes).len(), 0);
        assert_eq!(filter_nodes_by_layer(&nodes, "normal").len(), 0);
        assert_eq!(group_nodes_by_schema(&nodes).len(), 0);
    }
}
