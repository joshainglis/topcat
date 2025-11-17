use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for SQL dependency discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlDiscoveryConfig {
    /// Enable SQL dependency discovery
    #[serde(default)]
    pub enabled: bool,

    /// Regex pattern for matching schema.object references
    /// Example: `"(?:test|e|c|d\[pio\]|codegen|md)_\\w+"`
    #[serde(default)]
    pub schema_pattern: Option<String>,

    /// Regex pattern for matching object names
    /// Default matches standard SQL identifiers
    #[serde(default)]
    pub object_pattern: Option<String>,

    /// Type mappings for SQL type transformations
    /// Maps from SQL type to node name
    /// Example: {"TSTZRANGE": "c_tmf.t_time_period"}
    #[serde(default)]
    pub type_mappings: HashMap<String, String>,

    /// Extension mappings for PostgreSQL extensions
    /// Maps object name to extension name
    /// Example: {"digest": "pgcrypto", "nlevel": "ltree"}
    #[serde(default)]
    pub extension_mappings: HashMap<String, String>,

    /// Object name suffixes to strip
    /// Example: ["_or_ref"]
    #[serde(default)]
    pub strip_suffixes: Vec<String>,

    /// Patterns for model generation procedure calls
    /// Example: "codegen_tmf\\.proc_(?:make_model|combine_enums)"
    #[serde(default)]
    pub model_gen_patterns: Vec<String>,

    /// How to merge discovered dependencies with manual ones
    #[serde(default)]
    pub merge_strategy: MergeStrategy,
}

impl Default for SqlDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            schema_pattern: None,
            object_pattern: None,
            type_mappings: HashMap::new(),
            extension_mappings: HashMap::new(),
            strip_suffixes: Vec::new(),
            model_gen_patterns: Vec::new(),
            merge_strategy: MergeStrategy::DiscoveryOnly,
        }
    }
}

/// Strategy for merging manual and discovered dependencies
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MergeStrategy {
    /// Use only header dependencies (ignore discovered)
    HeaderOnly,
    /// Use only discovered dependencies (default)
    #[default]
    DiscoveryOnly,
    /// Combine both (union)
    Union,
    /// Use header if present, otherwise use discovered
    HeaderWithFallback,
    /// Check that manual and discovered match, warn on discrepancies
    Validate,
}

impl std::str::FromStr for MergeStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "header-only" => Ok(MergeStrategy::HeaderOnly),
            "discovery-only" => Ok(MergeStrategy::DiscoveryOnly),
            "union" => Ok(MergeStrategy::Union),
            "header-with-fallback" => Ok(MergeStrategy::HeaderWithFallback),
            "validate" => Ok(MergeStrategy::Validate),
            _ => Err(format!("Invalid merge strategy: {s}")),
        }
    }
}

/// Header update mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeaderUpdateMode {
    /// Never modify files
    #[default]
    Never,
    /// Update files in-place
    InPlace,
    /// Generate new files with updated headers
    Generate,
}

/// Configuration for analysis features (root node protection, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Specific nodes to always treat as root/entry points
    #[serde(default)]
    pub root_nodes: Vec<String>,

    /// Glob patterns for root files
    #[serde(default)]
    pub root_patterns: Vec<String>,

    /// Regex patterns for root node names
    #[serde(default)]
    pub root_regex: Vec<String>,

    /// Directories where all files are considered roots
    #[serde(default)]
    pub root_dirs: Vec<String>,

    /// Directories to check for external usage of SQL objects
    #[serde(default)]
    pub external_check_dirs: Vec<String>,

    /// File patterns to check for external usage (e.g., "*.py", "*.ts")
    #[serde(default)]
    pub external_check_patterns: Vec<String>,

    /// Protect implicit nodes (CAST, OPERATOR) from dead-branch cleanup
    #[serde(default = "default_true")]
    pub protect_implicit: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            root_nodes: Vec::new(),
            root_patterns: Vec::new(),
            root_regex: Vec::new(),
            root_dirs: Vec::new(),
            external_check_dirs: Vec::new(),
            external_check_patterns: Vec::new(),
            protect_implicit: true,
        }
    }
}

