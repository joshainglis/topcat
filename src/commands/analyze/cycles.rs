//! Cycle detection analysis.
//!
//! Detects and displays circular dependencies in the dependency graph.

use comfy_table::Cell;

use topcat::exceptions::TopCatError;
use topcat::settings::Settings;

use super::common::AnalysisLogger;
use crate::commands::common as cmd_common;

/// Detect and display cycles in the dependency graph.
///
/// Attempts to build the graph, which will fail if cycles exist. Displays
/// detailed information about each cycle found including participants and
/// the circular dependency path.
///
/// # Arguments
///
/// * `quiet` - Whether to suppress output
/// * `schemas` - Optional schema filter from CLI (overrides settings)
/// * `settings` - Configuration settings
///
/// # Returns
///
/// - `Ok(())` if no cycles are detected (valid DAG)
/// - `Err(TopCatError::CyclicDependency)` if cycles exist
/// - `Err(TopCatError)` for other errors during graph building
pub fn analyze(
    quiet: bool,
    schemas: &Option<Vec<String>>,
    settings: &Settings,
) -> Result<(), TopCatError> {
    let logger = AnalysisLogger::new(quiet);
    logger.section("🔄 Cycle Detection Analysis");

    // Try to build the graph - if it has cycles, it will return a CyclicDependency error
    let result = cmd_common::build_graph_from_settings(schemas, settings);

    match result {
        Ok(_) => {
            logger.info("✅ No cycles detected in the dependency graph");
            logger.info("   The graph is a valid DAG (Directed Acyclic Graph)");
            Ok(())
        }
        Err(TopCatError::CyclicDependency(cycles)) => {
            logger.info(&format!(
                "⚠️  Found {} cycle(s) in the dependency graph:\n",
                cycles.len()
            ));

            for (i, cycle) in cycles.iter().enumerate() {
                logger.separator();
                logger.info(&format!("Cycle #{}", i + 1));
                logger.separator();
                logger.newline();

                // Show participants
                logger.info("Participants:");
                let mut table = comfy_table::Table::new();
                table.set_header(vec!["Node Name", "File Path"]);

                // Deduplicate participants
                let mut seen = std::collections::HashSet::new();
                for node in cycle {
                    if seen.insert(&node.name) {
                        table.add_row(vec![Cell::new(&node.name), Cell::new(node.path.display())]);
                    }
                }
                logger.table(&table);
                logger.newline();

                // Show cycle edges
                logger.info("Cycle Path:");
                for (j, node) in cycle.iter().enumerate() {
                    let next_node = &cycle[(j + 1) % cycle.len()];
                    logger.info(&format!("  {} → {}", node.name, next_node.name));
                }
                logger.newline();
            }

            logger.separator();
            logger.newline();
            logger.info("💡 How to fix cycles:");
            logger.info("   1. Remove one of the dependencies in the cycle");
            logger.info("   2. Use 'exists' instead of 'requires' for soft dependencies");
            logger.info("   3. Restructure code to break circular dependencies");
            logger.info("   4. Use layers to enforce ordering between groups");

            // Return error with exit code 1 for scripting
            Err(TopCatError::CyclicDependency(cycles))
        }
        Err(e) => {
            // Other errors
            Err(e)
        }
    }
}
