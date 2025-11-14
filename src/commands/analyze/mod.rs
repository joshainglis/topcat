//! Analysis commands for dependency graph inspection.
//!
//! This module implements the `analyze` subcommand which provides various
//! analyses of dependency graphs built from files with header metadata.
//!
//! # Available Analysis Types
//!
//! - **Dead Branches**: Find complete subtrees that can be removed together
//! - **Orphans**: Files with no dependencies or dependents
//! - **Unrequired**: Files not required by any other files
//! - **Leaf Nodes**: Files with dependencies but no dependents
//! - **Root Nodes**: Files with dependents but no dependencies
//! - **Cycles**: Detect circular dependencies (invalid DAGs)
//! - **Missing**: Find referenced but non-existent dependencies
//! - **File**: Detailed analysis of a specific file
//!
//! # Module Structure
//!
//! The analyze command has been modularized for maintainability:
//!
//! - `common`: Shared utilities (AnalysisLogger, display config, generic analysis function)
//! - `dead_branches`: Dead branches analysis
//! - `orphans`: Orphan files analysis
//! - `unrequired`: Unrequired files analysis
//! - `leaf_nodes`: Leaf nodes analysis
//! - `root_nodes`: Root nodes analysis
//! - `cycles`: Cycle detection analysis
//! - `missing`: Missing dependencies analysis
//! - `file`: Single file detailed analysis
//!
//! # Examples
//!
//! ```bash
//! # Find orphaned files
//! topcat analyze -i sql/ -e sql orphans
//!
//! # Find dead branches with external usage checking
//! topcat analyze -i sql/ -e sql \
//!   --external-check-dir src/ \
//!   --external-check-pattern "*.py" \
//!   dead-branches
//!
//! # Analyze specific schema
//! topcat analyze -i sql/ -e sql --schema my_schema orphans
//! ```

mod common;
mod cycles;
mod dead_branches;
mod file;
mod leaf_nodes;
mod missing;
mod orphans;
mod root_nodes;
mod unrequired;

use std::path::PathBuf;

use clap::{Args, Subcommand};
use env_logger::Builder;
use log::LevelFilter;

use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;

use super::common as cmd_common;

