//! Clean unrequired files.
//!
//! Removes files that are not required by any other files (nodes with no dependents).

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

use super::common;

/// Remove unrequired files (nodes not needed by any other files).
///
/// Finds unrequired files, filters by external usage and root patterns,
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
    logger.progress("\n🍃 Finding unrequired files...\n");

    // Find unrequired
    let mut unrequired = graph.find_unrequired();

    // Filter out implicit nodes if protection is enabled
    if protect_implicit {
        unrequired = graph.filter_non_implicit(&unrequired);
    }

    // Filter out root nodes if specified
    unrequired = common::filter_by_root_matcher(unrequired, graph, root_matcher);

    // Filter out externally used files
    unrequired = common::apply_external_filter(unrequired, external_checker);

    if unrequired.is_empty() {
        logger.success("✅ No unrequired files found!");
        return Ok(());
    }

    // Show what will be deleted
    common::show_deletion_preview(logger, graph, &unrequired, "Unrequired Files")?;

    // Delete files if not dry-run
    if actually_delete {
        common::perform_deletion(logger, graph, &unrequired, force)?;
    } else {
        logger.info("\n💡 Run with --no-dry-run to actually delete these files");
    }

    Ok(())
}
