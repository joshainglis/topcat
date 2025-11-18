//! Common utilities and configuration logic shared across all commands.
//!
//! This module contains shared functionality for building dependency graphs,
//! parsing configuration, and other utilities used by multiple commands.
//! Extracting this common logic eliminates duplication and ensures consistent
//! behavior across analyze, clean, concat, schema, and export commands.

use std::path::PathBuf;

use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::config;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::schema_utils::SchemaFilter;
use topcat::sql_config;

// ============================================================================
// Platform Utilities
// ============================================================================

/// Returns the platform-specific null device path.
///
/// This is used when building graphs for analysis or export where no actual
/// output file is needed, but the Config struct requires an output path.
///
/// Delegates to the platform module for platform-specific implementation.
pub fn null_device() -> &'static str {
    topcat::platform::null_device()
}

// ============================================================================
// Schema Filtering
// ============================================================================

/// Convert schema filter CLI args to a SchemaFilter instance.
///
/// When filtering by specific schemas, this function converts schema names
/// into a SchemaFilter that can be used for node matching and graph filtering.
///
/// # Arguments
///
/// * `schema_filter` - List of schema names to filter by
///
/// # Returns
///
/// A `SchemaFilter` instance. Use `.to_option()` to get `Option<Vec<String>>`
/// for backward compatibility with APIs expecting node prefixes.
///
/// # Examples
///
/// ```
/// # use topcat::commands::common::build_schema_filter;
/// let filter = build_schema_filter(&vec!["auth".to_string()]);
/// let prefixes = filter.to_option(); // Some(vec!["auth", "auth."])
/// ```
pub fn build_schema_filter(schema_filter: &[String]) -> SchemaFilter {
    SchemaFilter::from(schema_filter)
}

// ============================================================================
// External Usage Checking
// ============================================================================

/// Build an external usage checker from CLI arguments.
///
/// Sets up an `ExternalUsageChecker` if both directories and patterns are provided.
/// Returns `None` if external checking is not configured.
///
/// # Arguments
///
/// * `external_check_dirs` - Directories to scan for external usage
/// * `external_check_patterns` - File patterns to check (e.g., "*.py")
/// * `verbose` - Whether to show verbose output during scanning
///
/// # Returns
///
/// `Ok(Some(ExternalUsageChecker))` if configured,
/// `Ok(None)` if not configured,
/// `Err(TopCatError)` if checker initialization fails
pub fn build_external_checker(
    external_check_dirs: &[PathBuf],
    external_check_patterns: &[String],
    verbose: bool,
) -> Result<Option<ExternalUsageChecker>, TopCatError> {
    if external_check_dirs.is_empty() || external_check_patterns.is_empty() {
        return Ok(None);
    }

    println!(
        "🔍 Setting up external usage checker for {} directories...",
        external_check_dirs.len()
    );

    ExternalUsageChecker::new(external_check_dirs, external_check_patterns, !verbose)
        .map(Some)
        .map_err(|e| TopCatError::ConfigError(format!("External checker error: {e}")))
}

// ============================================================================
// Graph Building
// ============================================================================

