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
/// * `protect_implicit` - Whether to protect implicit nodes from being marked as orphans
///
/// # Returns
///
/// `Ok(())` on success, `Err(TopCatError)` on error
pub fn analyze(
    logger: &Logger,
    graph: &TCGraph,
    external_checker: Option<&ExternalUsageChecker>,
    protect_implicit: bool,
) -> Result<(), TopCatError> {
    logger.section("🔍 Orphan Files Analysis");

    let mut orphans = graph.find_orphans();

    // Filter out implicit nodes if protection is enabled
    if protect_implicit {
        let implicit_orphans: Vec<_> = orphans
            .iter()
            .filter(|name| graph.get_node(name).map(|n| n.implicit).unwrap_or(false))
            .cloned()
            .collect();

        if !implicit_orphans.is_empty() {
            let mut sorted_implicit = implicit_orphans.clone();
            sorted_implicit.sort();
            logger.info(&format!(
                "ℹ️  Protected {} implicit node(s): {}",
                sorted_implicit.len(),
                sorted_implicit.join(", ")
            ));
        }

        orphans = graph.filter_non_implicit(&orphans);
    }

    // Apply external filtering if requested
    if let Some(checker) = external_checker {
        orphans = checker.filter_unused(&orphans);
    }

    if orphans.is_empty() {
        logger.info("✅ No orphaned files found");
        return Ok(());
    }

    let config = AnalysisDisplayConfig {
        title: "".to_string(),         // Already printed above
        empty_message: "".to_string(), // Already handled above
        result_summary: "📊 Found {} orphaned file(s) with no connections:".to_string(),
        table_headers: vec!["Node Name".to_string(), "File Path".to_string()],
        footer_message: Some(
            "\n💡 These files might be safe to delete or could be entry points".to_string(),
        ),
        apply_external_filter: false, // Already applied above
    };

    // Use analyze_and_display but with custom orphans set
    analyze_and_display(
        logger,
        graph,
        None, // Already applied external filtering
        config,
        |_| orphans.clone(),
        |node| vec![Cell::new(&node.name), Cell::new(node.path.display())],
    )
}
