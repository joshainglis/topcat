//! Unified logging module for Topcat.
//!
//! This module provides centralized logging functionality that respects both
//! quiet and verbose modes consistently across all commands.
//!
//! ## Two Logging Systems
//!
//! Topcat uses two complementary logging systems:
//!
//! 1. **User-facing output** (this module): All messages displayed to users,
//!    including errors, warnings, success messages, tables, and interactive prompts.
//!    Controlled by `--quiet` and `--verbose` flags.
//!
//! 2. **Internal debug logging** (`log` crate): Library-level debugging for
//!    development and troubleshooting. Controlled by `RUST_LOG` environment
//!    variable and `--verbose` flag.
//!
//! ## Usage
//!
//! ```rust
//! use topcat::logging::Logger;
//!
//! let logger = Logger::new(false, false); // not quiet, not verbose
//!
//! logger.error("Something went wrong");     // Always shown
//! logger.warn("Be careful");                // Always shown
//! logger.success("Operation completed");    // Hidden in quiet mode
//! logger.info("Processing files...");       // Hidden in quiet mode
//! logger.debug("Detailed information");     // Only shown in verbose mode
//! ```

use std::io::{self, Write};

use comfy_table::Table;
use log::LevelFilter;

/// Unified logger for all user-facing output.
///
/// Provides consistent output handling across all Topcat commands with
/// respect for quiet and verbose modes.
///
/// ## Output Levels
///
/// * **error**: Always shown (even in quiet mode) - critical failures
/// * **warn**: Always shown (even in quiet mode) - important warnings
/// * **success**: Shown unless quiet - successful operation confirmations
/// * **info**: Shown unless quiet - regular informational messages
/// * **debug**: Only shown if verbose - detailed debugging information
///
/// ## Quiet Mode
///
/// When `quiet` is true, only errors and warnings are shown. This is designed
/// for CI/CD environments where minimal output is desired.
///
/// ## Verbose Mode
///
/// When `verbose` is true, debug-level messages are shown in addition to all
/// other output levels. This provides detailed information for troubleshooting.
#[derive(Debug, Clone)]
pub struct Logger {
    quiet: bool,
    verbose: bool,
}

impl Logger {
    /// Create a new logger with the specified modes.
    ///
    /// # Arguments
    ///
    /// * `quiet` - If true, suppress info, success, and debug messages
    /// * `verbose` - If true, show debug messages
    ///
    /// # Note
    ///
    /// If both `quiet` and `verbose` are true, quiet takes precedence for
    /// most output, but debug messages will still be suppressed.
    pub fn new(quiet: bool, verbose: bool) -> Self {
        Self { quiet, verbose }
    }

    /// Log an error message (always shown, even in quiet mode).
    ///
    /// Use for critical failures that the user must be aware of.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use topcat::logging::Logger;
    /// let logger = Logger::new(false, false);
    /// logger.error("❌ Failed to read configuration file");
    /// ```
    pub fn error(&self, msg: &str) {
        eprintln!("{msg}");
    }

    /// Log a warning message (always shown, even in quiet mode).
    ///
    /// Use for important warnings that the user should be aware of,
    /// but which don't prevent the operation from completing.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use topcat::logging::Logger;
    /// let logger = Logger::new(false, false);
    /// logger.warn("⚠️  Warning: Some files were skipped");
    /// ```
    pub fn warn(&self, msg: &str) {
        eprintln!("{msg}");
    }

    /// Log a success message (hidden in quiet mode).
    ///
    /// Use for confirming successful completion of operations.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use topcat::logging::Logger;
    /// let logger = Logger::new(false, false);
    /// logger.success("✅ All files processed successfully");
    /// ```
    pub fn success(&self, msg: &str) {
        if !self.quiet {
            println!("{msg}");
        }
    }

    /// Log an informational message (hidden in quiet mode).
    ///
    /// Use for regular status updates and informational messages.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use topcat::logging::Logger;
    /// let logger = Logger::new(false, false);
    /// logger.info("Processing 42 files...");
    /// ```
    pub fn info(&self, msg: &str) {
        if !self.quiet {
            println!("{msg}");
        }
    }

    /// Log a debug message (only shown in verbose mode).
    ///
    /// Use for detailed information that's helpful for troubleshooting
    /// but too noisy for normal operation.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use topcat::logging::Logger;
    /// let logger = Logger::new(false, true); // verbose mode
    /// logger.debug("🔍 Inspecting node: my_schema.table_a");
    /// ```
    pub fn debug(&self, msg: &str) {
        if self.verbose && !self.quiet {
            println!("{msg}");
        }
    }

