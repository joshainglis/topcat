use clap::{Args, ValueEnum};
use std::path::PathBuf;

use crate::settings::{HeaderUpdateMode, MergeStrategy, Settings};

// ============================================================================
// Global Arguments (used by ALL commands)
// ============================================================================

/// Global arguments shared by all commands.
///
/// These are truly universal options that apply to every Topcat command.
#[derive(Debug, Args, Clone, Default)]
pub struct GlobalArgs {
    /// Path to configuration file (TOML)
    #[arg(long = "config", value_name = "FILE", global = true)]
    pub config_file: Option<PathBuf>,

    /// Enable verbose output with detailed logging
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::SetTrue, global = true)]
    pub verbose: bool,

    /// Suppress non-essential output (for CI/CD)
    #[arg(short = 'q', long = "quiet", action = clap::ArgAction::SetTrue, global = true)]
    pub quiet: bool,
}

impl GlobalArgs {
    /// Get the config file path if specified.
    pub fn config_path(&self) -> Option<&str> {
        self.config_file.as_ref().and_then(|p| p.to_str())
    }

    /// Apply global arguments to settings.
    pub fn apply_to_settings(&self, settings: &mut Settings) {
        if self.verbose {
            settings.behavior.verbose = true;
        }
        if self.quiet {
            settings.behavior.quiet = true;
        }
    }
}

// ============================================================================
// Graph Input Arguments (commands that build dependency graphs)
// ============================================================================

/// Arguments for commands that read and process input files.
///
/// Used by: concat, update, analyze, clean, schema, export
#[derive(Debug, Args, Clone, Default)]
pub struct GraphInputArgs {
    /// Directories to search for input files
    #[arg(short = 'i', long = "input-dirs", value_name = "DIRS")]
    pub input_dirs: Option<Vec<PathBuf>>,

    /// Include only files with these extensions (without leading dot)
    #[arg(short = 'e', long = "include-exts", value_name = "EXTS")]
    pub include_extensions: Option<Vec<String>>,

    /// Exclude files with these extensions
    #[arg(short = 'E', long = "exclude-exts", value_name = "EXTS")]
    pub exclude_extensions: Option<Vec<String>>,

    /// Include only files matching glob pattern (relative to working directory)
    #[arg(short = 'g', long = "include-glob", value_name = "PATTERN")]
    pub include_globs: Option<Vec<String>>,

    /// Exclude files matching glob pattern (relative to working directory)
    #[arg(short = 'G', long = "exclude-glob", value_name = "PATTERN")]
    pub exclude_globs: Option<Vec<String>>,

    /// Include hidden files and directories
    #[arg(long = "include-hidden")]
    pub include_hidden: Option<bool>,

    /// Comma-separated list of layer names in execution order
    #[arg(long = "layers", value_name = "LAYERS")]
    pub layers: Option<String>,

    /// Default layer for nodes without explicit layer declaration
    #[arg(long = "fallback-layer", value_name = "LAYER")]
    pub fallback_layer: Option<String>,
}

impl GraphInputArgs {
    /// Apply graph input arguments to settings.
    pub fn apply_to_settings(&self, settings: &mut Settings) {
        if let Some(ref input_dirs) = self.input_dirs {
            settings.input_dirs = input_dirs.clone();
        }

        if let Some(ref exts) = self.include_extensions {
            settings.filters.include_extensions = exts.clone();
        }

        if let Some(ref exts) = self.exclude_extensions {
            settings.filters.exclude_extensions = exts.clone();
        }

        if let Some(ref globs) = self.include_globs {
            settings.filters.include_globs = globs.clone();
        }

        if let Some(ref globs) = self.exclude_globs {
            settings.filters.exclude_globs = globs.clone();
        }

        if let Some(hidden) = self.include_hidden {
            settings.filters.include_hidden = hidden;
        }

        if let Some(ref layers_str) = self.layers {
            settings.layers.names = layers_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        if let Some(ref fallback) = self.fallback_layer {
            settings.layers.fallback = fallback.clone();
        }
    }
}

// ============================================================================
// Filter Arguments (node/schema filtering for graph operations)
// ============================================================================

/// Arguments for filtering nodes and schemas in graph operations.
///
/// Used by: concat, analyze, clean, export
#[derive(Debug, Args, Clone, Default)]
pub struct FilterArgs {
    /// Include only nodes with these name prefixes
    #[arg(long = "include-prefix", value_name = "PREFIXES")]
    pub include_node_prefixes: Option<Vec<String>>,

