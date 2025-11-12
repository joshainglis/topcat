use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use topcat::exceptions::TopCatError;
use topcat::file_node::FileNode;

// Constants for export formatting
pub const SCHEMA_COLORS: &[&str] = &[
    "lightblue",
    "lightgreen",
    "lightyellow",
    "lightpink",
    "lightcyan",
    "lavender",
];

pub const NODE_TYPE_ROOT: &str = "root";
pub const NODE_TYPE_LEAF: &str = "leaf";
pub const NODE_TYPE_INTERMEDIATE: &str = "intermediate";
pub const EDGE_TYPE_REQUIRES: &str = "requires";

/// Writes content to a file, handling I/O errors appropriately.
pub fn write_output_file(path: &PathBuf, content: &str) -> Result<(), TopCatError> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

/// Groups nodes by their schema, returning a map of schema -> nodes.
pub fn group_nodes_by_schema<'a>(
    nodes: &'a [&FileNode],
) -> HashMap<Option<String>, Vec<&'a FileNode>> {
    let mut schema_nodes: HashMap<Option<String>, Vec<&'a FileNode>> = HashMap::new();
    for node in nodes {
        schema_nodes
            .entry(node.schema.clone())
            .or_default()
            .push(node);
    }
    schema_nodes
}

/// Sanitizes schema names for use in GraphViz/Mermaid identifiers.
pub fn sanitize_schema_name(name: &str) -> String {
    name.replace('.', "_").replace("::", "_")
}

/// Checks if an edge crosses schema boundaries.
pub fn is_cross_schema_edge(
    source_node: &FileNode,
    target_name: &str,
    node_map: &HashMap<String, &FileNode>,
) -> bool {
    match (source_node.schema.as_ref(), node_map.get(target_name)) {
        (Some(src_schema), Some(target_node)) => Some(src_schema) != target_node.schema.as_ref(),
        _ => false, // If target is missing, don't mark as cross-schema
    }
}
