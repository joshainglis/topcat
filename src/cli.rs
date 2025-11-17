use clap::Args;
use std::path::PathBuf;

use crate::settings::{HeaderUpdateMode, MergeStrategy, Settings};

/// Common CLI arguments shared across all commands.
///
/// These arguments can be used with any Topcat command and will override
/// configuration file settings when specified.
#[derive(Debug, Args, Clone, Default)]
pub struct CommonArgs {
    // File Sources
    #[arg(
        short = 'i',
        long = "input-dirs",
        help = "Directories to search for input files",
        value_name = "DIRS"
    )]
    pub input_dirs: Option<Vec<PathBuf>>,

    #[arg(
        short = 'o',
        long = "output",
        help = "Output file path",
        value_name = "FILE"
    )]
    pub output: Option<PathBuf>,

    // File Filtering
    #[arg(
        short = 'e',
        long = "include-exts",
        help = "Include only files with these extensions (without leading dot)",
        value_name = "EXTS"
    )]
    pub include_extensions: Option<Vec<String>>,

    #[arg(
        short = 'E',
        long = "exclude-exts",
        help = "Exclude files with these extensions",
        value_name = "EXTS"
    )]
    pub exclude_extensions: Option<Vec<String>>,

    #[arg(
        short = 'g',
        long = "include-glob",
        help = "Include only files matching glob pattern (relative to working directory)",
        value_name = "PATTERN"
    )]
    pub include_globs: Option<Vec<String>>,

    #[arg(
        short = 'G',
        long = "exclude-glob",
        help = "Exclude files matching glob pattern (relative to working directory)",
        value_name = "PATTERN"
    )]
    pub exclude_globs: Option<Vec<String>>,

    #[arg(long = "include-hidden", help = "Include hidden files and directories")]
    pub include_hidden: Option<bool>,

    // Layers
    #[arg(
        long = "layers",
        help = "Comma-separated list of layer names in execution order",
        value_name = "LAYERS"
    )]
    pub layers: Option<String>,

    #[arg(
        long = "fallback-layer",
        help = "Default layer for nodes without explicit layer declaration",
        value_name = "LAYER"
    )]
    pub fallback_layer: Option<String>,

    // SQL Discovery
    #[arg(
        long = "enable-sql-discovery",
        help = "Enable automatic dependency discovery from SQL content"
    )]
    pub enable_sql_discovery: Option<bool>,

    #[arg(
        long = "schema-pattern",
        help = "Regex pattern for matching schema names (e.g., '(?:schema1|schema2)_\\w+')",
        value_name = "PATTERN"
    )]
    pub schema_pattern: Option<String>,

    #[arg(
        long = "merge-strategy",
        help = "How to merge discovered and manual dependencies",
        value_parser = ["header-only", "discovery-only", "union", "header-with-fallback", "validate"],
        value_name = "STRATEGY"
    )]
    pub merge_strategy: Option<String>,

    #[arg(
        long = "update-headers",
        help = "Update files in-place with discovered dependencies"
    )]
    pub update_headers: Option<bool>,

    #[arg(
        long = "generate-headers",
        help = "Generate files with updated headers in this directory",
        value_name = "DIR"
    )]
    pub generate_headers_dir: Option<PathBuf>,

    // Formatting
    #[arg(
        short = 'c',
        long = "comment-prefix",
        help = "Comment string for the target language (e.g., '--' for SQL)",
        value_name = "PREFIX"
    )]
    pub comment_str: Option<String>,

    #[arg(
        short = 's',
        long = "file-separator",
        help = "String to insert between concatenated files",
        value_name = "SEPARATOR"
    )]
    pub file_separator_str: Option<String>,

    #[arg(
        short = 'a',
        long = "file-suffix",
        help = "String to append at end of each file",
        value_name = "SUFFIX"
    )]
    pub file_end_str: Option<String>,

    // Behavior
    #[arg(
        short = 'v',
        long = "verbose",
        help = "Enable verbose output with detailed logging",
        action = clap::ArgAction::SetTrue
    )]
    pub verbose: bool,

    #[arg(
        short = 'q',
        long = "quiet",
        help = "Suppress non-essential output (for CI/CD)",
        action = clap::ArgAction::SetTrue
    )]
    pub quiet: bool,

    #[arg(
        short = 'd',
        long = "dry-run",
        help = "Preview operations without making changes"
    )]
    pub dry_run: Option<bool>,

    #[arg(
        long = "no-dry-run",
        help = "Actually perform operations (overrides dry-run default)",
        conflicts_with = "dry_run"
    )]
    pub no_dry_run: bool,

    #[arg(
        short = 'f',
        long = "force",
        help = "Force operations without confirmation prompts",
        action = clap::ArgAction::SetTrue
    )]
    pub force: bool,

    // Node Filtering
    #[arg(
        long = "include-prefix",
        help = "Include only nodes with these name prefixes",
        value_name = "PREFIXES"
    )]
    pub include_node_prefixes: Option<Vec<String>>,

    #[arg(
        long = "exclude-prefix",
        help = "Exclude nodes with these name prefixes",
        value_name = "PREFIXES"
    )]
    pub exclude_node_prefixes: Option<Vec<String>>,

    #[arg(
        long = "subdir-filter",
        help = "Include only files from this subdirectory and their dependencies",
        value_name = "SUBDIR"
    )]
    pub subdir_filter: Option<PathBuf>,

    // Schema Filtering
    #[arg(
        long = "schema",
        help = "Filter operations to specific schemas",
        value_name = "SCHEMAS"
    )]
    pub schemas: Option<Vec<String>>,

    // Analysis
    #[arg(
        long = "root-nodes",
        help = "Specific nodes to always treat as root/entry points",
        value_name = "NODES"
    )]
    pub root_nodes: Option<Vec<String>>,

    #[arg(
        long = "root-pattern",
        help = "Glob patterns for root files",
        value_name = "PATTERNS"
    )]
    pub root_patterns: Option<Vec<String>>,

    #[arg(
        long = "root-regex",
        help = "Regex patterns for root node names",
        value_name = "PATTERNS"
    )]
    pub root_regex: Option<Vec<String>>,

    #[arg(
        long = "root-dir",
        help = "Directories where all files are considered roots",
        value_name = "DIRS"
    )]
    pub root_dirs: Option<Vec<String>>,

    #[arg(
        long = "external-check-dir",
        help = "Directories to check for external usage of SQL objects",
        value_name = "DIRS"
    )]
    pub external_check_dirs: Option<Vec<PathBuf>>,

    #[arg(
        long = "external-check-pattern",
        help = "File patterns to check for external usage (e.g., '*.py', '*.ts')",
        value_name = "PATTERNS"
    )]
    pub external_check_patterns: Option<Vec<String>>,

    // Configuration
    #[arg(
        long = "config",
        help = "Path to configuration file (TOML)",
        value_name = "FILE"
    )]
    pub config_file: Option<PathBuf>,
}

