//! SQL discovery arguments for the update command.

use clap::{ArgAction, Args};
use std::path::PathBuf;

use crate::settings::{HeaderUpdateMode, MergeStrategy, Settings};

/// Arguments for SQL dependency discovery.
///
/// Used by: update
#[derive(Debug, Args, Clone, Default)]
pub struct SqlDiscoveryArgs {
    /// Enable automatic dependency discovery from SQL content
    #[arg(long = "sql-discovery", action = ArgAction::SetTrue, overrides_with = "no_sql_discovery")]
    pub sql_discovery: bool,

    /// Disable automatic dependency discovery from SQL content
    #[arg(long = "no-sql-discovery", action = ArgAction::SetTrue, overrides_with = "sql_discovery", hide = true)]
    pub no_sql_discovery: bool,

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
    #[arg(long = "update-headers", action = ArgAction::SetTrue, overrides_with = "no_update_headers")]
    pub update_headers: bool,

    /// Disable in-place header updates
    #[arg(long = "no-update-headers", action = ArgAction::SetTrue, overrides_with = "update_headers", hide = true)]
    pub no_update_headers: bool,

    /// Generate files with updated headers in this directory
    #[arg(long = "generate-headers", value_name = "DIR")]
    pub generate_headers_dir: Option<PathBuf>,

    /// Rename files based on discovered node names (requires --update-headers or --generate-headers)
    #[arg(long = "rename-files", action = ArgAction::SetTrue, overrides_with = "no_rename_files")]
    pub rename_files: bool,

    /// Disable file renaming
    #[arg(long = "no-rename-files", action = ArgAction::SetTrue, overrides_with = "rename_files", hide = true)]
    pub no_rename_files: bool,

    /// Comment string for the target language (e.g., '--' for SQL)
    #[arg(short = 'c', long = "comment-prefix", value_name = "PREFIX")]
    pub comment_str: Option<String>,
}

impl SqlDiscoveryArgs {
    /// Get effective sql_discovery setting: Some(true/false) if explicitly set, None if not specified.
    pub fn effective_sql_discovery(&self) -> Option<bool> {
        if self.sql_discovery {
            Some(true)
        } else if self.no_sql_discovery {
            Some(false)
        } else {
            None
        }
    }

    /// Get effective update_headers setting: Some(true/false) if explicitly set, None if not specified.
    pub fn effective_update_headers(&self) -> Option<bool> {
        if self.update_headers {
            Some(true)
        } else if self.no_update_headers {
            Some(false)
        } else {
            None
        }
    }

    /// Get effective rename_files setting: Some(true/false) if explicitly set, None if not specified.
    pub fn effective_rename_files(&self) -> Option<bool> {
        if self.rename_files {
            Some(true)
        } else if self.no_rename_files {
            Some(false)
        } else {
            None
        }
    }

    /// Apply SQL discovery arguments to settings.
    pub fn apply_to_settings(&self, settings: &mut Settings) {
        if let Some(enabled) = self.effective_sql_discovery() {
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

        if let Some(update_headers) = self.effective_update_headers() {
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

        if let Some(rename_files) = self.effective_rename_files() {
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
            sql_discovery: true,
            schema_pattern: Some(r"myapp_\w+".to_string()),
            update_headers: true,
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
            update_headers: true,
            generate_headers_dir: Some(PathBuf::from("/output")),
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        // generate_headers_dir takes precedence
        assert_eq!(settings.header_update_mode, HeaderUpdateMode::Generate);
        assert_eq!(settings.header_output_dir, Some(PathBuf::from("/output")));
    }

    #[test]
    fn test_effective_sql_discovery() {
        // Neither flag set -> None
        let args = SqlDiscoveryArgs::default();
        assert_eq!(args.effective_sql_discovery(), None);

        // --sql-discovery -> Some(true)
        let args = SqlDiscoveryArgs {
            sql_discovery: true,
            ..Default::default()
        };
        assert_eq!(args.effective_sql_discovery(), Some(true));

        // --no-sql-discovery -> Some(false)
        let args = SqlDiscoveryArgs {
            no_sql_discovery: true,
            ..Default::default()
        };
        assert_eq!(args.effective_sql_discovery(), Some(false));
    }

    #[test]
    fn test_effective_update_headers() {
        // Neither flag set -> None
        let args = SqlDiscoveryArgs::default();
        assert_eq!(args.effective_update_headers(), None);

        // --update-headers -> Some(true)
        let args = SqlDiscoveryArgs {
            update_headers: true,
            ..Default::default()
        };
        assert_eq!(args.effective_update_headers(), Some(true));

        // --no-update-headers -> Some(false)
        let args = SqlDiscoveryArgs {
            no_update_headers: true,
            ..Default::default()
        };
        assert_eq!(args.effective_update_headers(), Some(false));
    }

    #[test]
    fn test_effective_rename_files() {
        // Neither flag set -> None
        let args = SqlDiscoveryArgs::default();
        assert_eq!(args.effective_rename_files(), None);

        // --rename-files -> Some(true)
        let args = SqlDiscoveryArgs {
            rename_files: true,
            ..Default::default()
        };
        assert_eq!(args.effective_rename_files(), Some(true));

        // --no-rename-files -> Some(false)
        let args = SqlDiscoveryArgs {
            no_rename_files: true,
            ..Default::default()
        };
        assert_eq!(args.effective_rename_files(), Some(false));
    }
}
