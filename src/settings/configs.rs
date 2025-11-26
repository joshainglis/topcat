//! Configuration structs for various Topcat settings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
