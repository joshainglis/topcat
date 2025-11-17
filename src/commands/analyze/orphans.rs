//! Orphan files analysis.
//!
//! Finds files with no dependencies or dependents (completely isolated nodes).

use comfy_table::Cell;

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

use super::common::{AnalysisDisplayConfig, analyze_and_display};

/// Find and display orphan files (nodes with no dependencies or dependents).
///
/// Orphans are completely isolated nodes that neither depend on other files
/// nor are depended upon by other files. They may be safe to delete or could
/// be undocumented entry points.
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
        title: "🔍 Orphan Files Analysis".to_string(),
        empty_message: "✅ No orphaned files found".to_string(),
        result_summary: "📊 Found {} orphaned file(s) with no connections:".to_string(),
        table_headers: vec!["Node Name".to_string(), "File Path".to_string()],
        footer_message: Some(
            "\n💡 These files might be safe to delete or could be entry points".to_string(),
        ),
        apply_external_filter: true,
    };

    analyze_and_display(
        logger,
        graph,
        external_checker,
        config,
        |g| g.find_orphans(),
        |node| vec![Cell::new(&node.name), Cell::new(node.path.display())],
    )
}