    /// Print a section header with title and separator line.
    ///
    /// Hidden in quiet mode. Use to organize output into logical sections.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use topcat::logging::Logger;
    /// let logger = Logger::new(false, false);
    /// logger.section("🔍 Dead Branches Analysis");
    /// ```
    pub fn section(&self, title: &str) {
        if !self.quiet {
            println!("\n{title}");
            println!("═══════════════════════════════════════════════════════════\n");
        }
    }

    /// Print a formatted table.
    ///
    /// Hidden in quiet mode. Use for displaying structured data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use topcat::logging::Logger;
    /// # use comfy_table::Table;
    /// let logger = Logger::new(false, false);
    /// let mut table = Table::new();
    /// table.set_header(vec!["Name", "Status"]);
    /// table.add_row(vec!["File A", "OK"]);
    /// logger.table(&table);
    /// ```
    pub fn table(&self, table: &Table) {
        if !self.quiet {
            println!("{table}");
        }
    }

    /// Print a separator line for visual organization.
    ///
    /// Hidden in quiet mode.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use topcat::logging::Logger;
    /// let logger = Logger::new(false, false);
    /// logger.separator();
    /// ```
    pub fn separator(&self) {
        if !self.quiet {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
    }

    /// Print a blank line for spacing.
    ///
    /// Hidden in quiet mode.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use topcat::logging::Logger;
    /// let logger = Logger::new(false, false);
    /// logger.newline();
    /// ```
    pub fn newline(&self) {
        if !self.quiet {
            println!();
        }
    }

    /// Display an interactive prompt and read user input.
    ///
    /// Returns `true` if the user confirms (enters 'y' or 'Y'),
    /// `false` otherwise.
    ///
    /// Always shown, even in quiet mode, since it requires user interaction.
    ///
    /// # Arguments
    ///
    /// * `msg` - The prompt message to display
    ///
    /// # Returns
    ///
    /// `Ok(true)` if user confirms, `Ok(false)` if user declines,
    /// `Err` if there's an I/O error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use topcat::logging::Logger;
    /// let logger = Logger::new(false, false);
    /// match logger.prompt("⚠️  Are you sure? [y/N]: ") {
    ///     Ok(true) => println!("User confirmed"),
    ///     Ok(false) => println!("User declined"),
    ///     Err(e) => eprintln!("Error reading input: {}", e),
    /// }
    /// ```
    pub fn prompt(&self, msg: &str) -> io::Result<bool> {
        print!("{msg}");
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        Ok(response.trim().eq_ignore_ascii_case("y"))
    }

    /// Display a progress or status indicator.
    ///
    /// Hidden in quiet mode. Use for showing ongoing operations.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use topcat::logging::Logger;
    /// let logger = Logger::new(false, false);
    /// logger.progress("🔍 Building dependency graph...");
    /// ```
    pub fn progress(&self, msg: &str) {
        if !self.quiet {
            println!("{msg}");
        }
    }

    /// Check if quiet mode is enabled.
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Check if verbose mode is enabled.
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }
}

/// Initialize both the user-facing logger and internal debug logging.
///
/// This function sets up the `log` crate for internal library logging,
/// with the level controlled by the `verbose` flag. In quiet mode, only
/// warnings and errors from the `log` crate are shown.
///
/// # Arguments
///
/// * `verbose` - If true, enable DEBUG level logging; otherwise INFO level
/// * `quiet` - If true, suppress INFO and DEBUG logs, showing only WARN and ERROR
///
/// # Note
///
/// This function should be called once at application startup. Subsequent
/// calls are silently ignored.
pub fn init_logging(verbose: bool, quiet: bool) {
    // Determine the log level based on quiet and verbose flags
    let log_level = if quiet {
        // In quiet mode, only show warnings and errors from library code
        LevelFilter::Warn
    } else if verbose {
        // In verbose mode, show debug information
        LevelFilter::Debug
    } else {
        // Default: show info level
        LevelFilter::Info
    };

    // Try to initialize env_logger - ignore if already initialized
    env_logger::Builder::new()
        .filter(None, log_level)
        .try_init()
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_modes() {
        // Test that we can create loggers with different modes
        let _normal = Logger::new(false, false);
        let _quiet = Logger::new(true, false);
        let _verbose = Logger::new(false, true);
        let _both = Logger::new(true, true);
    }

    #[test]
    fn test_is_quiet() {
        let logger = Logger::new(true, false);
        assert!(logger.is_quiet());

        let logger = Logger::new(false, false);
        assert!(!logger.is_quiet());
    }

    #[test]
    fn test_is_verbose() {
        let logger = Logger::new(false, true);
        assert!(logger.is_verbose());

        let logger = Logger::new(false, false);
        assert!(!logger.is_verbose());
    }
}