/// Command-line arguments for the analyze subcommand.
///
/// Provides various dependency graph analyses to identify cleanup candidates,
/// detect issues, and understand graph structure.
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    #[arg(
        short = 'i',
        long = "input-dirs",
        help = "Paths to directories containing files to analyze",
        value_name = "DIRS"
    )]
    input_dirs: Vec<PathBuf>,

    #[arg(
        short = 'e',
        long = "include-exts",
        help = "Only include files with the given file extensions",
        value_name = "EXTENSIONS"
    )]
    include_file_extensions: Option<Vec<String>>,

    #[arg(
        short = 'E',
        long = "exclude-exts",
        help = "Exclude files with the given file extensions",
        value_name = "EXTENSIONS"
    )]
    exclude_file_extensions: Option<Vec<String>>,

    #[arg(
        short = 'g',
        long = "include-glob",
        help = "Only include files matching glob pattern",
        value_name = "PATTERN"
    )]
    include_globs: Option<Vec<String>>,

    #[arg(
        short = 'G',
        long = "exclude-glob",
        help = "Exclude files matching given glob pattern",
        value_name = "PATTERN"
    )]
    exclude_globs: Option<Vec<String>>,

    #[arg(
        short = 'c',
        long = "comment-prefix",
        help = "The string used to denote a comment",
        default_value = "--"
    )]
    comment_str: String,

    #[arg(long = "include-hidden", help = "Include hidden files and directories")]
    include_hidden_files_and_directories: bool,

    #[arg(short = 'v', long = "verbose", help = "Print debug information")]
    verbose: bool,

    #[arg(
        short = 'q',
        long = "quiet",
        help = "Suppress output, only return exit code"
    )]
    quiet: bool,

    #[arg(
        long = "layers",
        help = "Comma-separated list of layer names in order",
        value_name = "LAYERS"
    )]
    layers: Option<String>,

    #[arg(
        long = "fallback-layer",
        help = "Default layer for nodes without explicit layer declaration",
        value_name = "LAYER"
    )]
    fallback_layer: Option<String>,

    // SQL Discovery Options (for building the graph)
    #[arg(
        long = "enable-sql-discovery",
        help = "Enable automatic dependency discovery from SQL content"
    )]
    enable_sql_discovery: bool,

    #[arg(
        long = "sql-config",
        help = "Path to TOML configuration file for SQL discovery patterns",
        value_name = "FILE"
    )]
    sql_config_file: Option<PathBuf>,

    #[arg(
        long = "schema-pattern",
        help = "Regex pattern for matching schema names",
        value_name = "PATTERN"
    )]
    schema_pattern: Option<String>,

    #[arg(
        long = "merge-strategy",
        help = "How to merge discovered and manual dependencies",
        value_name = "STRATEGY",
        default_value = cmd_common::DEFAULT_MERGE_STRATEGY
    )]
    merge_strategy: String,

    // External usage checking
    #[arg(
        long = "external-check-dir",
        help = "Directory to check for external usage of SQL functions (can specify multiple times)",
        value_name = "DIR"
    )]
    external_check_dirs: Vec<PathBuf>,

    #[arg(
        long = "external-check-pattern",
        help = "File pattern to check for external usage (e.g., '*.py', can specify multiple times)",
        value_name = "PATTERN"
    )]
    external_check_patterns: Vec<String>,

    // Root Node Configuration
    #[arg(
        long = "root-nodes",
        help = "Specific node names to always treat as roots (can specify multiple times)",
        value_name = "NODE"
    )]
    root_nodes: Vec<String>,

    #[arg(
        long = "root-pattern",
        help = "Glob patterns for root files (e.g., '**/api/*.sql', can specify multiple times)",
        value_name = "PATTERN"
    )]
    root_patterns: Vec<String>,

    #[arg(
        long = "root-regex",
        help = "Regex patterns for root node names (e.g., '^api_.*', can specify multiple times)",
        value_name = "REGEX"
    )]
    root_regex: Vec<String>,

    #[arg(
        long = "root-dir",
        help = "Directories whose files are all roots (can specify multiple times)",
        value_name = "DIR"
    )]
    root_dirs: Vec<PathBuf>,

    // Schema Filtering
    #[arg(
        long = "schema",
        help = "Filter analysis to specific schema(s) (can specify multiple times)",
        value_name = "SCHEMA"
    )]
    schema_filter: Vec<String>,

    #[command(subcommand)]
    command: AnalyzeCommand,
}

#[derive(Debug, Subcommand)]
enum AnalyzeCommand {
    /// Find complete dead branches (subtrees that can be trimmed together)
    DeadBranches,
    /// Find files with no dependencies or dependents
    Orphans,
    /// Find files not required by any other files
    Unrequired,
    /// Find files with dependencies but no dependents
    LeafNodes,
    /// Find files with dependents but no dependencies
    RootNodes,
    /// Detect cycles in the dependency graph
    Cycles,
    /// Find missing dependencies (referenced but non-existent files)
    Missing,
    /// Analyze a specific file in detail
    File {
        #[arg(help = "Path to the file to analyze")]
        path: PathBuf,
    },
}

