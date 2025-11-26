//! Filter arguments for node and schema filtering in graph operations.

use clap::Args;
use std::path::PathBuf;

use crate::settings::Settings;

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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_filter_args_apply() {
        let mut settings = Settings::default();
        let args = FilterArgs {
            include_node_prefixes: Some(vec!["auth.".to_string()]),
            schemas: Some(vec!["auth".to_string(), "billing".to_string()]),
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        assert_eq!(
            settings.node_filtering.include_prefixes,
            vec!["auth.".to_string()]
        );
        assert_eq!(
            settings.schema_filtering.schemas,
            vec!["auth".to_string(), "billing".to_string()]
        );
    }
}
