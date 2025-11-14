//! Clean orphan files.
//!
//! Removes files with no dependencies or dependents (completely isolated nodes).

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;

use super::common;

/// Remove orphan files (nodes with no dependencies or dependents).
///
/// Finds orphans, filters by external usage and root patterns,
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
    println!("\n🌿 Finding orphan files...\n");

    // Find orphans
    let mut orphans = graph.find_orphans();

    // Filter out root nodes if specified
    orphans = common::filter_by_root_matcher(orphans, graph, root_matcher);

    // Filter out externally used files
    orphans = common::apply_external_filter(orphans, external_checker);

    if orphans.is_empty() {
        println!("✅ No orphan files found!");
        return Ok(());
    }

    // Show what will be deleted
    common::show_deletion_preview(graph, &orphans, "Orphan Files")?;

    // Delete files if not dry-run
    if actually_delete {
        common::perform_deletion(graph, &orphans, force, verbose)?;
    } else {
        println!("\n💡 Run with --no-dry-run to actually delete these files");
    }

    Ok(())
}
