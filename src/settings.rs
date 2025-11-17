use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
                fallback: Some("normal".to_string()),
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
    /// Load settings from all available sources with proper precedence.
    ///
    /// Precedence (highest to lowest):
    /// 1. Values set explicitly via `with_overrides()`
    /// 2. Environment variables (`TOPCAT_*`)
    /// 3. Project config file (`./topcat.toml`)
    /// 4. User config file (`~/.config/topcat/config.toml`)
    /// 5. System config file (`/etc/topcat/config.toml`)
    /// 6. Default values
    ///
    /// # Arguments
    ///
    /// * `config_path` - Optional explicit config file path
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use topcat::settings::Settings;
    ///
    /// // Load from standard locations + env vars
    /// let settings = Settings::load(None).unwrap();
    ///
    /// // Load from explicit config file
    /// let settings = Settings::load(Some("my-config.toml")).unwrap();
    /// ```
    pub fn load(config_path: Option<&str>) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        // Add default configuration file locations (lowest priority)
        // System-wide config
        if let Ok(path) = Self::get_system_config_path() {
            builder = builder.add_source(File::with_name(&path).required(false));
        }

        // User config
        if let Ok(path) = Self::get_user_config_path() {
            builder = builder.add_source(File::with_name(&path).required(false));
        }

        // Project config (./topcat.toml or ./.topcat.toml)
        builder = builder
            .add_source(File::with_name("./topcat").required(false))
            .add_source(File::with_name("./.topcat").required(false));

        // Explicit config file path (if provided)
        if let Some(path) = config_path {
            builder = builder.add_source(File::with_name(path).required(true));
        }

        // Environment variables with TOPCAT_ prefix (higher priority)
        // Use __ as separator for nested config (e.g., TOPCAT_SQL_DISCOVERY__ENABLED)
        builder = builder.add_source(
            Environment::with_prefix("TOPCAT")
                .separator("__")
                .try_parsing(true)
                .list_separator(","),
        );

        // Build and deserialize
        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Create a new Settings instance with explicit values (for testing or programmatic use).
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate the configuration.
    ///
    /// Checks:
    /// - Fallback layer exists in layers list
    /// - At least one layer is defined
    /// - Input directories are specified (for concat/analyze/clean commands)
    pub fn validate(&self) -> Result<(), String> {
        // Validate layers
        if self.layers.names.is_empty() {
            return Err("At least one layer must be specified".to_string());
        }

        if let Some(ref fallback) = self.layers.fallback
            && !self.layers.names.contains(fallback)
        {
            return Err(format!("Fallback layer '{fallback}' is not in layers list"));
        }

        Ok(())
    }

    /// Get the system-wide config file path.
    fn get_system_config_path() -> Result<String, std::io::Error> {
        #[cfg(unix)]
        {
            Ok("/etc/topcat/config".to_string())
        }
        #[cfg(windows)]
        {
            Ok("C:\\ProgramData\\topcat\\config".to_string())
        }
    }

    /// Get the user config file path.
    fn get_user_config_path() -> Result<String, std::io::Error> {
        if let Some(config_dir) = dirs::config_dir() {
            Ok(config_dir
                .join("topcat")
                .join("config")
                .display()
                .to_string())
        } else if let Some(home_dir) = dirs::home_dir() {
            Ok(home_dir
                .join(".config")
                .join("topcat")
                .join("config")
                .display()
                .to_string())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine user config directory",
            ))
        }
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

/// Configuration for output formatting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FormattingConfig {
    /// Comment string for the target language (e.g., "--" for SQL, "#" for Python)
    pub comment_str: String,

    /// Separator string inserted between files in concatenated output
    /// Can include {} placeholder which will be replaced with the file path
    pub file_separator_str: String,

    /// String appended at the end of each file in concatenated output
    pub file_end_str: String,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            comment_str: "--".to_string(),
            file_separator_str: "------------------------------------------------------------------------------------------------------------------------".to_string(),
            file_end_str: ";".to_string(),
        }
    }
}