    /// Exclude nodes with these name prefixes
    #[arg(long = "exclude-prefix", value_name = "PREFIXES")]
    pub exclude_node_prefixes: Option<Vec<String>>,

    /// Include only files from this subdirectory and their dependencies
    #[arg(long = "subdir-filter", value_name = "SUBDIR")]
    pub subdir_filter: Option<PathBuf>,

    /// Filter operations to specific schemas
    #[arg(long = "schema", value_name = "SCHEMAS")]
    pub schemas: Option<Vec<String>>,
}

impl FilterArgs {
    /// Apply filter arguments to settings.
    pub fn apply_to_settings(&self, settings: &mut Settings) {
        if let Some(ref prefixes) = self.include_node_prefixes {
            settings.node_filtering.include_prefixes = prefixes.clone();
        }

        if let Some(ref prefixes) = self.exclude_node_prefixes {
            settings.node_filtering.exclude_prefixes = prefixes.clone();
        }

        if let Some(ref subdir) = self.subdir_filter {
            settings.node_filtering.subdir_filter = Some(subdir.clone());
        }

        if let Some(ref schemas) = self.schemas {
            settings.schema_filtering.schemas = schemas.clone();
        }
    }

    /// Get schemas from args or fallback to settings.
    pub fn get_schemas(&self, settings: &Settings) -> Vec<String> {
        self.schemas
            .clone()
            .unwrap_or_else(|| settings.schema_filtering.schemas.clone())
    }
}

// ============================================================================
// Analysis Arguments (for analyze and clean commands)
// ============================================================================

/// Arguments for dependency analysis operations.
///
/// Used by: analyze, clean
#[derive(Debug, Args, Clone, Default)]
pub struct AnalysisArgs {
    /// Specific nodes to always treat as root/entry points
    #[arg(long = "root-nodes", value_name = "NODES")]
    pub root_nodes: Option<Vec<String>>,

    /// Glob patterns for root files
    #[arg(long = "root-pattern", value_name = "PATTERNS")]
    pub root_patterns: Option<Vec<String>>,

    /// Regex patterns for root node names
    #[arg(long = "root-regex", value_name = "PATTERNS")]
    pub root_regex: Option<Vec<String>>,

    /// Directories where all files are considered roots
    #[arg(long = "root-dir", value_name = "DIRS")]
    pub root_dirs: Option<Vec<String>>,

    /// Directories to check for external usage of SQL objects
    #[arg(long = "external-check-dir", value_name = "DIRS")]
    pub external_check_dirs: Option<Vec<PathBuf>>,

    /// File patterns to check for external usage (e.g., '*.py', '*.ts')
    #[arg(long = "external-check-pattern", value_name = "PATTERNS")]
    pub external_check_patterns: Option<Vec<String>>,