/// Build a TCGraph with common configuration pattern used across commands.
///
/// This function encapsulates the standard graph building process used by analyze,
/// clean, schema, and export commands. It creates a Config struct with appropriate
/// settings and builds the dependency graph.
///
/// # Arguments
///
/// * `input_dirs` - Directories to search for files
/// * `include_extensions` - File extensions to include (e.g., ["sql", "py"])
/// * `exclude_extensions` - File extensions to exclude
/// * `include_globs` - Optional glob patterns for file inclusion
/// * `exclude_globs` - Optional glob patterns for file exclusion
/// * `include_hidden` - Whether to include hidden files and directories
/// * `verbose` - Enable verbose output
/// * `comment_str` - Comment string for output (not used for analysis)
/// * `layers` - Layer ordering configuration
/// * `fallback_layer` - Default layer for files without layer metadata
/// * `sql_discovery` - SQL discovery configuration
/// * `schema_filter_prefixes` - Optional node name prefixes for schema filtering
///
/// # Returns
///
/// `Ok(TCGraph)` with the fully constructed dependency graph, or
/// `Err(TopCatError)` if graph building fails (e.g., cycle detected, missing deps)
#[allow(clippy::too_many_arguments)]
pub fn build_graph(
    input_dirs: Vec<PathBuf>,
    include_extensions: Option<&[String]>,
    exclude_extensions: Option<&[String]>,
    include_globs: Option<&[String]>,
    exclude_globs: Option<&[String]>,
    include_hidden: bool,
    verbose: bool,
    comment_str: String,
    layers: Vec<String>,
    fallback_layer: String,
    sql_discovery: sql_config::SqlDiscoveryConfig,
    schema_filter_prefixes: Option<Vec<String>>,
) -> Result<TCGraph, TopCatError> {
    let config = config::Config {
        input_dirs,
        include_extensions,
        exclude_extensions,
        include_globs,
        exclude_globs,
        output: PathBuf::from(null_device()),
        comment_str,
        file_separator_str: String::new(),
        file_end_str: String::new(),
        include_hidden,
        verbose,
        include_node_prefixes: schema_filter_prefixes.as_deref(),
        exclude_node_prefixes: None,
        dry_run: false,
        subdir_filter: None,
        layers,
        fallback_layer,
        sql_discovery,
        header_update_mode: sql_config::HeaderUpdateMode::Never,
        header_output_dir: None,
    };

    let mut graph = TCGraph::new(&config);
    graph.build_graph()?;
    Ok(graph)
}

// ============================================================================
// Node Utilities
// ============================================================================

/// Build a HashMap for O(1) node lookups by name.
///
/// This is a re-export of `topcat::graph_utils::build_name_to_node_map` for
/// backward compatibility. New code should use the graph_utils module directly.
///
/// This helper improves performance from O(n²) to O(n) for analyses that need
/// to look up node details repeatedly. Instead of linear searching through all
/// nodes for each result, build a hash map once and use O(1) lookups.
///
/// # Arguments
///
/// * `nodes` - Slice of all file nodes in the graph
///
/// # Returns
///
/// HashMap mapping node names to node references for fast lookup
pub fn build_node_map(
    nodes: &[topcat::file_node::FileNode],
) -> std::collections::HashMap<&str, &topcat::file_node::FileNode> {
    topcat::graph_utils::build_name_to_node_map(nodes)
}

/// Build a dependency graph from Settings configuration.
///
/// Constructs a `TCGraph` using the unified Settings configuration,
/// extracting schema filters from CLI arguments and applying all
/// necessary file filters, layers, and SQL discovery settings.
///
/// # Arguments
///
/// * `schemas` - Optional schema filter from CLI (overrides settings)
/// * `settings` - Configuration settings
///
/// # Returns
///
/// `Ok(TCGraph)` with the built graph, or `Err(TopCatError)` if:
/// - Configuration is invalid
/// - Files cannot be read
/// - Cycles are detected
/// - Required dependencies are missing
pub fn build_graph_from_settings(
    schemas: &Option<Vec<String>>,
    settings: &topcat::settings::Settings,
) -> Result<TCGraph, TopCatError> {
    // Extract schema filter for node prefixes
    let schema_filter: Vec<String> = schemas
        .clone()
        .unwrap_or_else(|| settings.schema_filtering.schemas.clone());

    let include_node_prefixes = if schema_filter.is_empty() {
        None
    } else {
        build_schema_filter(&schema_filter).to_option()
    };

    // Get fallback layer (required)
    let fallback_layer = settings.layers.fallback.clone();

    build_graph(
        settings.input_dirs.clone(),
        if settings.filters.include_extensions.is_empty() {
            None
        } else {
            Some(&settings.filters.include_extensions)
        },
        if settings.filters.exclude_extensions.is_empty() {
            None
        } else {
            Some(&settings.filters.exclude_extensions)
        },
        if settings.filters.include_globs.is_empty() {
            None
        } else {
            Some(&settings.filters.include_globs)
        },
        if settings.filters.exclude_globs.is_empty() {
            None
        } else {
            Some(&settings.filters.exclude_globs)
        },
        settings.filters.include_hidden,
        settings.behavior.verbose,
        settings.formatting.comment_str.clone(),
        settings.layers.names.clone(),
        fallback_layer,
        settings.sql_discovery.clone(),
        include_node_prefixes,
    )
}

