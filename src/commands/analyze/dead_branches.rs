//! Dead branches analysis.
//!
//! Finds complete dead branches (subtrees that can be removed together).
//! Dead branches are unrequired nodes plus all nodes that would become unrequired
//! if the initial unrequired nodes were removed.

use std::collections::HashMap;

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::display_utils::{build_tree_forest, render_forest_unified};
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

use crate::commands::common as cmd_common;

/// Find and display dead branches (complete subtrees that can be removed together).
///
/// Dead branches are unrequired nodes plus all nodes that would become unrequired
/// if the initial unrequired nodes were removed. Removing them as a group avoids
/// multiple deletion iterations.
///
/// # Arguments
///
/// * `logger` - Logger instance for output
/// * `graph` - The dependency graph to analyze
/// * `external_checker` - Optional checker to filter out externally-used nodes
/// * `root_matcher` - Optional matcher to identify entry point nodes
///
/// # Returns
///
/// `Ok(())` on success, `Err(TopCatError)` on error
pub fn analyze(
    logger: &Logger,
    graph: &TCGraph,
    external_checker: Option<&ExternalUsageChecker>,
    root_matcher: Option<&RootNodeMatcher>,
) -> Result<(), TopCatError> {
    logger.section("🌳 Dead Branches Analysis");

    let mut dead_branches = graph.find_dead_branches(root_matcher);
    let leaf_nodes = graph.find_leaf_nodes();

    // Filter by external usage if checker is provided
    if let Some(checker) = external_checker {
        dead_branches = checker.filter_unused(&dead_branches);
    }

    if dead_branches.is_empty() {
        logger.info("✅ No dead branches found (all unrequired files are needed)");
        return Ok(());
    }

    let additional_nodes: Vec<_> = dead_branches.difference(&leaf_nodes).collect();

    // Build necessary data structures for tree display
    let all_nodes = graph.get_all_nodes();
    let node_map = cmd_common::build_node_map(&all_nodes);

    // Create deps map (node -> its dependencies)
    let mut deps_map = HashMap::new();
    for node_name in &dead_branches {
        if let Some(&node) = node_map.get(node_name.as_str()) {
            deps_map.insert(node_name.clone(), node.deps.clone());
        }
    }

    // Create dependents map (node -> nodes that depend on it)
    let dependents_map = graph.build_dependents_map();

    // Create node paths map
    let mut node_paths = HashMap::new();
    for node_name in &dead_branches {
        if let Some(&node) = node_map.get(node_name.as_str()) {
            node_paths.insert(node_name.clone(), node.path.clone());
        }
    }

    // Build the forest with stable ordering (tree building now uses stable sort internally)
    let forest = build_tree_forest(&dead_branches, &deps_map, &dependents_map, &node_paths);

    logger.info(&format!(
        "📊 Found {} node(s) in {} disconnected tree(s):\n",
        dead_branches.len(),
        forest.len()
    ));
    logger.info(&format!("   • Initial leaf nodes: {}", leaf_nodes.len()));
    logger.info(&format!(
        "   • Additional nodes pulled in: {}",
        additional_nodes.len()
    ));
    logger.info(&format!("   • Disconnected trees: {}", forest.len()));

    if !additional_nodes.is_empty() {
        logger.info(&format!(
            "\n💡 Benefit: Trimming avoids {} additional deletion iteration(s)",
            additional_nodes.len()
        ));
    }

    // Render and display the forest
    if !forest.is_empty() {
        logger.info("");
        let tree_output = render_forest_unified(&forest);
        logger.info(&tree_output);
    }

    logger.info(&format!(
        "\n💡 These {} files can all be deleted together in one operation",
        dead_branches.len()
    ));
    logger.info("   Use 'topcat clean dead-branches' to remove them");

    Ok(())
}