    /// Don't protect implicit nodes (CAST, OPERATOR) from cleanup
    #[arg(long = "no-protect-implicit", action = clap::ArgAction::SetTrue)]
    pub no_protect_implicit: bool,
}

impl AnalysisArgs {
    /// Apply analysis arguments to settings.
    pub fn apply_to_settings(&self, settings: &mut Settings) {
        if let Some(ref nodes) = self.root_nodes {
            settings.analysis.root_nodes = nodes.clone();
        }

        if let Some(ref patterns) = self.root_patterns {
            settings.analysis.root_patterns = patterns.clone();
        }

        if let Some(ref patterns) = self.root_regex {
            settings.analysis.root_regex = patterns.clone();
        }

        if let Some(ref dirs) = self.root_dirs {
            settings.analysis.root_dirs = dirs.clone();
        }

        if let Some(ref dirs) = self.external_check_dirs {
            settings.analysis.external_check_dirs =
                dirs.iter().map(|p| p.display().to_string()).collect();
        }

        if let Some(ref patterns) = self.external_check_patterns {
            settings.analysis.external_check_patterns = patterns.clone();
        }

        if self.no_protect_implicit {
            settings.analysis.protect_implicit = false;
        }
    }
}

// ============================================================================
// SQL Discovery Arguments (for update command)
// ============================================================================

/// Arguments for SQL dependency discovery.
///
/// Used by: update
#[derive(Debug, Args, Clone, Default)]
pub struct SqlDiscoveryArgs {
    /// Enable automatic dependency discovery from SQL content
    #[arg(long = "enable-sql-discovery")]
    pub enable_sql_discovery: Option<bool>,

    /// Regex pattern for matching schema names (e.g., '(?:schema1|schema2)_\\w+')
    #[arg(long = "schema-pattern", value_name = "PATTERN")]
    pub schema_pattern: Option<String>,

    /// How to merge discovered and manual dependencies
    #[arg(
        long = "merge-strategy",
        value_parser = ["header-only", "discovery-only", "union", "header-with-fallback", "validate"],
        value_name = "STRATEGY"
    )]
    pub merge_strategy: Option<String>,

    /// Update files in-place with discovered dependencies
    #[arg(long = "update-headers")]
    pub update_headers: Option<bool>,

    /// Generate files with updated headers in this directory
    #[arg(long = "generate-headers", value_name = "DIR")]
    pub generate_headers_dir: Option<PathBuf>,

    /// Rename files based on discovered node names (requires --update-headers or --generate-headers)
    #[arg(long = "rename-files")]
    pub rename_files: Option<bool>,

    /// Comment string for the target language (e.g., '--' for SQL)
    #[arg(short = 'c', long = "comment-prefix", value_name = "PREFIX")]
    pub comment_str: Option<String>,
}

impl SqlDiscoveryArgs {
    /// Apply SQL discovery arguments to settings.
    pub fn apply_to_settings(&self, settings: &mut Settings) {
        if let Some(enabled) = self.enable_sql_discovery {
            settings.sql_discovery.enabled = enabled;
        }

        if let Some(ref pattern) = self.schema_pattern {
            settings.sql_discovery.schema_pattern = Some(pattern.clone());
        }

        if let Some(ref strategy) = self.merge_strategy
            && let Ok(parsed) = strategy.parse::<MergeStrategy>()
        {
            settings.sql_discovery.merge_strategy = parsed;
        }

        if let Some(update_headers) = self.update_headers {
            settings.header_update_mode = if update_headers {
                HeaderUpdateMode::InPlace
            } else {
                HeaderUpdateMode::Never
            };
        }

        if let Some(ref dir) = self.generate_headers_dir {
            settings.header_update_mode = HeaderUpdateMode::Generate;
            settings.header_output_dir = Some(dir.clone());
        }

        if let Some(rename_files) = self.rename_files {
            settings.rename_files = rename_files;
        }

        if let Some(ref comment) = self.comment_str {
            settings.formatting.comment_str = comment.clone();
        }
    }
}

// ============================================================================
// Output Formatting Arguments (for concat command)
// ============================================================================

/// Arguments for output formatting in concatenation.
///
/// Used by: concat
#[derive(Debug, Args, Clone, Default)]
pub struct FormattingArgs {
    /// Comment string for the target language (e.g., '--' for SQL)
    #[arg(short = 'c', long = "comment-prefix", value_name = "PREFIX")]
    pub comment_str: Option<String>,

    /// String to insert between concatenated files
    #[arg(short = 's', long = "file-separator", value_name = "SEPARATOR")]
    pub file_separator_str: Option<String>,

