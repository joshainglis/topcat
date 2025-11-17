//! Missing dependencies analysis.
//!
//! Finds and displays referenced dependencies that don't exist in the graph.

use std::path::PathBuf;

use comfy_table::{Cell, Color};

use topcat::config;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::sql_config;

use super::common::AnalysisLogger;
use crate::commands::common as cmd_common;

/// Find and display missing dependencies (referenced but non-existent files).
///
/// Scans all files and collects ALL missing dependencies in a single pass,
/// allowing users to fix all issues at once instead of iteratively.
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
/// - `Ok(())` if no missing dependencies are found
/// - `Err(TopCatError::MissingDependency)` if any are found (with full list)
/// - `Err(TopCatError)` for other errors during scanning
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
    logger.section("🔍 Missing Dependencies Analysis");

    // Build a minimal config for validation (same as build_graph but for validation only)
    let sql_discovery = cmd_common::load_sql_discovery_config(
        sql_config_file,
        enable_sql_discovery,
        schema_pattern,
        merge_strategy,
    )?;
    let (layers_parsed, fallback_layer_parsed) =
        cmd_common::parse_and_validate_layers(sql_config_file, layers, fallback_layer)?;
    let include_node_prefixes = cmd_common::build_schema_filter(schema_filter).to_option();

    let config = config::Config {
        input_dirs,
        include_extensions: include_file_extensions,
        exclude_extensions: exclude_file_extensions,
        include_globs,
        exclude_globs,
        output: PathBuf::from(cmd_common::null_device()),
        comment_str,
        file_separator_str: String::new(),
        file_end_str: String::new(),
        include_hidden,
        verbose,
        include_node_prefixes: include_node_prefixes.as_deref(),
        exclude_node_prefixes: None,
        dry_run: false,
        subdir_filter: None,
        layers: layers_parsed,
        fallback_layer: fallback_layer_parsed,
        sql_discovery,
        header_update_mode: sql_config::HeaderUpdateMode::Never,
        header_output_dir: None,
    };

    let mut graph = TCGraph::new(&config);

    // Use the new validate_dependencies_only method to get ALL missing dependencies
    let missing_deps = graph.validate_dependencies_only()?;

    if missing_deps.is_empty() {
        logger.info("✅ No missing dependencies found");
        logger.info("   All referenced dependencies exist in the graph");
        return Ok(());
    }

    logger.info(&format!(
        "⚠️  Found {} missing dependencies:\n",
        missing_deps.len()
    ));

    let mut table = comfy_table::Table::new();
    table.set_header(vec!["File", "Missing Dependency"]);

    // Sort for deterministic output
    let mut sorted_deps = missing_deps.clone();
    sorted_deps.sort();

    for (file, dep) in &sorted_deps {
        table.add_row(vec![
            Cell::new(file).fg(Color::Yellow),
            Cell::new(dep).fg(Color::Red),
        ]);
    }

    logger.table(&table);
    logger.newline();
    logger.info("💡 These files reference dependencies that don't exist:");
    logger.info("   1. Check if the dependency file name is spelled correctly");
    logger.info("   2. Verify the dependency file is in the input directory");
    logger.info("   3. Consider using 'exists' instead of 'requires' if optional");

    // Return error for scripting (exit code 1)
    Err(TopCatError::MissingDependency(
        format!("{} files with missing dependencies", sorted_deps.len()),
        format!("{} total missing", sorted_deps.len()),
    ))
}
