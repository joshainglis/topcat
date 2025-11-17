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
use topcat::cli::CommonArgs;
use topcat::exceptions::TopCatError;
use topcat::settings::Settings;

use super::common as cmd_common;

/// Command-line arguments for the analyze subcommand.
///
/// Provides various dependency graph analyses to identify cleanup candidates,
/// detect issues, and understand graph structure.
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    #[command(flatten)]
    pub common: CommonArgs,

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
    /// 1. Loads configuration from all sources (files, env vars, CLI)
    /// 2. Initializes logging based on verbose/quiet flags
    /// 3. Handles special cases (cycles, missing) that don't need full graph
    /// 4. Builds the dependency graph for other commands
    /// 5. Sets up external usage checker if requested
    /// 6. Dispatches to the appropriate analysis method
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(TopCatError)` if:
    /// - Configuration loading/validation fails
    /// - Graph building fails (cycles, missing deps, config errors)
    /// - Analysis execution fails
    /// - External checker setup fails
    pub fn execute(&self) -> Result<(), TopCatError> {
        // 1. Load settings from all sources (config files, env vars)
        let config_path = self.common.config_path();
        let mut settings = Settings::load(config_path)
            .map_err(|e| TopCatError::ConfigError(format!("Failed to load configuration: {e}")))?;

        // 2. Apply CLI overrides
        self.common.apply_to_settings(&mut settings);

        // 3. Validate settings
        settings.validate().map_err(TopCatError::ConfigError)?;

        // 4. Ensure required fields are set
        if settings.input_dirs.is_empty() {
            return Err(TopCatError::ConfigError(
                "At least one input directory must be specified via -i/--input-dirs or config file"
                    .to_string(),
            ));
        }

        // 5. Initialize logging (unless quiet mode)
        let quiet = settings.behavior.quiet;
        if !quiet {
            let log_level = if settings.behavior.verbose {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            };
            Builder::new().filter(None, log_level).try_init().ok();
        }

        // 6. Extract schema filter for special case analyses
        let schema_filter: Vec<String> = self
            .common
            .schemas
            .clone()
            .unwrap_or_else(|| settings.schema_filtering.schemas.clone());

        // 7. Convert merge strategy to string for legacy API
        let merge_strategy_str =
            Self::merge_strategy_to_string(&settings.sql_discovery.merge_strategy);

        // 8. Convert config_path to Option<PathBuf> for legacy API
        let config_path_buf = config_path.map(PathBuf::from);

        // 9. For cycles and missing commands, we handle graph building specially
        // (They bypass full graph construction to catch errors that would prevent it)
        match &self.command {
            AnalyzeCommand::Cycles => {
                return cycles::analyze(
                    quiet,
                    &config_path_buf,
                    settings.sql_discovery.enabled,
                    &settings.sql_discovery.schema_pattern,
                    &merge_strategy_str,
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
                    &Some(settings.layers.names.join(",")),
                    &settings.layers.fallback,
                    &schema_filter,
                );
            }
            AnalyzeCommand::Missing => {
                return missing::analyze(
                    quiet,
                    &config_path_buf,
                    settings.sql_discovery.enabled,
                    &settings.sql_discovery.schema_pattern,
                    &merge_strategy_str,
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
                    &Some(settings.layers.names.join(",")),
                    &settings.layers.fallback,
                    &schema_filter,
                );
            }
            _ => {}
        }

        // 8. Build the dependency graph (for all other commands)
        let graph = self.build_graph(&settings)?;

        // 9. Check for external usage if requested
        let external_checker = self.build_external_checker(&settings)?;

        // 10. Build root matcher from settings
        let root_matcher = self.build_root_matcher(&settings)?;

        // 11. Execute the requested analysis
        match &self.command {
            AnalyzeCommand::DeadBranches => dead_branches::analyze(
                quiet,
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
            ),
            AnalyzeCommand::Orphans => orphans::analyze(quiet, &graph, external_checker.as_ref()),
            AnalyzeCommand::Unrequired => {
                unrequired::analyze(quiet, &graph, external_checker.as_ref())
            }
            AnalyzeCommand::LeafNodes => {
                leaf_nodes::analyze(quiet, &graph, external_checker.as_ref())
            }
            AnalyzeCommand::RootNodes => root_nodes::analyze(quiet, &graph),
            AnalyzeCommand::File { path } => {
                file::analyze(quiet, &graph, path, external_checker.as_ref())
            }
            // Cycles and Missing are handled earlier
            AnalyzeCommand::Cycles | AnalyzeCommand::Missing => unreachable!(),
        }
    }

    /// Build the dependency graph from Settings.
    ///
    /// Delegates to the common implementation for graph building from Settings.
    fn build_graph(&self, settings: &Settings) -> Result<topcat::file_dag::TCGraph, TopCatError> {
        cmd_common::build_graph_from_settings(&self.common.schemas, settings)
    }

    /// Build a root node matcher from Settings.
    ///
    /// Delegates to the common implementation for root matcher building from Settings.
    fn build_root_matcher(
        &self,
        settings: &Settings,
    ) -> Result<Option<RootNodeMatcher>, TopCatError> {
        cmd_common::build_root_matcher_from_settings(settings)
    }

    /// Build an external usage checker from Settings.
    ///
    /// Delegates to the common implementation for external checker building from Settings.
    fn build_external_checker(
        &self,
        settings: &Settings,
    ) -> Result<Option<topcat::analysis::external_usage::ExternalUsageChecker>, TopCatError> {
        cmd_common::build_external_checker_from_settings(settings)
    }

    /// Convert MergeStrategy to string for legacy APIs.
    ///
    /// This is a workaround until all code uses Settings directly.
    fn merge_strategy_to_string(strategy: &topcat::sql_config::MergeStrategy) -> String {
        use topcat::sql_config::MergeStrategy;
        match strategy {
            MergeStrategy::HeaderOnly => "header-only".to_string(),
            MergeStrategy::DiscoveryOnly => "discovery-only".to_string(),
            MergeStrategy::Union => "union".to_string(),
            MergeStrategy::HeaderWithFallback => "header-with-fallback".to_string(),
            MergeStrategy::Validate => "validate".to_string(),
        }
    }
}
