//! Missing dependencies analysis.
//!
//! Finds and displays referenced dependencies that don't exist in the graph.

use std::path::PathBuf;

use comfy_table::{Cell, Color};

use topcat::config;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;
use topcat::settings::Settings;
use topcat::sql_config;

use crate::commands::common as cmd_common;

/// Find and display missing dependencies (referenced but non-existent files).
///
/// Scans all files and collects ALL missing dependencies in a single pass,
/// allowing users to fix all issues at once instead of iteratively.
///
/// # Arguments
///
/// * `logger` - Logger instance for output
/// * `schemas` - Optional schema filter from CLI (overrides settings)
/// * `settings` - Configuration settings
///
/// # Returns
///
/// - `Ok(())` if no missing dependencies are found
/// - `Err(TopCatError::MissingDependency)` if any are found (with full list)
/// - `Err(TopCatError)` for other errors during scanning
pub fn analyze(
    logger: &Logger,
    schemas: &Option<Vec<String>>,
    settings: &Settings,
) -> Result<(), TopCatError> {
    logger.section("🔍 Missing Dependencies Analysis");

    // Build config for validation - we need the old Config struct for TCGraph::new()
    // Extract schema filter for node prefixes
    let schema_filter: Vec<String> = schemas
        .clone()
        .unwrap_or_else(|| settings.schema_filtering.schemas.clone());

    let include_node_prefixes = cmd_common::build_schema_filter(&schema_filter).to_option();

    // Convert Settings to Config for TCGraph::new()
    let config = config::Config {
        input_dirs: settings.input_dirs.clone(),
        include_extensions: if settings.filters.include_extensions.is_empty() {
            None
        } else {
            Some(&settings.filters.include_extensions)
        },
        exclude_extensions: if settings.filters.exclude_extensions.is_empty() {
            None
        } else {
            Some(&settings.filters.exclude_extensions)
        },
        include_globs: if settings.filters.include_globs.is_empty() {
            None
        } else {
            Some(&settings.filters.include_globs)
        },
        exclude_globs: if settings.filters.exclude_globs.is_empty() {
            None
        } else {
            Some(&settings.filters.exclude_globs)
        },
        output: PathBuf::from(cmd_common::null_device()),
        comment_str: settings.formatting.comment_str.clone(),
        file_separator_str: String::new(),
        file_end_str: String::new(),
        include_hidden: settings.filters.include_hidden,
        verbose: settings.behavior.verbose,
        include_node_prefixes: include_node_prefixes.as_deref(),
        exclude_node_prefixes: None,
        dry_run: false,
        subdir_filter: None,
        layers: settings.layers.names.clone(),
        fallback_layer: settings
            .layers
            .fallback
            .clone()
            .unwrap_or_else(|| "normal".to_string()),
        sql_discovery: settings.sql_discovery.clone(),
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
