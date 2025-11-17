//! Leaf nodes analysis.
//!
//! Finds nodes with dependencies but no dependents (terminal nodes in the graph).

use comfy_table::Cell;

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

use super::common::{AnalysisDisplayConfig, analyze_and_display};

/// Find and display leaf nodes (nodes with dependencies but no dependents).
///
/// Leaf nodes depend on other files but are not depended upon by any files.
/// They represent terminal nodes in the dependency graph.
///
/// # Arguments
///
/// * `logger` - Logger instance for output
/// * `graph` - The dependency graph to analyze
/// * `external_checker` - Optional checker to filter out externally-used nodes
///
/// # Returns
///
/// `Ok(())` on success, `Err(TopCatError)` on error
pub fn analyze(
    logger: &Logger,
    graph: &TCGraph,
    external_checker: Option<&ExternalUsageChecker>,
) -> Result<(), TopCatError> {
    let config = AnalysisDisplayConfig {
        title: "🍃 Leaf Nodes Analysis".to_string(),
        empty_message: "✅ No leaf nodes found".to_string(),
        result_summary: "📊 Found {} leaf node(s) (have dependencies but no dependents):"
            .to_string(),
        table_headers: vec![
            "Node Name".to_string(),
            "Dependencies Count".to_string(),
            "File Path".to_string(),
        ],
        footer_message: None,
        apply_external_filter: true,
    };

    analyze_and_display(
        logger,
        graph,
        external_checker,
        config,
        |g| g.find_leaf_nodes(),
        |node| {
            vec![
                Cell::new(&node.name),
                Cell::new(node.deps.len()),
                Cell::new(node.path.display()),
            ]
        },
    )
}
