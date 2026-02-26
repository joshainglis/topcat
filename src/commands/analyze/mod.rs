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

use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::cli::{AnalysisArgs as AnalysisCliArgs, FilterArgs, GlobalArgs, GraphInputArgs};
use topcat::exceptions::TopCatError;
use topcat::settings::Settings;

use super::common as cmd_common;

/// Command-line arguments for the analyze subcommand.
///
/// The analyze command uses:
/// - GlobalArgs: config, verbose, quiet
/// - GraphInputArgs: input directories, file filtering, layers
/// - FilterArgs: schema filtering
/// - AnalysisArgs: root patterns, external usage checking
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(flatten)]
    pub input: GraphInputArgs,

    #[command(flatten)]
    pub filter: FilterArgs,

    #[command(flatten)]
    pub analysis: AnalysisCliArgs,

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
        // 1-2. Load settings and apply CLI overrides
        let settings =
            cmd_common::load_settings_with_overrides(self.global.config_path(), |settings| {
                self.global.apply_to_settings(settings);
                self.input.apply_to_settings(settings);
                self.filter.apply_to_settings(settings);
                self.analysis.apply_to_settings(settings);
            })?;

        // 3-4. Validate settings
        cmd_common::validate_settings(&settings, true)?;

        // 5-6. Initialize logging and create logger
        let logger = cmd_common::init_logger_from_settings(&settings);

        // 7. Get schema filter
        let schema_filter = self.filter.get_schemas(&settings);
        let schema_filter_opt = cmd_common::vec_to_option(&schema_filter);

        // 8. For cycles and missing commands, we handle graph building specially
        // (They bypass full graph construction to catch errors that would prevent it)
        match &self.command {
            AnalyzeCommand::Cycles => {
                return cycles::analyze(&logger, &schema_filter_opt, &settings);
            }
            AnalyzeCommand::Missing => {
                return missing::analyze(&logger, &schema_filter_opt, &settings);
            }
            _ => {}
        }

        // 9. Build the dependency graph (for all other commands)
        let graph = self.build_graph(&settings)?;

        // 10. Check for external usage if requested
        let external_checker = self.build_external_checker(&settings)?;

        // 11. Build root matcher from settings
        let root_matcher = self.build_root_matcher(&settings)?;

        // 12. Execute the requested analysis
        match &self.command {
            AnalyzeCommand::DeadBranches => dead_branches::analyze(
                &logger,
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                settings.analysis.protect_implicit,
            ),
            AnalyzeCommand::Orphans => orphans::analyze(
                &logger,
                &graph,
                external_checker.as_ref(),
                settings.analysis.protect_implicit,
            ),
            AnalyzeCommand::Unrequired => {
                unrequired::analyze(&logger, &graph, external_checker.as_ref())
            }
            AnalyzeCommand::LeafNodes => {
                leaf_nodes::analyze(&logger, &graph, external_checker.as_ref())
            }
            AnalyzeCommand::RootNodes => root_nodes::analyze(&logger, &graph),
            AnalyzeCommand::File { path } => {
                file::analyze(&logger, &graph, path, external_checker.as_ref())
            }
            // Cycles and Missing are handled earlier
            AnalyzeCommand::Cycles | AnalyzeCommand::Missing => unreachable!(),
        }
    }

    /// Build the dependency graph from Settings.
    ///
    /// Delegates to the common implementation for graph building from Settings.
    fn build_graph(&self, settings: &Settings) -> Result<topcat::file_dag::TCGraph, TopCatError> {
        let schema_filter = self.filter.get_schemas(settings);
        let schema_filter_opt = cmd_common::vec_to_option(&schema_filter);
        cmd_common::build_graph_from_settings(&schema_filter_opt, settings)
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
}
