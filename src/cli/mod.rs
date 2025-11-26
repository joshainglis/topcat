//! CLI argument structures for Topcat commands.
//!
//! This module provides composable argument groups that commands can mix and match
//! based on their specific needs. Each group handles a focused set of related options.
//!
//! ## Argument Groups
//!
//! | Group | Purpose | Used By |
//! |-------|---------|---------|
//! | [`GlobalArgs`] | config, verbose, quiet | All commands |
//! | [`GraphInputArgs`] | input_dirs, extensions, layers | concat, update, analyze, clean, schema, export |
//! | [`FilterArgs`] | node prefixes, schemas, subdir_filter | concat, analyze, clean, export |
//! | [`AnalysisArgs`] | root patterns, external checking | analyze, clean |
//! | [`SqlDiscoveryArgs`] | SQL discovery settings | update |
//! | [`FormattingArgs`] | comment/separator/suffix strings | concat |
//! | [`ExecutionArgs`] | mode (dry-run/execute), force | clean, update |
//!
//! ## Usage
//!
//! Commands compose these groups using `#[command(flatten)]`:
//!
//! ```ignore
//! #[derive(Debug, Args)]
//! pub struct MyCommandArgs {
//!     #[command(flatten)]
//!     pub global: GlobalArgs,
//!
//!     #[command(flatten)]
//!     pub input: GraphInputArgs,
//!
//!     // Command-specific args...
//! }
//! ```

mod analysis;
mod execution;
mod filter;
mod formatting;
mod global;
mod input;
mod sql_discovery;

pub use analysis::AnalysisArgs;
pub use execution::{ExecutionArgs, ExecutionMode};
pub use filter::FilterArgs;
pub use formatting::FormattingArgs;
pub use global::GlobalArgs;
pub use input::GraphInputArgs;
pub use sql_discovery::SqlDiscoveryArgs;
