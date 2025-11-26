//! Configuration management commands.
//!
//! This module provides commands for inspecting, validating, and generating
//! Topcat configuration files.
//!
//! # Available Commands
//!
//! - **show**: Display the effective configuration from all sources
//! - **validate**: Validate a configuration file
//! - **generate**: Generate an example configuration file
//!
//! # Examples
//!
//! ```bash
//! # Show effective configuration
//! topcat config show
//!
//! # Validate a specific config file
//! topcat config validate --config topcat.toml
//!
//! # Generate example config
//! topcat config generate > topcat.toml
//! ```

use clap::{Args, Subcommand};

use topcat::cli::GlobalArgs;
use topcat::exceptions::TopCatError;
use topcat::logging::{Logger, init_logging};
use topcat::settings::Settings;

/// Command-line arguments for the config subcommand.
///
/// The config command only needs global arguments (--config, --verbose, --quiet)
/// since it operates on configuration itself, not on the dependency graph.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Display the effective configuration from all sources
    ///
    /// Shows the final merged configuration after combining:
    /// - CLI arguments
    /// - Environment variables
    /// - Config files (project, user, system)
    /// - Default values
    Show,

    /// Validate a configuration file
    ///
    /// Loads and validates the config file, reporting any errors.
    /// Exit code 0 if valid, 1 if invalid.
    Validate,

    /// Generate an example configuration file
    ///
    /// Outputs a comprehensive example topcat.toml to stdout with
    /// all available options documented.
    Generate,
}

impl ConfigArgs {
    /// Execute the config command.
    pub fn execute(&self) -> Result<(), TopCatError> {
        match &self.command {
            ConfigCommand::Show => self.show(),
            ConfigCommand::Validate => self.validate(),
            ConfigCommand::Generate => self.generate(),
        }
    }

    /// Display the effective configuration.
    fn show(&self) -> Result<(), TopCatError> {
        let config_path = self.global.config_path();
        let mut settings = Settings::load(config_path)
            .map_err(|e| TopCatError::ConfigError(format!("Failed to load configuration: {e}")))?;

        // Apply CLI overrides
        self.global.apply_to_settings(&mut settings);

        // Validate
        settings.validate().map_err(TopCatError::ConfigError)?;

        // Initialize logging after settings are loaded
        let quiet = settings.behavior.quiet;
        let verbose = settings.behavior.verbose;
        init_logging(verbose, quiet);
        let logger = Logger::new(quiet, verbose);

        // Serialize to TOML for display
        let toml_str = toml::to_string_pretty(&settings)
            .map_err(|e| TopCatError::ConfigError(format!("Failed to serialize config: {e}")))?;

        logger.info("# Effective Configuration");
        logger.info("# (merged from all sources: CLI, env vars, config files, defaults)\n");
        logger.info(&toml_str);

        Ok(())
    }

    /// Validate a configuration file.
    fn validate(&self) -> Result<(), TopCatError> {
        let config_path = self.global.config_path();

        // Initialize logging with defaults (no settings loaded yet for validate command)
        init_logging(false, false);
        let logger = Logger::new(false, false);

        if config_path.is_none() {
            logger.warn(
                "⚠️  No config file specified. Use --config <path> to validate a specific file.",
            );
            logger.info("   Checking default locations...\n");
        }

        match Settings::load(config_path) {
            Ok(settings) => match settings.validate() {
                Ok(()) => {
                    let source = if let Some(path) = config_path {
                        format!("Config file: {path}")
                    } else {
                        "Default configuration".to_string()
                    };
                    logger.success("✅ Configuration is valid");
                    logger.info(&format!("   Source: {source}"));
                    Ok(())
                }
                Err(e) => {
                    logger.error("❌ Configuration validation failed:");
                    logger.error(&format!("   {e}"));
                    Err(TopCatError::ConfigError(e))
                }
            },
            Err(e) => {
                logger.error("❌ Failed to load configuration:");
                logger.error(&format!("   {e}"));
                Err(TopCatError::ConfigError(format!("{e}")))
            }
        }
    }

    /// Generate an example configuration file.
    fn generate(&self) -> Result<(), TopCatError> {
        // Note: We use raw print! here (not logger) because users typically
        // redirect this output to a file: `topcat config generate > topcat.toml`
        // Using the logger would add unwanted formatting/colors to the output file.
        let example = include_str!("../../topcat.toml.example");
        print!("{example}");
        Ok(())
    }
}
