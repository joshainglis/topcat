//! Global CLI arguments shared by all commands.

use clap::Args;
use std::path::PathBuf;

use crate::settings::Settings;

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
}
