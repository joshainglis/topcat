//! Execution mode arguments for destructive commands.

use clap::{Args, ValueEnum};

use crate::settings::Settings;

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
    fn test_execution_mode() {
        assert!(ExecutionMode::DryRun.is_dry_run());
        assert!(!ExecutionMode::DryRun.should_execute());
        assert!(!ExecutionMode::Execute.is_dry_run());
        assert!(ExecutionMode::Execute.should_execute());
    }

    #[test]
    fn test_execution_args_apply() {
        let mut settings = Settings::default();
        settings.behavior.dry_run = true;

        let args = ExecutionArgs {
            mode: ExecutionMode::Execute,
            force: true,
        };

        args.apply_to_settings(&mut settings);

        assert!(!settings.behavior.dry_run);
        assert!(settings.behavior.force);
    }

    #[test]
    fn test_default_is_dry_run() {
        let args = ExecutionArgs::default();
        assert!(args.mode.is_dry_run());
        assert!(!args.force);
    }
}
