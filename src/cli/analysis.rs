//! Analysis arguments for dependency analysis operations.

use clap::Args;
use std::path::PathBuf;

use crate::settings::Settings;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_args_apply() {
        let mut settings = Settings::default();
        let args = AnalysisArgs {
            root_nodes: Some(vec!["my_schema.main".to_string()]),
            root_patterns: Some(vec!["**/api/*.sql".to_string()]),
            no_protect_implicit: true,
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        assert_eq!(
            settings.analysis.root_nodes,
            vec!["my_schema.main".to_string()]
        );
        assert_eq!(
            settings.analysis.root_patterns,
            vec!["**/api/*.sql".to_string()]
        );
        assert!(!settings.analysis.protect_implicit);
    }

    #[test]
    fn test_external_check_dirs_conversion() {
        let mut settings = Settings::default();
        let args = AnalysisArgs {
            external_check_dirs: Some(vec![PathBuf::from("/app/src"), PathBuf::from("/app/lib")]),
            external_check_patterns: Some(vec!["*.py".to_string()]),
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        assert_eq!(settings.analysis.external_check_dirs.len(), 2);
        assert_eq!(
            settings.analysis.external_check_patterns,
            vec!["*.py".to_string()]
        );
    }
}
