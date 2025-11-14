//! Common utilities and configuration logic shared across all commands.
//!
//! This module contains shared functionality for building dependency graphs,
//! parsing configuration, and other utilities used by multiple commands.
//! Extracting this common logic eliminates duplication and ensures consistent
//! behavior across analyze, clean, concat, schema, and export commands.

use std::path::PathBuf;

use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::config;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::schema_utils::SchemaFilter;
use topcat::sql_config;

// ============================================================================
// Constants
// ============================================================================

/// Default fallback layer when no layer is specified in file metadata
pub const DEFAULT_FALLBACK_LAYER: &str = "normal";

/// Default prepend layer (executed before normal layer)
pub const DEFAULT_LAYER_PREPEND: &str = "prepend";

/// Default normal layer (main execution layer)
pub const DEFAULT_LAYER_NORMAL: &str = "normal";

/// Default append layer (executed after normal layer)
pub const DEFAULT_LAYER_APPEND: &str = "append";

/// Default merge strategy for SQL discovery
pub const DEFAULT_MERGE_STRATEGY: &str = "discovery-only";

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
// Layer Configuration
// ============================================================================

/// Parse and validate layers from CLI arguments.
///
/// Layers enforce ordering between groups of files. This function parses a
/// comma-separated layer list (or uses defaults), validates that the fallback
/// layer exists in the layer list, and returns both.
///
/// # Arguments
///
/// * `layers_arg` - Optional comma-separated layer string from CLI
/// * `fallback_layer_arg` - Optional fallback layer name from CLI
///
/// # Returns
///
/// `Ok((layers, fallback_layer))` with the validated configuration, or
/// `Err(TopCatError::ConfigError)` if the fallback layer is not in the layers list.
///
/// # Default Configuration
///
/// If not specified: `["prepend", "normal", "append"]` with fallback `"normal"`
pub fn parse_and_validate_layers(
    layers_arg: &Option<String>,
    fallback_layer_arg: &Option<String>,
) -> Result<(Vec<String>, String), TopCatError> {
    let layers = if let Some(layers_str) = layers_arg {
        layers_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    } else {
        vec![
            DEFAULT_LAYER_PREPEND.to_string(),
            DEFAULT_LAYER_NORMAL.to_string(),
            DEFAULT_LAYER_APPEND.to_string(),
        ]
    };

    let fallback_layer = fallback_layer_arg
        .clone()
        .unwrap_or_else(|| DEFAULT_FALLBACK_LAYER.to_string());

    if !layers.contains(&fallback_layer) {
        return Err(TopCatError::ConfigError(format!(
            "Fallback layer '{fallback_layer}' is not in the layers list: {layers:?}"
        )));
    }

    Ok((layers, fallback_layer))
}

// ============================================================================
// SQL Discovery Configuration
// ============================================================================

/// Load SQL discovery configuration from file and apply CLI overrides.
///
/// This function follows a layered configuration approach:
/// 1. Start with default configuration
/// 2. Override with settings from config file (if provided)
/// 3. Apply CLI argument overrides
///
/// # Arguments
///
/// * `config_file` - Optional path to topcat.toml configuration file
/// * `enable_sql_discovery` - CLI flag to enable SQL discovery
/// * `schema_pattern` - Optional regex pattern for schema extraction
/// * `merge_strategy_str` - Strategy for merging discovered and manual dependencies
///
/// # Returns
///
/// `Ok(SqlDiscoveryConfig)` with the merged configuration, or
/// `Err(TopCatError)` if configuration is invalid (e.g., invalid merge strategy)
pub fn load_sql_discovery_config(
    config_file: &Option<PathBuf>,
    enable_sql_discovery: bool,
    schema_pattern: &Option<String>,
    merge_strategy_str: &str,
) -> Result<sql_config::SqlDiscoveryConfig, TopCatError> {
    // Start with file config if provided
    let mut config = if let Some(config_path) = config_file {
        match sql_config::TopcatConfig::from_file(config_path) {
            Ok(cfg) => cfg.sql_discovery,
            Err(e) => {
                eprintln!("Warning: Failed to load SQL config file: {e}");
                sql_config::SqlDiscoveryConfig::default()
            }
        }
    } else {
        sql_config::SqlDiscoveryConfig::default()
    };

    // Apply CLI overrides
    if enable_sql_discovery {
        config.enabled = true;
    }

    if let Some(pattern) = schema_pattern {
        config.schema_pattern = Some(pattern.clone());
    }

    // Parse merge strategy
    config.merge_strategy = merge_strategy_str
        .parse()
        .map_err(|e: String| TopCatError::ConfigError(e))?;

    Ok(config)
}

