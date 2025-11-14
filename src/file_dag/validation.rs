//! Dependency validation and cycle detection for the file DAG.

use std::collections::HashMap;
use std::rc::Rc;

use graph_cycles::Cycles;
use petgraph::algo::is_cyclic_directed;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::{Directed, Graph};

use crate::exceptions::TopCatError;
use crate::file_node::FileNode;

/// Validate dependencies and enforce layer ordering constraints.
///
/// This function checks that:
/// - All `ensure_exists` dependencies exist in the name_map
/// - All `deps` (hard dependencies) exist in the name_map
/// - Layer ordering is respected (nodes in earlier layers cannot depend on later layers)
/// - Edges are added to the appropriate layer graphs
pub(super) fn validate_dependencies(
    name_map: &HashMap<String, Rc<FileNode>>,
    layer_graphs: &mut HashMap<String, DiGraph<Rc<FileNode>, ()>>,
    layer_index_maps: &HashMap<String, HashMap<String, NodeIndex>>,
    layers: &[String],
) -> Result<(), TopCatError> {
    // Create a map from layer name to its index for dependency validation
    let layer_indices: HashMap<String, usize> = layers
        .iter()
        .enumerate()
        .map(|(i, layer)| (layer.clone(), i))
        .collect();

    for file_node in name_map.values() {
        for ensure in &file_node.ensure_exists {
            if !name_map.contains_key(ensure) {
                return Err(TopCatError::MissingExist(
                    file_node.name.clone(),
                    ensure.clone(),
                ));
            }
        }

        for dep in &file_node.deps {
            let dep_node = name_map.get(dep).ok_or_else(|| {
                TopCatError::MissingDependency(file_node.name.clone(), dep.clone())
            })?;

            let file_layer_idx = layer_indices
                .get(&file_node.layer)
                .expect("Layer index should exist for node layer");
            let dep_layer_idx = layer_indices
                .get(&dep_node.layer)
                .expect("Layer index should exist for dependency layer");

            // Enforce layer ordering: lower index layers cannot depend on higher index layers
            if file_layer_idx < dep_layer_idx {
                return Err(TopCatError::InvalidDependency(
                    file_node.name.clone(),
                    format!(
                        "Node in layer '{}' (index {}) cannot depend on node '{}' in layer '{}' (index {})",
                        file_node.layer,
                        file_layer_idx,
                        dep.clone(),
                        dep_node.layer,
                        dep_layer_idx
                    ),
                ));
            }

            // Only add edges within the same layer
            if file_node.layer == dep_node.layer {
                let graph = layer_graphs
                    .get_mut(&file_node.layer)
                    .expect("Layer graph should exist for node layer");
                let index_map = layer_index_maps
                    .get(&file_node.layer)
                    .expect("Layer index map should exist for node layer");
                graph.add_edge(
                    *index_map
                        .get(dep)
                        .expect("Dependency node index should exist in map"),
                    *index_map
                        .get(&file_node.name)
                        .expect("Node index should exist in map"),
                    (),
                );
            }
        }
    }
    Ok(())
}

/// Extract FileNode instances from a cycle of node indices.
fn extract_cycle_nodes(
    cycle: Vec<NodeIndex>,
    graph: &Graph<Rc<FileNode>, (), Directed>,
) -> Vec<FileNode> {
    cycle
        .iter()
        .map(|n| {
            let rc_node = graph
                .node_weight(*n)
                .expect("Cycle node should exist in graph");
            (**rc_node).clone()
        })
        .collect()
}

/// Convert a list of cycles (as node indices) to cycles of FileNodes.
fn convert_cycle_indexes_to_cycle_nodes(
    cycles: Vec<Vec<NodeIndex>>,
    graph: &Graph<Rc<FileNode>, (), Directed>,
) -> Vec<Vec<FileNode>> {
    cycles
        .iter()
        .map(|c| extract_cycle_nodes(c.clone(), graph))
        .collect()
}

/// Check all layer graphs for cyclic dependencies.
///
/// Returns an error containing all detected cycles if any are found.
pub(super) fn check_cyclic_dependencies(
    layer_graphs: &HashMap<String, DiGraph<Rc<FileNode>, ()>>,
) -> Result<(), TopCatError> {
    let mut cycles: Vec<Vec<FileNode>> = Vec::new();

    for graph in layer_graphs.values() {
        if is_cyclic_directed(graph) {
            cycles.extend(convert_cycle_indexes_to_cycle_nodes(graph.cycles(), graph));
        }
    }

    if !cycles.is_empty() {
        return Err(TopCatError::CyclicDependency(cycles));
    }
    Ok(())
}
