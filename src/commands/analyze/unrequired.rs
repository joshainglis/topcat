//! Unrequired files analysis.
//!
//! Finds files that are not required by any other files (nodes with no dependents).

use comfy_table::Cell;

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

use super::common::{AnalysisDisplayConfig, analyze_and_display};

/// Find and display unrequired files (nodes not needed by any other files).
///
/// Unrequired nodes have no dependents, meaning no other files in the graph
/// require them. They may still have dependencies themselves (unlike orphans).
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
        title: "🧹 Unrequired Files Analysis".to_string(),
        empty_message: "✅ All files are required by at least one other file".to_string(),
        result_summary: "📊 Found {} unrequired file(s) (not needed by any other files):"
            .to_string(),
        table_headers: vec![
            "Node Name".to_string(),
            "Has Dependencies".to_string(),
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
        |g| g.find_unrequired(),
        |node| {
            vec![
                Cell::new(&node.name),
                Cell::new(if node.deps.is_empty() { "No" } else { "Yes" }),
                Cell::new(node.path.display()),
            ]
        },
    )
}
