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

use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::cli::CommonArgs;
use topcat::exceptions::TopCatError;
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
        let mut settings = Settings::load(config_path).map_err(|e| {
            TopCatError::ConfigError(format!("Failed to load configuration: {}", e))
        })?;

        // 2. Apply CLI overrides
        self.common.apply_to_settings(&mut settings);

        // 3. Validate settings
        settings
            .validate()
            .map_err(|e| TopCatError::ConfigError(e))?;

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

        // 6. Determine if we're actually deleting or just previewing
        let actually_delete = self.common.no_dry_run || !settings.behavior.dry_run;

        if actually_delete {
            println!("⚠️  DELETION MODE: Files will be permanently removed!");
        } else {
            println!("🔍 DRY-RUN MODE: No files will be deleted");
        }

        // 7. Initialize logging
        if !settings.behavior.quiet {
            let log_level = if settings.behavior.verbose {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            };
            Builder::new().filter(None, log_level).try_init().ok();
        }

        // 8. Build the dependency graph
        let graph = self.build_graph(&settings)?;

        // 9. Check for external usage if requested
        let external_checker = self.build_external_checker(&settings)?;

        // 10. Build root matcher from settings
        let root_matcher = self.build_root_matcher(&settings)?;

        // 11. Extract force and verbose from settings
        let force = settings.behavior.force;
        let verbose = settings.behavior.verbose;

        // 12. Execute the requested cleanup
        match &self.command {
            CleanCommand::DeadBranches => dead_branches::clean(
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
                force,
                verbose,
            ),
            CleanCommand::Orphans => orphans::clean(
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
                force,
                verbose,
            ),
            CleanCommand::Unrequired => unrequired::clean(
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
                force,
                verbose,
            ),
            CleanCommand::Targets { files } => targets::clean(
                &graph,
                files,
                root_matcher.as_ref(),
                actually_delete,
                force,
                verbose,
            ),
        }
    }

    /// Build the dependency graph from Settings.
    ///
    /// Constructs a `TCGraph` using the unified Settings configuration.
    fn build_graph(&self, settings: &Settings) -> Result<topcat::file_dag::TCGraph, TopCatError> {
        // Extract schema filter for node prefixes
        let schema_filter: Vec<String> = self
            .common
            .schemas
            .clone()
            .unwrap_or_else(|| settings.schema_filtering.schemas.clone());

        let include_node_prefixes = if schema_filter.is_empty() {
            None
        } else {
            cmd_common::build_schema_filter(&schema_filter).to_option()
        };

        // Get fallback layer (required)
        let fallback_layer = settings.layers.fallback.clone().ok_or_else(|| {
            TopCatError::ConfigError("Fallback layer must be specified".to_string())
        })?;

        cmd_common::build_graph(
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

    /// Build a root node matcher from Settings.
    ///
    /// Root matchers identify which nodes should be protected from deletion.
    fn build_root_matcher(
        &self,
        settings: &Settings,
    ) -> Result<Option<RootNodeMatcher>, TopCatError> {
        let root_nodes = settings.analysis.root_nodes.clone();
        let root_patterns = settings.analysis.root_patterns.clone();
        let root_regex = settings.analysis.root_regex.clone();
        let root_dirs: Vec<PathBuf> = settings
            .analysis
            .root_dirs
            .iter()
            .map(|s| PathBuf::from(s))
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

    /// Build an external usage checker from Settings.
    ///
    /// Sets up external usage checking if configured in settings.
    fn build_external_checker(
        &self,
        settings: &Settings,
    ) -> Result<Option<topcat::analysis::external_usage::ExternalUsageChecker>, TopCatError> {
        let dirs: Vec<PathBuf> = settings
            .analysis
            .external_check_dirs
            .iter()
            .map(|s| PathBuf::from(s))
            .collect();
        let patterns = settings.analysis.external_check_patterns.clone();

        cmd_common::build_external_checker(&dirs, &patterns, settings.behavior.verbose)
    }
}
