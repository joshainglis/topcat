use std::collections::HashMap;
use std::path::PathBuf;

use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::file_node::FileNode;
use topcat::logging::Logger;

use super::common::{
    SCHEMA_COLORS, group_nodes_by_schema, is_cross_schema_edge, sanitize_schema_name,
    write_output_file,
};

/// Renders a single DOT schema subgraph with colored nodes.
fn render_dot_schema_subgraph(
    output: &mut String,
    schema_name: &str,
    nodes: &[&FileNode],
    color: &str,
) {
    output.push_str(&format!(
        "    subgraph cluster_{} {{\n",
        sanitize_schema_name(schema_name)
    ));
    output.push_str(&format!("        label=\"{schema_name}\";\n"));
    output.push_str("        style=filled;\n");
    output.push_str(&format!("        color={color};\n"));
    output.push_str(&format!("        fillcolor={color};\n\n"));

    for node in nodes {
        output.push_str(&format!("        \"{}\";\n", node.name));
    }

    output.push_str("    }\n\n");
}

/// Renders DOT edges, marking cross-schema dependencies with dashed red lines.
fn render_dot_edges(
    output: &mut String,
    all_nodes: &[FileNode],
    node_map: &HashMap<String, &FileNode>,
) {
    output.push_str("    // Dependencies\n");
    for node in all_nodes {
        for dep in &node.deps {
            if is_cross_schema_edge(node, dep, node_map) {
                output.push_str(&format!(
                    "    \"{}\" -> \"{}\" [color=red, style=dashed];\n",
                    node.name, dep
                ));
            } else {
                output.push_str(&format!("    \"{}\" -> \"{}\";\n", node.name, dep));
            }
        }
    }
}

/// Exports the graph to DOT format for GraphViz visualization.
///
/// Generates a directed graph in DOT format with:
/// - Nodes grouped by schema (with colored subgraphs)
/// - Cross-schema edges shown as red dashed lines
/// - Left-to-right layout (rankdir=LR)
///
/// The output can be visualized using GraphViz tools like `dot`:
/// ```bash
/// dot -Tpng graph.dot -o graph.png
/// ```
///
/// # Errors
///
/// Returns an error if file writing fails.
pub fn export_dot(
    graph: &TCGraph,
    output_path: &PathBuf,
    logger: &Logger,
) -> Result<(), TopCatError> {
    let mut output = String::new();
    output.push_str("digraph dependencies {\n");
    output.push_str("    rankdir=LR;\n");
    output.push_str("    node [shape=box, style=filled];\n\n");

    // Get all nodes and build lookup map
    let all_nodes = graph.get_all_nodes();
    let node_refs: Vec<&FileNode> = all_nodes.iter().collect();
    let node_map: HashMap<String, &FileNode> =
        all_nodes.iter().map(|n| (n.name.clone(), n)).collect();

    // Group nodes by schema
    let schema_nodes = group_nodes_by_schema(&node_refs);

    // Render schema subgraphs
    let mut color_idx = 0;
    for (schema, nodes) in schema_nodes.iter() {
        if let Some(schema_name) = schema {
            let color = SCHEMA_COLORS[color_idx % SCHEMA_COLORS.len()];
            color_idx += 1;
            render_dot_schema_subgraph(&mut output, schema_name, nodes, color);
        } else {
            // Nodes without schema
            for node in nodes {
                output.push_str(&format!("    \"{}\" [fillcolor=white];\n", node.name));
            }
            output.push('\n');
        }
    }

    // Render edges
    render_dot_edges(&mut output, &all_nodes, &node_map);

    output.push_str("}\n");
    write_output_file(output_path, &output)?;

    logger.success(&format!("Exported DOT to: {}", output_path.display()));
    Ok(())
}