impl AnalyzeArgs {
    /// Execute the analysis command.
    ///
    /// Main entry point that:
    /// 1. Initializes logging based on verbose/quiet flags
    /// 2. Handles special cases (cycles, missing) that don't need full graph
    /// 3. Builds the dependency graph for other commands
    /// 4. Sets up external usage checker if requested
    /// 5. Dispatches to the appropriate analysis method
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(TopCatError)` if:
    /// - Graph building fails (cycles, missing deps, config errors)
    /// - Analysis execution fails
    /// - External checker setup fails
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Initialize logging (unless quiet mode)
        if !self.quiet {
            if self.verbose {
                Builder::new()
                    .filter(None, LevelFilter::Debug)
                    .try_init()
                    .ok();
            } else {
                Builder::new()
                    .filter(None, LevelFilter::Info)
                    .try_init()
                    .ok();
            }
        }

        // For cycles and missing commands, we handle graph building specially
        match &self.command {
            AnalyzeCommand::Cycles => {
                return cycles::analyze(
                    self.quiet,
                    &self.sql_config_file,
                    self.enable_sql_discovery,
                    &self.schema_pattern,
                    &self.merge_strategy,
                    self.input_dirs.clone(),
                    self.include_file_extensions.as_deref(),
                    self.exclude_file_extensions.as_deref(),
                    self.include_globs.as_deref(),
                    self.exclude_globs.as_deref(),
                    self.include_hidden_files_and_directories,
                    self.verbose,
                    self.comment_str.clone(),
                    &self.layers,
                    &self.fallback_layer,
                    &self.schema_filter,
                );
            }
            AnalyzeCommand::Missing => {
                return missing::analyze(
                    self.quiet,
                    &self.sql_config_file,
                    self.enable_sql_discovery,
                    &self.schema_pattern,
                    &self.merge_strategy,
                    self.input_dirs.clone(),
                    self.include_file_extensions.as_deref(),
                    self.exclude_file_extensions.as_deref(),
                    self.include_globs.as_deref(),
                    self.exclude_globs.as_deref(),
                    self.include_hidden_files_and_directories,
                    self.verbose,
                    self.comment_str.clone(),
                    &self.layers,
                    &self.fallback_layer,
                    &self.schema_filter,
                );
            }
            _ => {}
        }

        // Build the dependency graph (for all other commands)
        let graph = self.build_graph()?;

        // Check for external usage if requested
        let external_checker = cmd_common::build_external_checker(
            &self.external_check_dirs,
            &self.external_check_patterns,
            self.verbose,
        )?;

        // Build root matcher from CLI args and config
        let root_matcher = self.build_root_matcher()?;

        // Execute the requested analysis
        match &self.command {
            AnalyzeCommand::DeadBranches => dead_branches::analyze(
                self.quiet,
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
            ),
            AnalyzeCommand::Orphans => {
                orphans::analyze(self.quiet, &graph, external_checker.as_ref())
            }
            AnalyzeCommand::Unrequired => {
                unrequired::analyze(self.quiet, &graph, external_checker.as_ref())
            }
            AnalyzeCommand::LeafNodes => {
                leaf_nodes::analyze(self.quiet, &graph, external_checker.as_ref())
            }
            AnalyzeCommand::RootNodes => root_nodes::analyze(self.quiet, &graph),
            AnalyzeCommand::File { path } => {
                file::analyze(self.quiet, &graph, path, external_checker.as_ref())
            }
            // Cycles and Missing are handled earlier
            AnalyzeCommand::Cycles | AnalyzeCommand::Missing => unreachable!(),
        }
    }

    /// Build the dependency graph from configuration.
    ///
    /// Constructs a `TCGraph` by loading configuration and building the graph.
    ///
    /// # Returns
    ///
    /// `Ok(TCGraph)` with the built graph, or `Err(TopCatError)` if:
    /// - Configuration is invalid
    /// - Files cannot be read
    /// - Cycles are detected
    /// - Required dependencies are missing
    fn build_graph(&self) -> Result<topcat::file_dag::TCGraph, TopCatError> {
        let sql_discovery = cmd_common::load_sql_discovery_config(
            &self.sql_config_file,
            self.enable_sql_discovery,
            &self.schema_pattern,
            &self.merge_strategy,
        )?;
        let (layers, fallback_layer) =
            cmd_common::parse_and_validate_layers(&self.layers, &self.fallback_layer)?;
        let include_node_prefixes =
            cmd_common::build_schema_filter(&self.schema_filter).to_option();

        cmd_common::build_graph(
            self.input_dirs.clone(),
            self.include_file_extensions.as_deref(),
            self.exclude_file_extensions.as_deref(),
            self.include_globs.as_deref(),
            self.exclude_globs.as_deref(),
            self.include_hidden_files_and_directories,
            self.verbose,
            self.comment_str.clone(),
            layers,
            fallback_layer,
            sql_discovery,
            include_node_prefixes,
        )
    }

    /// Build a root node matcher from config file and CLI args.
    ///
    /// Root matchers identify which nodes should be treated as entry points
    /// (roots) in the dependency graph. Configuration from files is merged with
    /// CLI arguments (CLI extends/overrides config).
    ///
    /// # Returns
    ///
    /// - `Ok(Some(RootNodeMatcher))` if any patterns are specified
    /// - `Ok(None)` if no root patterns are configured
    /// - `Err(TopCatError)` if configuration is invalid or patterns cannot be compiled
    fn build_root_matcher(&self) -> Result<Option<RootNodeMatcher>, TopCatError> {
        cmd_common::build_root_matcher(
            &self.sql_config_file,
            self.root_nodes.clone(),
            self.root_patterns.clone(),
            self.root_regex.clone(),
            self.root_dirs.clone(),
        )
    }
}
