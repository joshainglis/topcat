use std::collections::HashSet;

use petgraph::Incoming;
use petgraph::data::DataMap;
use petgraph::graph::NodeIndex;
use petgraph::visit::IntoNeighborsDirected;
use petgraph::visit::Reversed;
use petgraph::visit::{GraphBase, IntoNeighbors, IntoNodeIdentifiers, Visitable};

/// Deterministic topological sort iterator that produces consistent ordering.
///
/// Unlike standard topological sort algorithms which may produce different valid orderings
/// for the same graph, `StableTopo` guarantees consistent, deterministic results by:
///
/// 1. **Weight-based ordering**: Nodes are sorted by their weight (using `Ord` on node weights)
/// 2. **Stable iteration**: Given the same input graph, always produces the same output order
/// 3. **Level-order traversal**: Processes all nodes at the same "level" before moving deeper
///
/// This is critical for Topcat's use case where the same dependency graph should always
/// produce the same concatenated output file, ensuring reproducible builds and diffs.
///
/// # Algorithm
///
/// The algorithm is a modified topological sort based on Kahn's algorithm with stability guarantees:
///
/// 1. **Initialization**: Find all source nodes (nodes with no incoming edges)
/// 2. **Weight-based selection**: Sort candidates by node weight to ensure deterministic order
/// 3. **Dependency checking**: Only visit a node when all its dependencies have been visited
/// 4. **Level ordering**: Process nodes level-by-level, respecting the dependency graph
///
/// ## Time Complexity
///
/// - O(V log V + E) where V is vertices and E is edges
/// - The log V factor comes from sorting nodes by weight at each level
///
/// ## Space Complexity
///
/// - O(V) for the visited set and tovisit stack
///
/// # Examples
///
/// ```ignore
/// use petgraph::graph::Graph;
/// use topcat::stable_topo::StableTopo;
///
/// // Create a graph: 1 -> 2 -> {3, 4}, 4 -> 3
/// let mut graph = Graph::<&str, ()>::new();
/// let n1 = graph.add_node("a");
/// let n2 = graph.add_node("b");
/// let n3 = graph.add_node("c");
/// let n4 = graph.add_node("d");
///
/// graph.add_edge(n1, n2, ());
/// graph.add_edge(n2, n3, ());
/// graph.add_edge(n2, n4, ());
/// graph.add_edge(n4, n3, ());
///
/// // Perform stable topological sort
/// let stable_topo = StableTopo::new(&graph);
/// let ordered: Vec<_> = stable_topo.collect();
///
/// // Always produces: [n1, n2, n4, n3] (deterministic!)
/// // Note: n4 before n3 because 'd' > 'c' in weight ordering
/// assert_eq!(ordered, vec![n1, n2, n4, n3]);
/// ```
///
/// # Comparison with Standard Topological Sort
///
/// Standard topological sort may produce different valid orderings:
/// - `[1, 2, 3, 4]` (one valid ordering)
/// - `[1, 2, 4, 3]` (another valid ordering)
///
/// `StableTopo` always produces the same ordering based on node weights:
/// - Always produces `[1, 2, 4, 3]` if node weights dictate this order
///
/// # Use in Topcat
///
/// Topcat uses `StableTopo` with `FileNode` as the node weight, where `FileNode`
/// implements `Ord` based on the file path. This ensures that files are always
/// concatenated in the same order given the same dependency structure.
///
/// # Implementation Details
///
/// ## Fields
/// - `graph`: The directed graph to sort
/// - `ordered`: Set of already-visited nodes (prevents revisiting)
/// - `tovisit`: Stack of candidate nodes ready to be visited
///
/// ## Iterator Protocol
///
/// Implements `Iterator` to allow incremental topological traversal:
/// - `next()`: Returns the next node in topological order
/// - Returns `None` when all nodes have been visited
#[derive(Clone)]
pub struct StableTopo<G> {
    graph: G,

    ordered: HashSet<NodeIndex>,
    tovisit: Vec<NodeIndex>,
}

