//! Clean unrequired files.
//!
//! Removes files that are not required by any other files (nodes with no dependents).

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;

use super::common;

/// Remove unrequired files (nodes not needed by any other files).
///
/// Finds unrequired files, filters by external usage and root patterns,
/// then deletes the files (unless in dry-run mode).
///
/// # Arguments
///
/// * `graph` - The dependency graph
/// * `external_checker` - Optional checker to filter out externally-used nodes
/// * `root_matcher` - Optional matcher to identify protected nodes
/// * `actually_delete` - Whether to actually delete files (false = dry-run)
/// * `force` - Skip confirmation prompt if true
/// * `verbose` - Show each file as it's deleted
///
/// # Returns
///
/// `Ok(())` on success, `Err(TopCatError)` on error
pub fn clean(
    graph: &TCGraph,
    external_checker: Option<&ExternalUsageChecker>,
    root_matcher: Option<&RootNodeMatcher>,
    actually_delete: bool,
    force: bool,
    verbose: bool,
) -> Result<(), TopCatError> {
    println!("\n🍃 Finding unrequired files...\n");

    // Find unrequired
    let mut unrequired = graph.find_unrequired();

    // Filter out root nodes if specified
    unrequired = common::filter_by_root_matcher(unrequired, graph, root_matcher);

    // Filter out externally used files
    unrequired = common::apply_external_filter(unrequired, external_checker);

    if unrequired.is_empty() {
        println!("✅ No unrequired files found!");
        return Ok(());
    }

    // Show what will be deleted
    common::show_deletion_preview(graph, &unrequired, "Unrequired Files")?;

    // Delete files if not dry-run
    if actually_delete {
        common::perform_deletion(graph, &unrequired, force, verbose)?;
    } else {
        println!("\n💡 Run with --no-dry-run to actually delete these files");
    }

    Ok(())
}
