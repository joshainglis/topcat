//! Clean orphan files.
//!
//! Removes files with no dependencies or dependents (completely isolated nodes).

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

use super::common;

/// Remove orphan files (nodes with no dependencies or dependents).
///
/// Finds orphans, filters by external usage and root patterns,
/// then deletes the files (unless in dry-run mode).
///
/// # Arguments
///
/// * `logger` - Logger instance for output
/// * `graph` - The dependency graph
/// * `external_checker` - Optional checker to filter out externally-used nodes
/// * `root_matcher` - Optional matcher to identify protected nodes
/// * `protect_implicit` - Whether to protect implicit nodes from cleanup
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
    protect_implicit: bool,
    actually_delete: bool,
    force: bool,
) -> Result<(), TopCatError> {
    logger.progress("\n🌿 Finding orphan files...\n");

    // Find orphans
    let mut orphans = graph.find_orphans();

    // Filter out implicit nodes if protection is enabled
    if protect_implicit {
        orphans = graph.filter_non_implicit(&orphans);
    }

    // Filter out root nodes if specified
    orphans = common::filter_by_root_matcher(logger, orphans, graph, root_matcher);

    // Filter out externally used files
    orphans = common::apply_external_filter(logger, orphans, external_checker);

    if orphans.is_empty() {
        logger.success("✅ No orphan files found!");
        return Ok(());
    }

    // Show what will be deleted
    common::show_deletion_preview(logger, graph, &orphans, "Orphan Files")?;

    // Delete files if not dry-run
    if actually_delete {
        common::perform_deletion(logger, graph, &orphans, force)?;
    } else {
        logger.info("\n💡 Run with --mode execute to actually delete these files");
    }

    Ok(())
}
