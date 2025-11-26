//! Output formatting arguments for the concat command.

use clap::Args;

use crate::settings::Settings;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatting_args_apply() {
        let mut settings = Settings::default();
        let args = FormattingArgs {
            comment_str: Some("//".to_string()),
            file_separator_str: Some("\n\n---\n\n".to_string()),
            file_end_str: Some(";".to_string()),
        };

        args.apply_to_settings(&mut settings);

        assert_eq!(settings.formatting.comment_str, "//");
        assert_eq!(settings.formatting.file_separator_str, "\n\n---\n\n");
        assert_eq!(settings.formatting.file_end_str, ";");
    }
}
