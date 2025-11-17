//! Single file detailed analysis.
//!
//! Provides comprehensive information about a specific file in the dependency graph.

use std::path::PathBuf;

use comfy_table::Cell;

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;

/// Perform detailed analysis of a specific file.
///
/// Displays comprehensive information about a single file including:
/// - Node metadata (name, path, layer)
/// - Direct dependencies
/// - Direct dependents
/// - External usage status (if checker provided)
/// - Node classification (orphan, root, leaf, intermediate)
/// - Whether the node is required by others
///
/// # Arguments
///
/// * `logger` - Logger instance for output
/// * `graph` - The dependency graph containing the file
/// * `path` - Path to the file to analyze
/// * `external_checker` - Optional checker for external usage status
///
/// # Returns
///
/// `Ok(())` on success, or `Err(TopCatError::ConfigError)` if the file
/// is not found in the graph.
pub fn analyze(
    logger: &Logger,
    graph: &TCGraph,
    path: &PathBuf,
    external_checker: Option<&ExternalUsageChecker>,
) -> Result<(), TopCatError> {
    logger.section(&format!("📄 File Analysis: {}", path.display()));

    // Find the node in the graph
    let all_nodes = graph.get_all_nodes();
    let target_node = all_nodes.iter().find(|n| n.path == *path).ok_or_else(|| {
        TopCatError::ConfigError(format!("File not found in graph: {}", path.display()))
    })?;

    logger.info(&format!("Node Name: {}", target_node.name));
    logger.info(&format!("File Path: {}", target_node.path.display()));
    logger.info(&format!("Layer: {}", target_node.layer));
    logger.newline();

    // Get dependencies
    logger.separator();
    logger.info(&format!(
        "Direct Dependencies ({}):",
        target_node.deps.len()
    ));
    logger.separator();
    logger.newline();

    if target_node.deps.is_empty() {
        logger.info("  (none)\n");
    } else {
        let mut table = comfy_table::Table::new();
        table.set_header(vec!["Dependency Name"]);
        for dep in &target_node.deps {
            table.add_row(vec![Cell::new(dep)]);
        }
        logger.table(&table);
        logger.newline();
    }

    // Get dependents
    let dependents_map = graph.build_dependents_map();
    let direct_dependents = dependents_map
        .get(&target_node.name)
        .cloned()
        .unwrap_or_default();

    logger.separator();
    logger.info(&format!("Direct Dependents ({}):", direct_dependents.len()));
    logger.separator();
    logger.newline();

    if direct_dependents.is_empty() {
        logger.info("  (none)\n");
    } else {
        let mut table = comfy_table::Table::new();
        table.set_header(vec!["Dependent Name"]);
        for dep in &direct_dependents {
            table.add_row(vec![Cell::new(dep)]);
        }
        logger.table(&table);
        logger.newline();
    }

    // Check external usage if available
    if let Some(checker) = external_checker {
        let mut single_node_set = std::collections::HashSet::new();
        single_node_set.insert(target_node.name.clone());
        let externally_used = checker.filter_unused(&single_node_set);

        logger.separator();
        logger.info("External Usage:");
        logger.separator();
        logger.newline();

        if externally_used.is_empty() {
            logger.info("  ❌ Not used by external files\n");
        } else {
            logger.info("  ✅ Used by external files\n");
        }
    }

    // Analysis summary
    logger.separator();
    logger.info("Summary:");
    logger.separator();
    logger.newline();

    // Determine node type
    let node_type = if target_node.deps.is_empty() && direct_dependents.is_empty() {
        "Orphan (no connections)"
    } else if target_node.deps.is_empty() {
        "Root Node (entry point)"
    } else if direct_dependents.is_empty() {
        "Leaf Node (terminal)"
    } else {
        "Intermediate Node"
    };

    logger.info(&format!("  Node Type: {node_type}"));

    // Check if in dead branches
    let unrequired = graph.find_unrequired();
    if unrequired.contains(&target_node.name) {
        logger.info("  ⚠️  Part of unrequired files (not used by others)");
    } else {
        logger.info("  ✅ Required by other files");
    }

    Ok(())
}