/// Build a root node matcher from Settings configuration.
///
/// Root matchers identify which nodes should be treated as entry points
/// (roots) in the dependency graph. This is used for protecting nodes
/// from deletion and determining dead branches.
///
/// # Arguments
///
/// * `settings` - Configuration settings containing root node patterns
///
/// # Returns
///
/// - `Ok(Some(RootNodeMatcher))` if any patterns are specified
/// - `Ok(None)` if no root patterns are configured
/// - `Err(TopCatError)` if configuration is invalid or patterns cannot be compiled
pub fn build_root_matcher_from_settings(
    settings: &topcat::settings::Settings,
) -> Result<Option<topcat::analysis::root_matcher::RootNodeMatcher>, TopCatError> {
    use topcat::analysis::root_matcher::RootNodeMatcher;

    let root_nodes = settings.analysis.root_nodes.clone();
    let root_patterns = settings.analysis.root_patterns.clone();
    let root_regex = settings.analysis.root_regex.clone();
    let root_dirs: Vec<PathBuf> = settings
        .analysis
        .root_dirs
        .iter()
        .map(PathBuf::from)
        .collect();

    // Create matcher only if we have any root configuration
    if root_nodes.is_empty()
        && root_patterns.is_empty()
        && root_regex.is_empty()
        && root_dirs.is_empty()
    {
        Ok(None)
    } else {
        RootNodeMatcher::new(root_nodes, root_patterns, root_regex, root_dirs)
            .map(Some)
            .map_err(TopCatError::ConfigError)
    }
}

/// Build an external usage checker from Settings configuration.
///
/// Sets up external usage checking if configured in settings. This checks
/// for references to file nodes in external codebases (e.g., Python files
/// referencing SQL functions).
///
/// # Arguments
///
/// * `settings` - Configuration settings containing external check directories and patterns
///
/// # Returns
///
/// `Ok(Some(ExternalUsageChecker))` if configured,
/// `Ok(None)` if not configured,
/// `Err(TopCatError)` if checker initialization fails
pub fn build_external_checker_from_settings(
    settings: &topcat::settings::Settings,
) -> Result<Option<ExternalUsageChecker>, TopCatError> {
    let dirs: Vec<PathBuf> = settings
        .analysis
        .external_check_dirs
        .iter()
        .map(PathBuf::from)
        .collect();
    let patterns = settings.analysis.external_check_patterns.clone();

    build_external_checker(&dirs, &patterns, settings.behavior.verbose)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_schema_filter_empty() {
        let filter = build_schema_filter(&[]);
        assert!(filter.is_empty());
        assert_eq!(filter.to_option(), None);
    }

    #[test]
    fn test_build_schema_filter_single() {
        let filter = build_schema_filter(&["auth".to_string()]);
        assert!(!filter.is_empty());
        assert_eq!(
            filter.to_option(),
            Some(vec!["auth".to_string(), "auth.".to_string()])
        );
    }

    #[test]
    fn test_build_schema_filter_multiple() {
        let filter = build_schema_filter(&["auth".to_string(), "billing".to_string()]);
        assert!(!filter.is_empty());
        let prefixes = filter.to_option().unwrap();
        assert_eq!(prefixes.len(), 4);
        assert!(prefixes.contains(&"auth".to_string()));
        assert!(prefixes.contains(&"auth.".to_string()));
        assert!(prefixes.contains(&"billing".to_string()));
        assert!(prefixes.contains(&"billing.".to_string()));
    }

    #[test]
    fn test_null_device() {
        let device = null_device();
        #[cfg(unix)]
        assert_eq!(device, "/dev/null");
        #[cfg(windows)]
        assert_eq!(device, "NUL");
    }
}
