use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

use super::common::{
    EDGE_TYPE_REQUIRES, NODE_TYPE_INTERMEDIATE, NODE_TYPE_LEAF, NODE_TYPE_ROOT, write_output_file,
};

// JSON export data structures

/// Top-level JSON export structure containing all graph data.
#[derive(Debug, Serialize, Deserialize)]
struct JsonExport {
    /// Export metadata (version, timestamp, counts)
    metadata: MetadataExport,
    /// Schema information and statistics
    schemas: Vec<SchemaExport>,
    /// Node details with dependencies and attributes
    nodes: Vec<NodeExport>,
    /// Edge list representing dependencies
    edges: Vec<EdgeExport>,
}

/// Metadata about the export and graph statistics.
#[derive(Debug, Serialize, Deserialize)]
struct MetadataExport {
    /// Topcat version that generated this export
    version: String,
    /// ISO 8601 timestamp of export generation
    generated_at: String,
    /// Total number of nodes in the graph
    node_count: usize,
    /// Total number of edges in the graph
    edge_count: usize,
    /// Number of distinct schemas
    schema_count: usize,
}

/// Schema-level statistics and metadata.
#[derive(Debug, Serialize, Deserialize)]
struct SchemaExport {
    /// Schema name
    name: String,
    /// Number of nodes in this schema
    node_count: usize,
    /// Number of dependencies within this schema
    internal_deps: usize,
    /// Number of dependencies to other schemas
    external_deps: usize,
}

/// Individual node with all its attributes and relationships.
#[derive(Debug, Serialize, Deserialize)]
struct NodeExport {
    /// Unique node name
    name: String,
    /// File path
    path: String,
    /// Layer assignment (e.g., "prepend", "normal", "append")
    layer: String,
    /// Schema this node belongs to (if any)
    schema: Option<String>,
    /// List of node names this node depends on
    dependencies: Vec<String>,
    /// List of node names that depend on this node
    dependents: Vec<String>,
    /// Node classification: "root", "leaf", or "intermediate"
    node_type: String,
}

/// Directed edge in the dependency graph.
#[derive(Debug, Serialize, Deserialize)]
struct EdgeExport {
    /// Source node name
    source: String,
    /// Target node name
    target: String,
    /// Edge type (e.g., "requires")
    edge_type: String,
}

impl JsonExport {
    fn from_graph(graph: &TCGraph) -> Self {
        let all_nodes = graph.get_all_nodes();
        let schemas = graph.get_schemas();

        // Count edges
        let edge_count: usize = all_nodes.iter().map(|n| n.deps.len()).sum();

        // Build dependents map
        let dependents_map = graph.build_dependents_map();

        // Export schemas
        let schema_exports: Vec<SchemaExport> = schemas
            .iter()
            .map(|(name, nodes)| {
                let internal_deps = graph.get_internal_dependencies(name).len();
                let external_deps = graph.get_external_dependencies(name).len();
                SchemaExport {
                    name: name.clone(),
                    node_count: nodes.len(),
                    internal_deps,
                    external_deps,
                }
            })
            .collect();

        // Export nodes
        let nodes: Vec<NodeExport> = all_nodes
            .iter()
            .map(|node| {
                let dependencies: Vec<String> = node.deps.iter().cloned().collect();
                let dependents: Vec<String> = dependents_map
                    .get(&node.name)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect();

                let node_type = if dependents.is_empty() {
                    NODE_TYPE_LEAF
                } else if dependencies.is_empty() {
                    NODE_TYPE_ROOT
                } else {
                    NODE_TYPE_INTERMEDIATE
                };

                NodeExport {
                    name: node.name.clone(),
                    path: node.path.display().to_string(),
                    layer: node.layer.clone(),
                    schema: node.schema.clone(),
                    dependencies,
                    dependents,
                    node_type: node_type.to_string(),
                }
            })
            .collect();

        // Export edges
        let edges: Vec<EdgeExport> = all_nodes
            .iter()
            .flat_map(|node| {
                node.deps
                    .iter()
                    .map(|dep| EdgeExport {
                        source: node.name.clone(),
                        target: dep.clone(),
                        edge_type: EDGE_TYPE_REQUIRES.to_string(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        JsonExport {
            metadata: MetadataExport {
                version: env!("CARGO_PKG_VERSION").to_string(),
                generated_at: Utc::now().to_rfc3339(),
                node_count: all_nodes.len(),
                edge_count,
                schema_count: schemas.len(),
            },
            schemas: schema_exports,
            nodes,
            edges,
        }
    }
}

/// Exports the graph to JSON format with full metadata.
///
/// The JSON export includes:
/// - Metadata (version, timestamp, counts)
/// - Schema information (node counts, dependency counts)
/// - Node details (name, path, layer, schema, dependencies, dependents, type)
/// - Edge list (source, target, type)
///
/// # Errors
///
/// Returns an error if JSON serialization or file writing fails.
pub fn export_json(
    graph: &TCGraph,
    output_path: &PathBuf,
    logger: &Logger,
) -> Result<(), TopCatError> {
    let export_data = JsonExport::from_graph(graph);
    let json = serde_json::to_string_pretty(&export_data)
        .map_err(|e| TopCatError::SerializationError(format!("JSON serialization failed: {e}")))?;

    write_output_file(output_path, &json)?;

    logger.success(&format!("Exported JSON to: {}", output_path.display()));
    Ok(())
}
