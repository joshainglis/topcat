//! Graph input arguments for commands that build dependency graphs.

use clap::Args;
use std::path::PathBuf;

use crate::settings::Settings;

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_layer_parsing_with_whitespace() {
        let mut settings = Settings::default();
        let args = GraphInputArgs {
            layers: Some("  prepend , normal  ,  append  ".to_string()),
            ..Default::default()
        };

        args.apply_to_settings(&mut settings);

        assert_eq!(settings.layers.names, vec!["prepend", "normal", "append"]);
    }
}