// ============================================================================
// Root Node Protection
// ============================================================================

/// Build a root node matcher from CLI arguments and config file.
///
/// Root node matchers protect important nodes (e.g., API endpoints, migration entry points)
/// from being flagged as dead code during analysis. This function merges root node
/// specifications from both CLI arguments and config files.
///
/// # Arguments
///
/// * `config_file` - Optional path to topcat.toml configuration file
/// * `root_nodes` - Explicit node names to protect
/// * `root_patterns` - Glob patterns for protecting nodes (e.g., "api/**/*.sql")
/// * `root_regex` - Regex patterns for protecting nodes
/// * `root_dirs` - Directory paths to protect (all nodes in these dirs are roots)
///
/// # Returns
///
/// `Ok(Some(RootNodeMatcher))` if any root configuration is specified,
/// `Ok(None)` if no root protection is configured,
/// `Err(TopCatError)` if configuration is invalid (e.g., invalid regex)
pub fn build_root_matcher(
    config_file: &Option<PathBuf>,
    root_nodes: Vec<String>,
    root_patterns: Vec<String>,
    root_regex: Vec<String>,
    root_dirs: Vec<PathBuf>,
) -> Result<Option<RootNodeMatcher>, TopCatError> {
    let mut all_root_nodes = root_nodes;
    let mut all_root_patterns = root_patterns;
    let mut all_root_regex = root_regex;
    let mut all_root_dirs = root_dirs;

    // Load additional configuration from config file
    if let Some(config_path) = config_file {
        if let Ok(cfg) = sql_config::TopcatConfig::from_file(config_path) {
            let analysis_cfg = cfg.analysis;
            all_root_nodes.extend(analysis_cfg.root_nodes);
            all_root_patterns.extend(analysis_cfg.root_patterns);
            all_root_regex.extend(analysis_cfg.root_regex);
            all_root_dirs.extend(analysis_cfg.root_dirs.into_iter().map(PathBuf::from));
        }
    }

    // Create matcher only if we have any root configuration
    if !all_root_nodes.is_empty()
        || !all_root_patterns.is_empty()
        || !all_root_regex.is_empty()
        || !all_root_dirs.is_empty()
    {
        RootNodeMatcher::new(
            all_root_nodes,
            all_root_patterns,
            all_root_regex,
            all_root_dirs,
        )
        .map(Some)
        .map_err(TopCatError::ConfigError)
    } else {
        Ok(None)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_validate_layers_with_defaults() {
        let result = parse_and_validate_layers(&None, &None);
        assert!(result.is_ok());
        let (layers, fallback) = result.unwrap();
        assert_eq!(layers, vec!["prepend", "normal", "append"]);
        assert_eq!(fallback, "normal");
    }

    #[test]
    fn test_parse_and_validate_layers_custom() {
        let layers_str = Some("first,second,third".to_string());
        let fallback = Some("second".to_string());
        let result = parse_and_validate_layers(&layers_str, &fallback);
        assert!(result.is_ok());
        let (layers, fallback_layer) = result.unwrap();
        assert_eq!(layers, vec!["first", "second", "third"]);
        assert_eq!(fallback_layer, "second");
    }

    #[test]
    fn test_parse_and_validate_layers_invalid_fallback() {
        let layers_str = Some("first,second,third".to_string());
        let fallback = Some("invalid".to_string());
        let result = parse_and_validate_layers(&layers_str, &fallback);
        assert!(result.is_err());
    }

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
