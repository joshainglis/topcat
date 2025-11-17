//! Common utilities for analysis commands.
//!
//! This module provides shared functionality used across different analysis types:
//! - `AnalysisDisplayConfig`: Configuration for formatted table output
//! - `analyze_and_display`: Generic analysis function pattern

use std::collections::HashSet;

use comfy_table::{Cell, Table};

use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::file_node::FileNode;
use topcat::logging::Logger;

use crate::commands::common as cmd_common;

/// Configuration for displaying analysis results in a table format.
///
/// This struct standardizes the display format across different analysis types,
/// enabling the generic `analyze_and_display()` function to handle multiple
/// analysis commands with consistent formatting.
pub struct AnalysisDisplayConfig {
    /// Section title (e.g., "🔍 Orphan Files Analysis")
    pub title: String,
    /// Message when no results found (e.g., "✅ No orphaned files found")
    pub empty_message: String,
    /// Summary format with placeholder for count (e.g., "📊 Found {} orphaned file(s)")
    pub result_summary: String,
    /// Table column headers
    pub table_headers: Vec<String>,
    /// Optional footer message shown after the table
    pub footer_message: Option<String>,
    /// Whether to apply external checker filtering to results
    pub apply_external_filter: bool,
}

/// Generic analysis function that handles the common pattern across multiple analyses.
///
/// This function eliminates ~70% code duplication by providing a reusable pattern for:
/// 1. Finding nodes based on criteria (via `finder` closure)
/// 2. Optionally filtering by external usage
/// 3. Building result table rows (via `row_builder` closure)
/// 4. Displaying results in a formatted table
///
/// # Type Parameters
///
/// * `F` - Finder function that locates nodes matching analysis criteria
/// * `R` - Row builder function that formats a node into table cells
///
/// # Arguments
///
/// * `logger` - Logger instance for output
/// * `graph` - The dependency graph to analyze
/// * `external_checker` - Optional checker to filter out externally-used nodes
/// * `config` - Display configuration (titles, headers, messages)
/// * `finder` - Closure that finds relevant nodes in the graph
/// * `row_builder` - Closure that builds table row cells for a node
///
/// # Returns
///
/// `Ok(())` on success, `Err(TopCatError)` on error
pub fn analyze_and_display<F, R>(
    logger: &Logger,
    graph: &TCGraph,
    external_checker: Option<&ExternalUsageChecker>,
    config: AnalysisDisplayConfig,
    finder: F,
    row_builder: R,
) -> Result<(), TopCatError>
where
    F: Fn(&TCGraph) -> HashSet<String>,
    R: Fn(&FileNode) -> Vec<Cell>,
{
    logger.section(&config.title);

    let mut results = finder(graph);

    // Apply external filtering if requested
    if config.apply_external_filter
        && let Some(checker) = external_checker
    {
        results = checker.filter_unused(&results);
    }

    if results.is_empty() {
        logger.info(&config.empty_message);
        return Ok(());
    }

    logger.info(&format!(
        "{}\n",
        config
            .result_summary
            .replace("{}", &results.len().to_string())
    ));

    let mut table = Table::new();
    table.set_header(
        config
            .table_headers
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
    );

    // Sort for deterministic output
    let mut sorted_results: Vec<_> = results.iter().collect();
    sorted_results.sort();

    // Build node map once for O(1) lookups
    let all_nodes = graph.get_all_nodes();
    let node_map = cmd_common::build_node_map(&all_nodes);

    for name in sorted_results {
        if let Some(&node) = node_map.get(name.as_str()) {
            table.add_row(row_builder(node));
        }
    }

    logger.table(&table);

    if let Some(footer) = &config.footer_message {
        logger.info(footer);
    }

    Ok(())
}
