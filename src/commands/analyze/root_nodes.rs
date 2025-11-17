//! Root nodes analysis.
//!
//! Finds nodes with dependents but no dependencies (entry points in the graph).

use comfy_table::Cell;

use topcat::analysis::GraphAnalyzer;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

use super::common::{AnalysisDisplayConfig, analyze_and_display};

/// Find and display root nodes (nodes with dependents but no dependencies).
///
/// Root nodes have no dependencies but are depended upon by other files.
/// They represent entry points or foundational components in the dependency graph.
///
/// # Arguments
///
/// * `logger` - Logger instance for output
/// * `graph` - The dependency graph to analyze
///
/// # Returns
///
/// `Ok(())` on success, `Err(TopCatError)` on error
pub fn analyze(logger: &Logger, graph: &TCGraph) -> Result<(), TopCatError> {
    let config = AnalysisDisplayConfig {
        title: "🌱 Root Nodes Analysis".to_string(),
        empty_message: "✅ No root nodes found".to_string(),
        result_summary: "📊 Found {} root node(s) (have dependents but no dependencies):"
            .to_string(),
        table_headers: vec!["Node Name".to_string(), "File Path".to_string()],
        footer_message: Some(
            "\n💡 These files are entry points in your dependency graph".to_string(),
        ),
        apply_external_filter: false,
    };

    analyze_and_display(
        logger,
        graph,
        None,
        config,
        |g| g.find_root_nodes(),
        |node| vec![Cell::new(&node.name), Cell::new(node.path.display())],
    )
}