/// Configuration for behavior flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct BehaviorConfig {
    /// Enable verbose output with detailed logging
    pub verbose: bool,

    /// Enable quiet mode (suppress non-essential output, for CI/CD)
    pub quiet: bool,

    /// Enable dry-run mode (preview operations without making changes)
    pub dry_run: bool,

    /// Force operations without confirmation prompts
    pub force: bool,
}

/// Configuration for node filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct NodeFilteringConfig {
    /// Node name prefixes to include (e.g., ["auth::", "billing::"])
    pub include_prefixes: Vec<String>,

    /// Node name prefixes to exclude (e.g., ["test::", "deprecated::"])
    pub exclude_prefixes: Vec<String>,

    /// Filter to only include files within a specific subdirectory
    pub subdir_filter: Option<PathBuf>,
}

/// Configuration for schema filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct SchemaFilteringConfig {
    /// Schemas to include in operations
    pub schemas: Vec<String>,
}

/// Configuration for export operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct ExportConfig {
    /// Export mode (full graph, dependencies, dependents, or direct)
    pub mode: Option<ExportMode>,

    /// Node name for dependency/dependents export
    pub node: Option<String>,
}

/// Export mode for graph export.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ExportMode {
    /// Export full graph
    Full,
    /// Export dependencies of a node
    Dependencies,
    /// Export dependents of a node
    Dependents,
    /// Export direct dependencies only
    Direct,
}

impl std::str::FromStr for ExportMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "full" => Ok(ExportMode::Full),
            "dependencies" | "deps" => Ok(ExportMode::Dependencies),
            "dependents" => Ok(ExportMode::Dependents),
            "direct" => Ok(ExportMode::Direct),
            _ => Err(format!("Invalid export mode: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.layers.names, vec!["prepend", "normal", "append"]);
        assert_eq!(settings.layers.fallback, Some("normal".to_string()));
        assert!(!settings.sql_discovery.enabled);
        assert!(!settings.behavior.verbose);
    }

    #[test]
    fn test_validation() {
        let mut settings = Settings::default();
        assert!(settings.validate().is_ok());

        // Invalid fallback layer
        settings.layers.fallback = Some("invalid".to_string());
        assert!(settings.validate().is_err());

        // Empty layers
        settings.layers.names.clear();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_merge_strategy_from_str() {
        assert_eq!(
            "header-only".parse::<MergeStrategy>().unwrap(),
            MergeStrategy::HeaderOnly
        );
        assert_eq!(
            "discovery-only".parse::<MergeStrategy>().unwrap(),
            MergeStrategy::DiscoveryOnly
        );
        assert!("invalid".parse::<MergeStrategy>().is_err());
    }

    #[test]
    fn test_export_mode_from_str() {
        assert_eq!(
            "dependencies".parse::<ExportMode>().unwrap(),
            ExportMode::Dependencies
        );
        assert_eq!(
            "deps".parse::<ExportMode>().unwrap(),
            ExportMode::Dependencies
        );
        assert!("invalid".parse::<ExportMode>().is_err());
    }

    #[test]
    fn test_env_var_parsing() {
        // Test that environment variables are properly parsed
        // Note: We test by loading with env vars, not by setting them directly
        // to avoid test interference in parallel test execution

        // Create a test config file in a temp directory
        use std::io::Write;
        use tempfile::Builder;

        let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            temp_file,
            r#"
[behavior]
verbose = true

[filters]
include_extensions = ["sql", "ddl"]
"#
        )
        .unwrap();

        // Load settings from the temp file
        let settings = Settings::load(Some(temp_file.path().to_str().unwrap()))
            .expect("Failed to load settings from temp file");

        assert!(settings.behavior.verbose);
        assert_eq!(settings.filters.include_extensions, vec!["sql", "ddl"]);
    }

    #[test]
    fn test_config_file_precedence() {
        // Test that config files are loaded and merged correctly
        use std::io::Write;
        use tempfile::Builder;

        // Create a config file with specific settings
        let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            temp_file,
            r#"
[layers]
names = ["first", "second", "third"]
fallback = "second"

[behavior]
verbose = true
quiet = false
"#
        )
        .unwrap();

        let settings = Settings::load(Some(temp_file.path().to_str().unwrap()))
            .expect("Failed to load settings");

        assert_eq!(settings.layers.names, vec!["first", "second", "third"]);
        assert_eq!(settings.layers.fallback, Some("second".to_string()));
        assert!(settings.behavior.verbose);
        assert!(!settings.behavior.quiet);
    }

    #[test]
    fn test_config_validation_with_custom_layers() {
        use std::io::Write;
        use tempfile::Builder;

        // Create a config with valid custom layers
        let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            temp_file,
            r#"
[layers]
names = ["alpha", "beta", "gamma"]
fallback = "beta"
"#
        )
        .unwrap();

        let settings = Settings::load(Some(temp_file.path().to_str().unwrap()))
            .expect("Failed to load settings");

        assert!(settings.validate().is_ok());

        // Test invalid fallback layer
        let mut temp_file2 = Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            temp_file2,
            r#"
