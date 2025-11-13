//! Dead branches analysis.
//!
//! Finds complete dead branches (subtrees that can be removed together).
//! Dead branches are unrequired nodes plus all nodes that would become unrequired
//! if the initial unrequired nodes were removed.

use comfy_table::{Cell, Color};

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;

use super::common::AnalysisLogger;
use crate::commands::common as cmd_common;

/// Find and display dead branches (complete subtrees that can be removed together).
///
/// Dead branches are unrequired nodes plus all nodes that would become unrequired
/// if the initial unrequired nodes were removed. Removing them as a group avoids
/// multiple deletion iterations.
///
/// # Arguments
///
/// * `quiet` - Whether to suppress output
/// * `graph` - The dependency graph to analyze
/// * `external_checker` - Optional checker to filter out externally-used nodes
/// * `root_matcher` - Optional matcher to identify entry point nodes
///
/// # Returns
///
/// `Ok(())` on success, `Err(TopCatError)` on error
pub fn analyze(
    quiet: bool,
    graph: &TCGraph,
    external_checker: Option<&ExternalUsageChecker>,
    root_matcher: Option<&RootNodeMatcher>,
) -> Result<(), TopCatError> {
    let logger = AnalysisLogger::new(quiet);
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

    logger.info(&format!(
        "📊 Found {} node(s) in dead branches:\n",
        dead_branches.len()
    ));
    logger.info(&format!("   • Leaf nodes (initial): {}", leaf_nodes.len()));
    logger.info(&format!(
        "   • Additional nodes (pulled in): {}",
        additional_nodes.len()
    ));
    logger.info(&format!(
        "   • Total nodes in dead branches: {}",
        dead_branches.len()
    ));

    if !additional_nodes.is_empty() {
        logger.info(&format!(
            "\n💡 Benefit: Trimming avoids {} additional deletion iteration(s)",
            additional_nodes.len()
        ));
    }

    // Create a table for better formatting
    let mut table = comfy_table::Table::new();
    table.set_header(vec!["Node Name", "Type", "File Path"]);

    // Sort for consistent output
    let mut sorted_branches: Vec<_> = dead_branches.iter().collect();
    sorted_branches.sort();

    // Build node map once for O(1) lookups
    let all_nodes = graph.get_all_nodes();
    let node_map = cmd_common::build_node_map(&all_nodes);

    for node_name in sorted_branches {
        if let Some(&node) = node_map.get(node_name.as_str()) {
            let node_type = if leaf_nodes.contains(node_name) {
                "leaf 🍃"
            } else {
                "branch 🌿"
            };

            table.add_row(vec![
                Cell::new(&node.name),
                Cell::new(node_type).fg(if leaf_nodes.contains(node_name) {
                    Color::Green
                } else {
                    Color::Yellow
                }),
                Cell::new(node.path.display()),
            ]);
        }
    }

    logger.info("");
    logger.table(&table);

    logger.info(&format!(
        "\n💡 These {} files can all be deleted together in one operation",
        dead_branches.len()
    ));
    logger.info("   Use 'topcat clean dead-branches' to remove them (coming in Phase 3)");

    Ok(())
}
