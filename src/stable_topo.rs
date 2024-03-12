use std::collections::HashSet;

use petgraph::data::DataMap;
use petgraph::graph::NodeIndex;
use petgraph::visit::IntoNeighborsDirected;
use petgraph::visit::Reversed;
use petgraph::visit::{GraphBase, IntoNeighbors, IntoNodeIdentifiers, Visitable};
use petgraph::Incoming;

/// `StableTopo` represents a stable topological sort of a directed graph.
/// It is implemented using a depth-first search (DFS) algorithm.
///
/// # Examples
///
/// ```
/// use std::collections::HashSet;
/// use petgraph::graph::{Graph, NodeIndex};
/// use petgraph::stable_topo::StableTopo;
///
/// // Create a new graph
/// let mut graph = Graph::<i32, ()>::new();
///
/// // Add nodes to the graph
/// let n1 = graph.add_node(1);
/// let n2 = graph.add_node(2);
/// let n3 = graph.add_node(3);
/// let n4 = graph.add_node(4);
///
/// // Add edges to the graph
/// graph.add_edge(n1, n2, ());
/// graph.add_edge(n2, n3, ());
/// graph.add_edge(n2, n4, ());
/// graph.add_edge(n4, n3, ());
///
/// // Perform a stable topological sort
/// let stable_topo = StableTopo::new(&graph);
///
/// // Get the ordered nodes
/// let ordered_nodes = stable_topo.ordered();
///
/// assert_eq!(ordered_nodes, vec![n1, n2, n4, n3]);
/// ```
///
/// # Implementation Details
///
/// The `StableTopo` struct has the following fields:
/// - `graph`: The directed graph.
/// - `ordered`: A set containing the nodes in the order they were visited during the DFS.
/// - `tovisit : A stack containing the nodes to visit during the DFS.
///
/// The `StableTopo` struct implements the `Clone` trait to allow for creating clones of the struct
/// with an independent state.
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
    pub fn new(graph: G) -> Self {
        let mut topo = StableTopo {
            graph,
            ordered: HashSet::new(),
            tovisit: Vec::new(),
        };
        topo.extend_with_initials();
        topo
    }

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
        self.tovisit.sort_unstable_by(|a, b| {
            match self.graph.node_weight(*a) {
                Some(x) => x,
                None => panic!("Node not found in graph: {:?}", a),
            }
            .cmp(match self.graph.node_weight(*b) {
                Some(x) => x,
                None => panic!("Node not found in graph: {:?}", b),
            })
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
            // Sort the neighbors based on the node index
            neighbors.sort_unstable_by(|a, b| {
                match self.graph.node_weight(*a) {
                    Some(x) => x,
                    None => panic!("Node not found in graph: {:?}", a),
                }
                .cmp(match self.graph.node_weight(*b) {
                    Some(x) => x,
                    None => panic!("Node not found in graph: {:?}", b),
                })
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