    /// String to append at end of each file
    #[arg(short = 'a', long = "file-suffix", value_name = "SUFFIX")]
    pub file_end_str: Option<String>,
}

impl FormattingArgs {
    /// Apply formatting arguments to settings.
    pub fn apply_to_settings(&self, settings: &mut Settings) {
        if let Some(ref comment) = self.comment_str {
            settings.formatting.comment_str = comment.clone();
        }

        if let Some(ref separator) = self.file_separator_str {
            settings.formatting.file_separator_str = separator.clone();
        }

        if let Some(ref suffix) = self.file_end_str {
            settings.formatting.file_end_str = suffix.clone();
        }
    }
}

// ============================================================================
// Execution Mode (for destructive commands)
// ============================================================================

/// Execution mode for commands that can modify or delete files.
///
/// Commands that modify the filesystem (clean, update) require explicit
/// specification of whether to preview or execute changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ExecutionMode {
    /// Preview changes without executing them
    #[default]
    DryRun,
    /// Actually perform the operations
    Execute,
}

impl ExecutionMode {
    /// Returns true if this mode should actually execute changes.
    pub fn should_execute(&self) -> bool {
        matches!(self, ExecutionMode::Execute)
    }

    /// Returns true if this mode is a dry-run preview.
    pub fn is_dry_run(&self) -> bool {
        matches!(self, ExecutionMode::DryRun)
    }
}

/// Arguments for controlling execution mode on destructive commands.
///
/// Used by: clean, update (when modifying files)
#[derive(Debug, Args, Clone, Default)]
pub struct ExecutionArgs {
    /// Execution mode: preview changes (dry-run) or execute them
    #[arg(
        long = "mode",
        value_enum,
        default_value = "dry-run",
        help = "Preview changes without executing (dry-run) or actually perform operations (execute)"
    )]
    pub mode: ExecutionMode,

    /// Force operations without confirmation prompts
    #[arg(short = 'f', long = "force", action = clap::ArgAction::SetTrue)]
    pub force: bool,
}

impl ExecutionArgs {
    /// Apply execution arguments to settings.
    pub fn apply_to_settings(&self, settings: &mut Settings) {
        settings.behavior.dry_run = self.mode.is_dry_run();
        if self.force {
            settings.behavior.force = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_args_apply() {
        let mut settings = Settings::default();
        let args = GlobalArgs {
            config_file: Some(PathBuf::from("/test/config.toml")),
            verbose: true,
            quiet: false,
        };

        args.apply_to_settings(&mut settings);

        assert!(settings.behavior.verbose);
        assert!(!settings.behavior.quiet);
    }

    #[test]
    fn test_graph_input_args_apply() {
        let mut settings = Settings::default();
        let args = GraphInputArgs {
            input_dirs: Some(vec![PathBuf::from("/test")]),
            include_extensions: Some(vec!["sql".to_string()]),
            layers: Some("alpha,beta".to_string()),
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        assert_eq!(settings.input_dirs, vec![PathBuf::from("/test")]);
        assert_eq!(settings.filters.include_extensions, vec!["sql"]);
        assert_eq!(settings.layers.names, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_execution_mode() {
        assert!(ExecutionMode::DryRun.is_dry_run());
        assert!(!ExecutionMode::DryRun.should_execute());
        assert!(!ExecutionMode::Execute.is_dry_run());
        assert!(ExecutionMode::Execute.should_execute());
    }

    #[test]
    fn test_filter_args_get_schemas() {
        let settings = Settings::default();

        // With explicit schemas in args
        let args = FilterArgs {
            schemas: Some(vec!["auth".to_string()]),
            ..Default::default()
        };
        assert_eq!(args.get_schemas(&settings), vec!["auth"]);

        // Without schemas in args, falls back to settings
        let args = FilterArgs::default();
        assert!(args.get_schemas(&settings).is_empty());
    }
}
