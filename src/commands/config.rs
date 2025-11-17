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

use topcat::cli::CommonArgs;
use topcat::exceptions::TopCatError;
use topcat::settings::Settings;

/// Command-line arguments for the config subcommand.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(flatten)]
    pub common: CommonArgs,

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
        let config_path = self.common.config_path();
        let mut settings = Settings::load(config_path)
            .map_err(|e| TopCatError::ConfigError(format!("Failed to load configuration: {e}")))?;

        // Apply CLI overrides
        self.common.apply_to_settings(&mut settings);

        // Validate
        settings.validate().map_err(TopCatError::ConfigError)?;

        // Serialize to TOML for display
        let toml_str = toml::to_string_pretty(&settings)
            .map_err(|e| TopCatError::ConfigError(format!("Failed to serialize config: {e}")))?;

        println!("# Effective Configuration");
        println!("# (merged from all sources: CLI, env vars, config files, defaults)\n");
        println!("{toml_str}");

        Ok(())
    }

    /// Validate a configuration file.
    fn validate(&self) -> Result<(), TopCatError> {
        let config_path = self.common.config_path();

        if config_path.is_none() {
            eprintln!(
                "⚠️  No config file specified. Use --config <path> to validate a specific file."
            );
            eprintln!("   Checking default locations...\n");
        }

        match Settings::load(config_path) {
            Ok(settings) => match settings.validate() {
                Ok(()) => {
                    let source = if let Some(path) = config_path {
                        format!("Config file: {path}")
                    } else {
                        "Default configuration".to_string()
                    };
                    println!("✅ Configuration is valid");
                    println!("   Source: {source}");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("❌ Configuration validation failed:");
                    eprintln!("   {e}");
                    Err(TopCatError::ConfigError(e))
                }
            },
            Err(e) => {
                eprintln!("❌ Failed to load configuration:");
                eprintln!("   {e}");
                Err(TopCatError::ConfigError(format!("{e}")))
            }
        }
    }

    /// Generate an example configuration file.
    fn generate(&self) -> Result<(), TopCatError> {
        let example = include_str!("../../topcat.toml.example");
        print!("{example}");
        Ok(())
    }
}
