//! Clean commands for removing unused files.
//!
//! This module implements the `clean` subcommand which safely removes unused files
//! from the project based on dependency analysis.
//!
//! # Available Clean Operations
//!
//! - **Dead Branches**: Remove complete subtrees that can be trimmed together
//! - **Orphans**: Remove files with no dependencies or dependents
//! - **Unrequired**: Remove files not required by any other files
//! - **Targets**: Remove specific files by pattern (with dependency checking)
//!
//! # Module Structure
//!
//! The clean command has been modularized for maintainability:
//!
//! - `common`: Shared utilities (deletion preview, perform deletion, root filtering)
//! - `dead_branches`: Dead branches cleanup
//! - `orphans`: Orphan files cleanup
//! - `unrequired`: Unrequired files cleanup
//! - `targets`: Specific target files cleanup
//!
//! # Safety Features
//!
//! - **Dry-run by default**: Must explicitly enable deletion with `--no-dry-run`
//! - **Confirmation prompt**: Asks for confirmation before deletion (unless `--force`)
//! - **External usage checking**: Optional filtering to preserve externally-used files
//! - **Root protection**: Optional patterns to protect important entry point files
//! - **Dependency checking**: Prevents deletion of files still required by others
//!
//! # Examples
//!
//! ```bash
//! # Preview what would be deleted
//! topcat clean -i sql/ -e sql dead-branches
//!
//! # Actually delete orphan files
//! topcat clean -i sql/ -e sql orphans --no-dry-run
//!
//! # Delete with protection and external checking
//! topcat clean -i sql/ -e sql \
//!   --root-pattern "**/api/*.sql" \
//!   --external-check-dir src/ \
//!   --external-check-pattern "*.py" \
//!   dead-branches --no-dry-run
//! ```

mod common;
mod dead_branches;
mod orphans;
mod targets;
mod unrequired;

use std::path::PathBuf;

use clap::{Args, Subcommand};
use env_logger::Builder;
use log::LevelFilter;

use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;

use super::common as cmd_common;

/// Remove unused files based on dependency analysis
#[derive(Debug, Args)]
pub struct CleanArgs {
    #[arg(
        short = 'i',
        long = "input-dirs",
        help = "Paths to directories containing files to clean",
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
        default_value = "discovery-only"
    )]
    merge_strategy: String,

    // External usage checking
    #[arg(
        long = "external-check-dir",
        help = "Directory to check for external usage before deletion (can specify multiple times)",
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
        help = "Specific node names to protect from deletion (can specify multiple times)",
        value_name = "NODE"
    )]
    root_nodes: Vec<String>,

    #[arg(
        long = "root-pattern",
        help = "Glob patterns for protected files (e.g., '**/api/*.sql', can specify multiple times)",
        value_name = "PATTERN"
    )]
    root_patterns: Vec<String>,

    #[arg(
        long = "root-regex",
        help = "Regex patterns for protected node names (e.g., '^api_.*', can specify multiple times)",
        value_name = "REGEX"
    )]
    root_regex: Vec<String>,

    #[arg(
        long = "root-dir",
        help = "Directories whose files are protected (can specify multiple times)",
        value_name = "DIR"
    )]
    root_dirs: Vec<PathBuf>,

    // Schema Filtering
    #[arg(
        long = "schema",
        help = "Filter cleaning to specific schema(s) (can specify multiple times)",
        value_name = "SCHEMA"
    )]
    schema_filter: Vec<String>,

    // Clean-specific flags
    #[arg(
        long = "dry-run",
        help = "Show what would be deleted without actually deleting",
        default_value = "true"
    )]
    dry_run: bool,

    #[arg(
        long = "no-dry-run",
        help = "Actually perform deletion (overrides --dry-run)",
        conflicts_with = "dry_run"
    )]
    no_dry_run: bool,

    #[arg(
        long = "force",
        short = 'f',
        help = "Skip confirmation prompt (for automation)"
    )]
    force: bool,

    #[command(subcommand)]
    command: CleanCommand,
}

#[derive(Debug, Subcommand)]
enum CleanCommand {
    /// Remove complete dead branches (subtrees that can be trimmed together)
    DeadBranches,
    /// Remove files with no dependencies or dependents
    Orphans,
    /// Remove files not required by any other files
    Unrequired,
    /// Remove specific target files (with dependency checking)
    Targets {
        #[arg(help = "File paths or patterns to remove")]
        files: Vec<String>,
    },
}

impl CleanArgs {
    /// Execute the clean command.
    ///
    /// Main entry point that:
    /// 1. Initializes logging
    /// 2. Determines if actually deleting or dry-run
    /// 3. Builds the dependency graph
    /// 4. Sets up external usage checker if requested
    /// 5. Builds root matcher for protection patterns
    /// 6. Dispatches to the appropriate clean operation
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(TopCatError)` if:
    /// - Graph building fails
    /// - External checker setup fails
    /// - Deletion fails
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Initialize logging
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

        // Determine if we're actually deleting or just previewing
        let actually_delete = self.no_dry_run || !self.dry_run;

        if actually_delete {
            println!("⚠️  DELETION MODE: Files will be permanently removed!");
        } else {
            println!("🔍 DRY-RUN MODE: No files will be deleted");
        }

        // Build the dependency graph
        let graph = self.build_graph()?;

        // Check for external usage if requested
        let external_checker = if !self.external_check_dirs.is_empty()
            && !self.external_check_patterns.is_empty()
        {
            println!(
                "🔍 Setting up external usage checker for {} directories...",
                self.external_check_dirs.len()
            );
            Some(
                ExternalUsageChecker::new(
                    &self.external_check_dirs,
                    &self.external_check_patterns,
                    !self.verbose,
                )
                .map_err(|e| TopCatError::ConfigError(format!("External checker error: {e}")))?,
            )
        } else {
            None
        };

        // Build root matcher from CLI args and config
        let root_matcher = self.build_root_matcher()?;

        // Execute the requested cleanup
        match &self.command {
            CleanCommand::DeadBranches => dead_branches::clean(
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
                self.force,
                self.verbose,
            ),
            CleanCommand::Orphans => orphans::clean(
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
                self.force,
                self.verbose,
            ),
            CleanCommand::Unrequired => unrequired::clean(
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
                self.force,
                self.verbose,
            ),
            CleanCommand::Targets { files } => targets::clean(
                &graph,
                files,
                root_matcher.as_ref(),
                actually_delete,
                self.force,
                self.verbose,
            ),
        }
    }

    /// Build the dependency graph from configuration.
    fn build_graph(&self) -> Result<topcat::file_dag::TCGraph, TopCatError> {
        let sql_discovery = cmd_common::load_sql_discovery_config(
            &self.sql_config_file,
            self.enable_sql_discovery,
            &self.schema_pattern,
            &self.merge_strategy,
        )?;
        let (layers, fallback_layer) =
            cmd_common::parse_and_validate_layers(&self.layers, &self.fallback_layer)?;
        let include_node_prefixes = cmd_common::build_schema_filter_prefixes(&self.schema_filter);

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
