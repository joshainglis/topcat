use std::collections::HashMap;
use std::path::PathBuf;

use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::file_node::FileNode;
use topcat::logging::Logger;

use super::common::{
    group_nodes_by_schema, is_cross_schema_edge, sanitize_schema_name, write_output_file,
};

/// Renders a single Mermaid schema subgraph.
fn render_mermaid_schema_subgraph(
    output: &mut String,
    schema_name: &str,
    nodes: &[&FileNode],
    node_id_map: &HashMap<String, String>,
) {
    output.push_str(&format!(
        "    subgraph {}\n",
        sanitize_schema_name(schema_name)
    ));

    for node in nodes {
        let node_id = node_id_map
            .get(&node.name)
            .expect("node should exist in node_id_map");
        output.push_str(&format!("        {}[\"{}\"]\n", node_id, node.name));
    }

    output.push_str("    end\n");
}

/// Renders Mermaid edges, marking cross-schema dependencies with dotted lines.
fn render_mermaid_edges(
    output: &mut String,
    all_nodes: &[FileNode],
    node_id_map: &HashMap<String, String>,
    node_map: &HashMap<String, &FileNode>,
) {
    for node in all_nodes {
        let source_id = node_id_map
            .get(&node.name)
            .expect("node should exist in node_id_map");

        for dep in &node.deps {
            if let Some(target_id) = node_id_map.get(dep) {
                if is_cross_schema_edge(node, dep, node_map) {
                    output.push_str(&format!("    {source_id} -.-> {target_id}\n"));
                } else {
                    output.push_str(&format!("    {source_id} --> {target_id}\n"));
                }
            }
        }
    }
}

/// Exports the graph to Mermaid diagram format.
///
/// Generates a Mermaid flowchart with:
/// - Nodes grouped by schema (using subgraphs)
/// - Cross-schema edges shown as dotted lines (-.->)
/// - Top-down layout
///
/// The output can be rendered in Markdown viewers that support Mermaid,
/// or using the Mermaid CLI:
/// ```bash
/// mmdc -i graph.md -o graph.png
/// ```
///
/// # Errors
///
/// Returns an error if file writing fails.
pub fn export_mermaid(
    graph: &TCGraph,
    output_path: &PathBuf,
    logger: &Logger,
) -> Result<(), TopCatError> {
    let mut output = String::new();
    output.push_str("```mermaid\n");
    output.push_str("graph TD\n");

    // Get all nodes and build lookup maps
    let all_nodes = graph.get_all_nodes();
    let node_refs: Vec<&FileNode> = all_nodes.iter().collect();
    let node_map: HashMap<String, &FileNode> =
        all_nodes.iter().map(|n| (n.name.clone(), n)).collect();

    // Group nodes by schema
    let schema_nodes = group_nodes_by_schema(&node_refs);

    // Create node ID mapping (mermaid needs simple IDs)
    let node_id_map: HashMap<String, String> = all_nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.clone(), format!("N{i}")))
        .collect();

    // Render schema subgraphs
    for (schema, nodes) in schema_nodes.iter() {
        if let Some(schema_name) = schema {
            render_mermaid_schema_subgraph(&mut output, schema_name, nodes, &node_id_map);
        } else {
            // Nodes without schema
            for node in nodes {
                let node_id = node_id_map
                    .get(&node.name)
                    .expect("node should exist in node_id_map");
                output.push_str(&format!("    {}[\"{}\"]\n", node_id, node.name));
            }
        }
    }

    output.push('\n');

    // Render edges
    render_mermaid_edges(&mut output, &all_nodes, &node_id_map, &node_map);

    output.push_str("```\n");
    write_output_file(output_path, &output)?;

    logger.success(&format!(
        "Exported Mermaid diagram to: {}",
        output_path.display()
    ));
    Ok(())
}
