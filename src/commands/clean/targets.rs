//! Clean specific target files.
//!
//! Removes specific files by pattern, with dependency checking to ensure safety.

use std::collections::HashSet;

use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

use super::common;

/// Remove specific target files (with dependency checking).
///
/// Matches patterns to node names or file paths, checks for dependents,
/// filters by root patterns, then deletes the files (unless in dry-run mode).
///
/// # Arguments
///
/// * `logger` - Logger instance for output
/// * `graph` - The dependency graph
/// * `target_files` - File patterns or names to remove
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
    target_files: &[String],
    root_matcher: Option<&RootNodeMatcher>,
    actually_delete: bool,
    force: bool,
) -> Result<(), TopCatError> {
    logger.progress("\n🎯 Processing target files for deletion...\n");

    if target_files.is_empty() {
        return Err(TopCatError::ConfigError(
            "No target files specified".to_string(),
        ));
    }

    // Build a map of node names to check
    let node_to_path = graph.build_node_to_path_map();
    let mut targets_to_delete = HashSet::new();

    // Match target patterns to actual nodes
    for pattern in target_files {
        let mut found_match = false;
        for (node_name, path) in &node_to_path {
            // Check if the pattern matches the node name or file path
            if node_name.contains(pattern) || path.to_string_lossy().contains(pattern) {
                targets_to_delete.insert(node_name.clone());
                found_match = true;
            }
        }
        if !found_match {
            logger.warn(&format!(
                "⚠️  Warning: No files matched pattern '{pattern}'"
            ));
        }
    }

    if targets_to_delete.is_empty() {
        logger.error("❌ No files matched the specified patterns");
        return Ok(());
    }

    // Filter out root nodes if specified
    targets_to_delete = common::filter_by_root_matcher(targets_to_delete, graph, root_matcher);

    // Check for dependents that would break
    let dependents_map = graph.build_dependents_map();
    let mut has_dependents = false;
    let mut dependent_warnings = Vec::new();

    for target in &targets_to_delete {
        if let Some(dependents) = dependents_map.get(target)
            && !dependents.is_empty()
        {
            has_dependents = true;
            dependent_warnings.push((target.clone(), dependents.clone()));
        }
    }

    if has_dependents {
        logger.warn("⚠️  WARNING: Some target files have dependents!\n");
        for (target, dependents) in &dependent_warnings {
            logger.info(&format!("  {target} is required by:"));
            for dep in dependents {
                logger.info(&format!("    - {dep}"));
            }
            logger.newline();
        }
        logger.error("❌ Cannot delete files that are still required by others");
        logger.info("💡 Tip: Use 'clean dead-branches' to remove entire unused subtrees\n");
        return Ok(());
    }

    // Show what will be deleted
    common::show_deletion_preview(logger, graph, &targets_to_delete, "Target Files")?;

    // Delete files if not dry-run
    if actually_delete {
        common::perform_deletion(logger, graph, &targets_to_delete, force)?;
    } else {
        logger.info("\n💡 Run with --no-dry-run to actually delete these files");
    }

    Ok(())
}