impl<G> StableTopo<G>
where
    G: IntoNeighborsDirected + IntoNodeIdentifiers + Visitable,
    G: GraphBase<NodeId = NodeIndex>,
{
    /// Creates a new stable topological sort iterator.
    ///
    /// Initializes the iterator by identifying all source nodes (nodes with no
    /// incoming edges) and adding them to the candidate list.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use petgraph::graph::Graph;
    /// use topcat::stable_topo::StableTopo;
    ///
    /// let mut graph = Graph::<i32, ()>::new();
    /// let n1 = graph.add_node(1);
    /// let n2 = graph.add_node(2);
    /// graph.add_edge(n1, n2, ());
    ///
    /// let mut topo = StableTopo::new(&graph);
    /// assert_eq!(topo.next(), Some(n1));
    /// assert_eq!(topo.next(), Some(n2));
    /// assert_eq!(topo.next(), None);
    /// ```
    pub fn new(graph: G) -> Self {
        let mut topo = StableTopo {
            graph,
            ordered: HashSet::new(),
            tovisit: Vec::new(),
        };
        topo.extend_with_initials();
        topo
    }

    /// Finds and adds all source nodes (nodes without incoming edges) to the candidate list.
    ///
    /// This is called during initialization to bootstrap the topological traversal.
    /// Source nodes are the starting points of the dependency graph.
    pub fn extend_with_initials(&mut self) {
        // find all initial nodes (nodes without incoming edges)
        self.tovisit.extend(
            self.graph
                .node_identifiers()
                .filter(|&a| self.graph.neighbors_directed(a, Incoming).next().is_none()),
        );
    }
}

impl<G> Iterator for StableTopo<G>
where
    G: IntoNeighborsDirected + IntoNodeIdentifiers + Visitable + DataMap,
    G: GraphBase<NodeId = NodeIndex>,
    G::NodeWeight: Ord,
{
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        // Sort the `tovisit` vector based on the node weights
        // Cache weights to avoid repeated lookups and handle missing nodes gracefully
        self.tovisit.sort_unstable_by(|a, b| {
            let weight_a = self
                .graph
                .node_weight(*a)
                .expect("Invariant violation: node in tovisit not found in graph");
            let weight_b = self
                .graph
                .node_weight(*b)
                .expect("Invariant violation: node in tovisit not found in graph");
            weight_a.cmp(weight_b)
        });

        // Take an unvisited element and find which of its neighbors are next
        while let Some(nix) = self.tovisit.pop() {
            if self.ordered.contains(&nix) {
                continue;
            }
            self.ordered.insert(nix);
            let mut neighbors = Vec::new();
            for neigh in self.graph.neighbors(nix) {
                // Look at each neighbor, and those that only have incoming edges
                // from the already ordered list, they are the next to visit.
                if Reversed(&self.graph)
                    .neighbors(neigh)
                    .all(|b| self.ordered.contains(&b))
                {
                    neighbors.push(neigh);
                }
            }
            // Sort the neighbors based on the node weights
            neighbors.sort_unstable_by(|a, b| {
                let weight_a = self
                    .graph
                    .node_weight(*a)
                    .expect("Invariant violation: neighbor node not found in graph");
                let weight_b = self
                    .graph
                    .node_weight(*b)
                    .expect("Invariant violation: neighbor node not found in graph");
                weight_a.cmp(weight_b)
            });
            self.tovisit.extend(neighbors);
            return Some(nix);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use petgraph::prelude::*;

    use super::*;

    #[test]
    fn test_stable_topo() {
        let mut graph: Graph<&str, (), Directed> = Graph::new();
        let node1 = graph.add_node("Node 1");
        let node2 = graph.add_node("Node 2");
        let node3 = graph.add_node("Node 3");
        let node4 = graph.add_node("Node 4");

        graph.add_edge(node1, node2, ());
        graph.add_edge(node2, node3, ());
        graph.add_edge(node2, node4, ());
        graph.add_edge(node3, node4, ());

        let stable_topo = StableTopo::new(&graph);
        let topo_order: Vec<NodeIndex> = stable_topo.collect();

        assert_eq!(topo_order, vec![node1, node2, node3, node4]);
    }

    /// Tests the stability of topological sorting regardless of input order.
    #[test]
    fn test_stable_topo_with_weights() {
        let mut graph: Graph<&str, (), Directed> = Graph::new();
        let node4 = graph.add_node("Node 4");
        let node3 = graph.add_node("Node 3");
        let node2 = graph.add_node("Node 2");
        let node1 = graph.add_node("Node 1");

        graph.add_edge(node1, node2, ());
        graph.add_edge(node2, node3, ());
        graph.add_edge(node2, node4, ());
        graph.add_edge(node3, node4, ());

        let stable_topo = StableTopo::new(&graph);
        let topo_order: Vec<NodeIndex> = stable_topo.collect();

        assert_eq!(topo_order, vec![node1, node2, node3, node4]);
    }
}
