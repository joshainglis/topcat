//! Dependency validation and cycle detection for the file DAG.
//!
//! This module provides two critical validation operations:
//!
//! # 1. Dependency Validation
//!
//! Ensures the dependency graph is well-formed and respects layer constraints:
//!
//! - **Existence checking**: All declared dependencies (`requires`, `exists`) must reference valid nodes
//! - **Layer ordering**: Files in earlier layers cannot depend on files in later layers
//! - **Graph construction**: Builds per-layer directed graphs for cycle detection
//!
//! ## Layer Ordering Rules
//!
//! Given layers `["prepend", "normal", "append"]`:
//! - ✅ `normal` can depend on `prepend` (same or earlier layer)
//! - ✅ `append` can depend on `normal` (same or earlier layer)
//! - ❌ `prepend` **cannot** depend on `normal` (later layer)
//! - ❌ `normal` **cannot** depend on `append` (later layer)
//!
//! This ensures files are always ordered consistently: prepend → normal → append
//!
//! # 2. Cycle Detection
//!
//! Detects circular dependencies within each layer using two algorithms:
//!
//! ## Algorithm 1: Petgraph's `is_cyclic_directed`
//!
//! Fast cycle existence check using depth-first search (DFS):
//! - Time: O(V + E) - visits each vertex and edge once
//! - Returns: Boolean indicating if any cycles exist
//!
//! ## Algorithm 2: Johnson's Algorithm (via `graph_cycles` crate)
//!
//! Enumerates **all** simple cycles when cycles are detected:
//! - Time: O((V + E)(C + 1)) where C is the number of cycles
//! - Returns: Complete list of all cycles for error reporting
//!
//! ## Why Both Algorithms?
//!
//! - **Performance**: Check existence first (fast) before enumerating all cycles (slower)
//! - **User experience**: Provide detailed cycle information for debugging
//! - **Layer separation**: Cycles can only exist within a single layer (cross-layer deps are validated separately)
//!
//! # Examples
//!
//! ## Valid dependency structure
//!
//! ```text
//! Layer "prepend":
//!   schema.sql (no deps)
//!
//! Layer "normal":
//!   users.sql (requires: schema.sql)  ✅ depends on earlier layer
//!   posts.sql (requires: users.sql)   ✅ depends on same layer
//!
//! Layer "append":
//!   views.sql (requires: posts.sql)   ✅ depends on earlier layer
//! ```
//!
//! ## Invalid: Cross-layer violation
//!
//! ```text
//! Layer "prepend":
//!   schema.sql (requires: users.sql)  ❌ depends on later layer!
//!
//! Layer "normal":
//!   users.sql (no deps)
//! ```
//!
//! ## Invalid: Circular dependency
//!
//! ```text
//! Layer "normal":
//!   a.sql (requires: b.sql)
//!   b.sql (requires: c.sql)
//!   c.sql (requires: a.sql)  ❌ cycle: a → b → c → a
//! ```

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
/// This is the first validation pass that ensures the dependency graph is well-formed
/// before attempting cycle detection.
///
/// # Validation Checks
///
/// 1. **Soft dependencies (`ensure_exists`)**: Verify all referenced nodes exist
///    - These dependencies don't create edges, just ensure inclusion
///
/// 2. **Hard dependencies (`deps`, `dropped_by`)**: Verify all referenced nodes exist
///    - These dependencies create directed edges in the graph
///
/// 3. **Layer ordering constraints**: Ensure earlier layers don't depend on later layers
///    - Prevents: `prepend` depending on `normal` or `append`
///    - Allows: `append` depending on `normal` or `prepend`
///
/// 4. **Graph construction**: Add edges to per-layer dependency graphs
///    - Only edges within the same layer are added to layer graphs
///    - Cross-layer dependencies are validated but not added as edges
///    - This allows per-layer cycle detection
///
/// # Returns
///
/// - `Ok(())` if all validations pass
/// - `Err(TopCatError::MissingExist)` if a soft dependency is missing
/// - `Err(TopCatError::MissingDependency)` if a hard dependency is missing
/// - `Err(TopCatError::InvalidDependency)` if a layer ordering constraint is violated
///
/// # Example Error
///
/// ```text
/// Error: Node in layer 'prepend' (index 0) cannot depend on node 'users'
///        in layer 'normal' (index 1)
/// ```
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
/// This is the second validation pass that detects circular dependencies within each layer.
/// Uses a two-stage approach:
///
/// 1. **Fast check**: `is_cyclic_directed` (O(V + E)) - quickly determine if cycles exist
/// 2. **Enumerate**: `graph.cycles()` (Johnson's algorithm) - find all cycles for reporting
///
/// # Why Per-Layer Checking?
///
/// Layers are processed independently because:
/// - Cross-layer dependencies are already validated (earlier layers can't depend on later ones)
/// - This guarantees no cycles can span multiple layers
/// - Per-layer checking is more efficient and provides better error messages
///
/// # Returns
///
/// - `Ok(())` if no cycles are detected in any layer
/// - `Err(TopCatError::CyclicDependency)` with all detected cycles
///
/// # Example Error
///
/// ```text
/// Error: Cyclic dependencies detected:
///   Cycle 1: a.sql → b.sql → c.sql → a.sql
///   Cycle 2: d.sql → e.sql → d.sql
/// ```
///
/// # Algorithm Details
///
/// Uses Johnson's algorithm to find all elementary cycles:
/// - Guarantees finding all simple (non-repeating) cycles
/// - Time: O((V + E)(C + 1)) where C is the number of cycles
/// - Space: O(V + E) for graph representation
///
/// Reference: Donald B. Johnson, "Finding all the elementary circuits of a directed graph",
/// SIAM Journal on Computing, 1975.
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
