//! Clean dead branches.
//!
//! Removes complete dead branches (subtrees that can be trimmed together).

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

use super::common;

/// Remove complete dead branches (subtrees that can be trimmed together).
///
/// Finds dead branches, filters by external usage and root patterns,
/// then deletes the files (unless in dry-run mode).
///
/// # Arguments
///
/// * `logger` - Logger instance for output
/// * `graph` - The dependency graph
/// * `external_checker` - Optional checker to filter out externally-used nodes
/// * `root_matcher` - Optional matcher to identify protected nodes
/// * `actually_delete` - Whether to actually delete files (false = dry-run)
/// * `force` - Skip confirmation prompt if true
///
/// # Returns
///
/// `Ok(())` on success, `Err(TopCatError)` on error
pub fn clean(
    logger: &Logger,
    graph: &TCGraph,
    external_checker: Option<&ExternalUsageChecker>,
    root_matcher: Option<&RootNodeMatcher>,
    actually_delete: bool,
    force: bool,
) -> Result<(), TopCatError> {
    logger.progress("\n🌳 Finding dead branches...\n");

    // Find dead branches
    let dead_branches = graph.find_dead_branches(root_matcher);

    // Filter out externally used files
    let dead_branches = common::apply_external_filter(dead_branches, external_checker);

    if dead_branches.is_empty() {
        logger.success("✅ No dead branches found! Your codebase is clean.");
        return Ok(());
    }

    // Show what will be deleted
    common::show_deletion_preview(logger, graph, &dead_branches, "Dead Branches")?;

    // Delete files if not dry-run
    if actually_delete {
        common::perform_deletion(logger, graph, &dead_branches, force)?;
    } else {
        logger.info("\n💡 Run with --no-dry-run to actually delete these files");
    }

    Ok(())
}
