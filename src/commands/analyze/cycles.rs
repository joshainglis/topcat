//! Cycle detection analysis.
//!
//! Detects and displays circular dependencies in the dependency graph.

use std::path::PathBuf;

use comfy_table::Cell;

use topcat::exceptions::TopCatError;

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
/// * `sql_config_file` - Optional path to SQL config file
/// * `enable_sql_discovery` - Whether SQL discovery is enabled
/// * `schema_pattern` - Optional schema pattern for SQL discovery
/// * `merge_strategy` - Merge strategy for SQL discovery
/// * `input_dirs` - Input directories to scan
/// * `include_file_extensions` - Extensions to include
/// * `exclude_file_extensions` - Extensions to exclude
/// * `include_globs` - Glob patterns to include
/// * `exclude_globs` - Glob patterns to exclude
/// * `include_hidden` - Whether to include hidden files
/// * `verbose` - Whether to enable verbose logging
/// * `comment_str` - Comment prefix string
/// * `layers` - Optional layer configuration
/// * `fallback_layer` - Optional fallback layer
/// * `schema_filter` - Optional schema filter
///
/// # Returns
///
/// - `Ok(())` if no cycles are detected (valid DAG)
/// - `Err(TopCatError::CyclicDependency)` if cycles exist
/// - `Err(TopCatError)` for other errors during graph building
#[allow(clippy::too_many_arguments)]
pub fn analyze(
    quiet: bool,
    sql_config_file: &Option<PathBuf>,
    enable_sql_discovery: bool,
    schema_pattern: &Option<String>,
    merge_strategy: &str,
    input_dirs: Vec<PathBuf>,
    include_file_extensions: Option<&[String]>,
    exclude_file_extensions: Option<&[String]>,
    include_globs: Option<&[String]>,
    exclude_globs: Option<&[String]>,
    include_hidden: bool,
    verbose: bool,
    comment_str: String,
    layers: &Option<String>,
    fallback_layer: &Option<String>,
    schema_filter: &[String],
) -> Result<(), TopCatError> {
    let logger = AnalysisLogger::new(quiet);
    logger.section("🔄 Cycle Detection Analysis");

    // Try to build the graph - if it has cycles, it will return a CyclicDependency error
    let sql_discovery = cmd_common::load_sql_discovery_config(
        sql_config_file,
        enable_sql_discovery,
        schema_pattern,
        merge_strategy,
    )?;
    let (layers_parsed, fallback_layer_parsed) =
        cmd_common::parse_and_validate_layers(layers, fallback_layer)?;
    let include_node_prefixes = cmd_common::build_schema_filter_prefixes(schema_filter);

    let result = cmd_common::build_graph(
        input_dirs,
        include_file_extensions,
        exclude_file_extensions,
        include_globs,
        exclude_globs,
        include_hidden,
        verbose,
        comment_str,
        layers_parsed,
        fallback_layer_parsed,
        sql_discovery,
        include_node_prefixes,
    );

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