impl CommonArgs {
    /// Apply these CLI arguments to a Settings instance, overriding config file values.
    pub fn apply_to_settings(&self, settings: &mut Settings) {
        // File Sources
        if let Some(ref input_dirs) = self.input_dirs {
            settings.input_dirs = input_dirs.clone();
        }

        if let Some(ref output) = self.output {
            settings.output = Some(output.clone());
        }

        // File Filtering (additive by default, but CLI replaces config)
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

        // Layers (replace when specified)
        if let Some(ref layers_str) = self.layers {
            settings.layers.names = layers_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        if let Some(ref fallback) = self.fallback_layer {
            settings.layers.fallback = Some(fallback.clone());
        }

        // SQL Discovery
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

        // Formatting
        if let Some(ref comment) = self.comment_str {
            settings.formatting.comment_str = comment.clone();
        }

        if let Some(ref separator) = self.file_separator_str {
            settings.formatting.file_separator_str = separator.clone();
        }

        if let Some(ref suffix) = self.file_end_str {
            settings.formatting.file_end_str = suffix.clone();
        }

        // Behavior
        if self.verbose {
            settings.behavior.verbose = true;
        }

        if self.quiet {
            settings.behavior.quiet = true;
        }

        if let Some(dry_run) = self.dry_run {
            settings.behavior.dry_run = dry_run;
        }

        if self.no_dry_run {
            settings.behavior.dry_run = false;
        }

        if self.force {
            settings.behavior.force = true;
        }

        // Node Filtering
        if let Some(ref prefixes) = self.include_node_prefixes {
            settings.node_filtering.include_prefixes = prefixes.clone();
        }

        if let Some(ref prefixes) = self.exclude_node_prefixes {
            settings.node_filtering.exclude_prefixes = prefixes.clone();
        }

        if let Some(ref subdir) = self.subdir_filter {
            settings.node_filtering.subdir_filter = Some(subdir.clone());
        }

        // Schema Filtering
        if let Some(ref schemas) = self.schemas {
            settings.schema_filtering.schemas = schemas.clone();
        }

        // Analysis
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
    }

    /// Get the config file path if specified.
    pub fn config_path(&self) -> Option<&str> {
        self.config_file.as_ref().map(|p| p.to_str().unwrap())
    }
}

/// Helper macro to create command-specific args that include CommonArgs.
#[macro_export]
macro_rules! with_common_args {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field:ident : $type:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, clap::Args, Clone)]
        pub struct $name {
            #[command(flatten)]
            pub common: $crate::cli::CommonArgs,

            $(
                $(#[$field_meta])*
                pub $field : $type,
            )*
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_common_args() {
        let mut settings = Settings::default();
        let args = CommonArgs {
            input_dirs: Some(vec![PathBuf::from("/test")]),
            verbose: true,
            layers: Some("a,b,c".to_string()),
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        assert_eq!(settings.input_dirs, vec![PathBuf::from("/test")]);
        assert!(settings.behavior.verbose);
        assert_eq!(settings.layers.names, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_layer_parsing() {
        let mut settings = Settings::default();
        let args = CommonArgs {
            layers: Some("  prepend , normal  ,  append  ".to_string()),
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        assert_eq!(settings.layers.names, vec!["prepend", "normal", "append"]);
    }

    #[test]
    fn test_no_dry_run_override() {
        let mut settings = Settings::default();
        settings.behavior.dry_run = true;

        let args = CommonArgs {
            no_dry_run: true,
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        assert!(!settings.behavior.dry_run);
    }
}