/// Configuration for file filtering
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FiltersConfig {
    /// Glob patterns for files to include (e.g., "**/*.sql")
    #[serde(default)]
    pub include_globs: Vec<String>,

    /// Glob patterns for files to exclude (e.g., "**/test_*.sql")
    #[serde(default)]
    pub exclude_globs: Vec<String>,

    /// File extensions to include (without leading dot)
    #[serde(default)]
    pub include_extensions: Vec<String>,

    /// File extensions to exclude
    #[serde(default)]
    pub exclude_extensions: Vec<String>,

    /// Include hidden files and directories
    #[serde(default)]
    pub include_hidden: bool,
}

/// Configuration for layer ordering
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayersConfig {
    /// Layer names in execution order (e.g., ["prepend", "normal", "append"])
    #[serde(default)]
    pub names: Vec<String>,

    /// Default layer for files without explicit layer declaration
    #[serde(default)]
    pub fallback: Option<String>,
}

/// Full TOML configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopcatConfig {
    #[serde(default)]
    pub sql_discovery: SqlDiscoveryConfig,

    #[serde(default)]
    pub analysis: AnalysisConfig,

    #[serde(default)]
    pub filters: FiltersConfig,

    #[serde(default)]
    pub layers: LayersConfig,
}

impl TopcatConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: TopcatConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Merge CLI overrides into this configuration
    #[allow(dead_code)]
    pub fn with_overrides(mut self, overrides: &SqlDiscoveryConfig) -> Self {
        // Override only the fields that are explicitly set
        if overrides.enabled {
            self.sql_discovery.enabled = true;
        }
        if overrides.schema_pattern.is_some() {
            self.sql_discovery.schema_pattern = overrides.schema_pattern.clone();
        }
        if overrides.object_pattern.is_some() {
            self.sql_discovery.object_pattern = overrides.object_pattern.clone();
        }
        if !overrides.type_mappings.is_empty() {
            self.sql_discovery
                .type_mappings
                .extend(overrides.type_mappings.clone());
        }
        if !overrides.extension_mappings.is_empty() {
            self.sql_discovery
                .extension_mappings
                .extend(overrides.extension_mappings.clone());
        }
        if !overrides.strip_suffixes.is_empty() {
            self.sql_discovery
                .strip_suffixes
                .extend(overrides.strip_suffixes.clone());
        }
        if !overrides.model_gen_patterns.is_empty() {
            self.sql_discovery
                .model_gen_patterns
                .extend(overrides.model_gen_patterns.clone());
        }
        if overrides.merge_strategy != MergeStrategy::default() {
            self.sql_discovery.merge_strategy = overrides.merge_strategy;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            "union".parse::<MergeStrategy>().unwrap(),
            MergeStrategy::Union
        );
        assert_eq!(
            "validate".parse::<MergeStrategy>().unwrap(),
            MergeStrategy::Validate
        );
        assert!("invalid".parse::<MergeStrategy>().is_err());
    }

    #[test]
    fn test_default_config() {
        let config = SqlDiscoveryConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.merge_strategy, MergeStrategy::DiscoveryOnly);
    }

    #[test]
    fn test_config_with_overrides() {
        let base_config = TopcatConfig {
            sql_discovery: SqlDiscoveryConfig {
                enabled: false,
                schema_pattern: Some("old_pattern".to_string()),
                ..Default::default()
            },
            analysis: AnalysisConfig::default(),
            filters: FiltersConfig::default(),
            layers: LayersConfig::default(),
        };

        let overrides = SqlDiscoveryConfig {
            enabled: true,
            schema_pattern: Some("new_pattern".to_string()),
            ..Default::default()
        };

        let merged = base_config.with_overrides(&overrides);
        assert!(merged.sql_discovery.enabled);
        assert_eq!(
            merged.sql_discovery.schema_pattern,
            Some("new_pattern".to_string())
        );
    }
}
