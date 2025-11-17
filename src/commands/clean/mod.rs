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

use clap::{Args, Subcommand};

use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::cli::CommonArgs;
use topcat::exceptions::TopCatError;
use topcat::logging::{Logger, init_logging};
use topcat::settings::Settings;

use super::common as cmd_common;

/// Remove unused files based on dependency analysis
#[derive(Debug, Args)]
pub struct CleanArgs {
    #[command(flatten)]
    pub common: CommonArgs,

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
    /// 1. Loads configuration from all sources (files, env vars, CLI)
    /// 2. Initializes logging
    /// 3. Determines if actually deleting or dry-run (defaults to dry-run for clean)
    /// 4. Builds the dependency graph
    /// 5. Sets up external usage checker if requested
    /// 6. Builds root matcher for protection patterns
    /// 7. Dispatches to the appropriate clean operation
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(TopCatError)` if:
    /// - Configuration loading/validation fails
    /// - Graph building fails
    /// - External checker setup fails
    /// - Deletion fails
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

        // 5. SPECIAL CASE: Clean command defaults to dry-run=true
        // Apply clean-specific dry-run default if not explicitly set
        if !self.common.no_dry_run && self.common.dry_run.is_none() {
            settings.behavior.dry_run = true;
        }

        // 6. Initialize logging
        let quiet = settings.behavior.quiet;
        let verbose = settings.behavior.verbose;
        init_logging(verbose, quiet);

        // 7. Create logger instance
        let logger = Logger::new(quiet, verbose);

        // 8. Determine if we're actually deleting or just previewing
        let actually_delete = self.common.no_dry_run || !settings.behavior.dry_run;

        if actually_delete {
            logger.warn("⚠️  DELETION MODE: Files will be permanently removed!");
        } else {
            logger.info("🔍 DRY-RUN MODE: No files will be deleted");
        }

        // 9. Build the dependency graph
        let graph = self.build_graph(&settings)?;

        // 10. Check for external usage if requested
        let external_checker = self.build_external_checker(&settings)?;

        // 11. Build root matcher from settings
        let root_matcher = self.build_root_matcher(&settings)?;

        // 12. Extract force flag from settings
        let force = settings.behavior.force;

        // 13. Execute the requested cleanup
        match &self.command {
            CleanCommand::DeadBranches => dead_branches::clean(
                &logger,
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
                force,
            ),
            CleanCommand::Orphans => orphans::clean(
                &logger,
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
                force,
            ),
            CleanCommand::Unrequired => unrequired::clean(
                &logger,
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
                force,
            ),
            CleanCommand::Targets { files } => targets::clean(
                &logger,
                &graph,
                files,
                root_matcher.as_ref(),
                actually_delete,
                force,
            ),
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
}
