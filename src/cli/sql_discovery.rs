//! SQL discovery arguments for the update command.

use clap::Args;
use std::path::PathBuf;

use crate::settings::{HeaderUpdateMode, MergeStrategy, Settings};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_discovery_args_apply() {
        let mut settings = Settings::default();
        let args = SqlDiscoveryArgs {
            enable_sql_discovery: Some(true),
            schema_pattern: Some(r"myapp_\w+".to_string()),
            update_headers: Some(true),
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        assert!(settings.sql_discovery.enabled);
        assert_eq!(
            settings.sql_discovery.schema_pattern,
            Some(r"myapp_\w+".to_string())
        );
        assert_eq!(settings.header_update_mode, HeaderUpdateMode::InPlace);
    }

    #[test]
    fn test_generate_headers_overrides_update() {
        let mut settings = Settings::default();
        let args = SqlDiscoveryArgs {
            update_headers: Some(true),
            generate_headers_dir: Some(PathBuf::from("/output")),
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        // generate_headers_dir takes precedence
        assert_eq!(settings.header_update_mode, HeaderUpdateMode::Generate);
        assert_eq!(settings.header_output_dir, Some(PathBuf::from("/output")));
    }
}
