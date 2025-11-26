//! Configuration and settings management for Topcat.
//!
//! This module provides the unified configuration system that combines settings from
//! multiple sources: default values, config files, environment variables, and CLI arguments.
//!
//! ## Configuration Precedence (highest to lowest)
//!
//! 1. CLI arguments
//! 2. Environment variables (`TOPCAT_*`)
//! 3. Project config file (`./topcat.toml`)
//! 4. User config file (`~/.config/topcat/config.toml`)
//! 5. System config file (`/etc/topcat/config.toml`)
//! 6. Default values

mod configs;
mod loading;
mod validation;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Re-export config structs
pub use configs::{BehaviorConfig, ExportConfig, ExportMode, FormattingConfig, NodeFilteringConfig, SchemaFilteringConfig};

// Re-export types from sql_config for convenience
pub use crate::sql_config::{
    AnalysisConfig, FiltersConfig, HeaderUpdateMode, LayersConfig, MergeStrategy,
    SqlDiscoveryConfig,
};

/// Main configuration struct for Topcat.
///
/// This struct combines all configuration options from various sources:
/// - Default values
/// - Configuration files (`/etc/topcat/config.toml`, `~/.config/topcat/config.toml`, `./topcat.toml`)
/// - Environment variables (prefix: `TOPCAT_`)
/// - CLI arguments (highest priority)
///
/// # Environment Variables
///
/// All configuration options can be set via environment variables with the `TOPCAT_` prefix:
/// - `TOPCAT_VERBOSE=true`
/// - `TOPCAT_INPUT_DIRS="/path/one,/path/two"`
/// - `TOPCAT_SQL_DISCOVERY__ENABLED=true`
/// - `TOPCAT_SQL_DISCOVERY__SCHEMA_PATTERN="myapp_\w+"`
/// - `TOPCAT_FILTERS__INCLUDE_EXTENSIONS="sql,ddl"`
///
/// Nested options use double underscore (`__`) as separator.
/// Array values are comma-separated.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Directories to search for input files
    pub input_dirs: Vec<PathBuf>,

    /// Output file path
    pub output: Option<PathBuf>,

    /// File filtering configuration
    pub filters: FiltersConfig,

    /// Layer configuration for hierarchical ordering
    pub layers: LayersConfig,

    /// SQL dependency discovery configuration
    pub sql_discovery: SqlDiscoveryConfig,

    /// Header update mode
    pub header_update_mode: HeaderUpdateMode,

    /// Output directory for updated headers (if different from source)
    pub header_output_dir: Option<PathBuf>,

    /// Rename files based on discovered node names
    pub rename_files: bool,

    /// Analysis configuration (root nodes, external usage checking)
    pub analysis: AnalysisConfig,

    /// Output formatting configuration
    pub formatting: FormattingConfig,

    /// Behavior flags
    pub behavior: BehaviorConfig,

    /// Node filtering
    pub node_filtering: NodeFilteringConfig,

    /// Schema filtering
    pub schema_filtering: SchemaFilteringConfig,

    /// Export-specific settings
    pub export: ExportConfig,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            input_dirs: Vec::new(),
            output: None,
            filters: FiltersConfig::default(),
            layers: LayersConfig {
                names: vec![
                    "prepend".to_string(),
                    "normal".to_string(),
                    "append".to_string(),
                ],
                fallback: "normal".to_string(),
                auto_mapping: Default::default(), // Empty IndexMap by default
            },
            sql_discovery: SqlDiscoveryConfig::default(),
            header_update_mode: HeaderUpdateMode::Never,
            header_output_dir: None,
            rename_files: false,
            analysis: AnalysisConfig::default(),
            formatting: FormattingConfig::default(),
            behavior: BehaviorConfig::default(),
            node_filtering: NodeFilteringConfig::default(),
            schema_filtering: SchemaFilteringConfig::default(),
            export: ExportConfig::default(),
        }
    }
}

impl Settings {
    /// Create a new Settings instance with explicit values (for testing or programmatic use).
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge CLI overrides into settings.
    ///
    /// This allows CLI arguments to override config file values.
    pub fn with_cli_overrides(mut self, cli: &impl CliOverrides) -> Self {
        cli.apply_to_settings(&mut self);
        self
    }
}

/// Trait for CLI argument structs to apply their values to Settings.
pub trait CliOverrides {
    fn apply_to_settings(&self, settings: &mut Settings);
}