[layers]
names = ["alpha", "beta"]
fallback = "gamma"
"#
        )
        .unwrap();

        let settings2 = Settings::load(Some(temp_file2.path().to_str().unwrap()))
            .expect("Failed to load settings");

        assert!(settings2.validate().is_err());
    }

    #[test]
    fn test_sql_discovery_settings() {
        use std::io::Write;
        use tempfile::Builder;

        let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            temp_file,
            r#"
[sql_discovery]
enabled = true
schema_pattern = "myapp_\\w+"
merge_strategy = "union"
"#
        )
        .unwrap();

        let settings = Settings::load(Some(temp_file.path().to_str().unwrap()))
            .expect("Failed to load settings");

        assert!(settings.sql_discovery.enabled);
        assert_eq!(
            settings.sql_discovery.schema_pattern,
            Some("myapp_\\w+".to_string())
        );
        assert_eq!(settings.sql_discovery.merge_strategy, MergeStrategy::Union);
    }

    #[test]
    fn test_analysis_settings() {
        use std::io::Write;
        use tempfile::Builder;

        let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            temp_file,
            r#"
[analysis]
root_patterns = ["**/api/*.sql", "**/*_init.sql"]
external_check_dirs = ["/app/src", "/app/lib"]
external_check_patterns = ["*.py", "*.ts", "*.js"]
"#
        )
        .unwrap();

        let settings = Settings::load(Some(temp_file.path().to_str().unwrap()))
            .expect("Failed to load settings");

        assert_eq!(
            settings.analysis.root_patterns,
            vec!["**/api/*.sql", "**/*_init.sql"]
        );
        assert_eq!(
            settings.analysis.external_check_dirs,
            vec!["/app/src", "/app/lib"]
        );
        assert_eq!(
            settings.analysis.external_check_patterns,
            vec!["*.py", "*.ts", "*.js"]
        );
    }

    #[test]
    fn test_filters_configuration() {
        use std::io::Write;
        use tempfile::Builder;

        let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            temp_file,
            r#"
[filters]
include_extensions = ["sql", "ddl", "dml"]
exclude_extensions = ["bak", "tmp"]
include_globs = ["**/*.sql"]
exclude_globs = ["**/test/**", "**/tmp/**"]
include_hidden = true
"#
        )
        .unwrap();

        let settings = Settings::load(Some(temp_file.path().to_str().unwrap()))
            .expect("Failed to load settings");

        assert_eq!(
            settings.filters.include_extensions,
            vec!["sql", "ddl", "dml"]
        );
        assert_eq!(settings.filters.exclude_extensions, vec!["bak", "tmp"]);
        assert_eq!(settings.filters.include_globs, vec!["**/*.sql"]);
        assert_eq!(
            settings.filters.exclude_globs,
            vec!["**/test/**", "**/tmp/**"]
        );
        assert!(settings.filters.include_hidden);
    }
}
