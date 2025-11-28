//! Import commands for bringing external data into topcat-managed projects.
//!
//! This module provides functionality to import database dumps and other sources
//! into organized directory structures with topcat-compatible headers.
//!
//! # Architecture
//!
//! The import system uses a layered architecture:
//!
//! 1. **Sources** ([`sources`]): Extract raw object data from various inputs (pg_dump, etc.)
//! 2. **Handlers** ([`handlers`]): Process objects with type-specific logic
//! 3. **Output** ([`output`]): Write processed objects to files with headers
//!
//! # Available Subcommands
//!
//! - **pg-dump**: Import a PostgreSQL pg_dump file and split into per-object files
//!
//! # Examples
//!
//! ```bash
//! # Import a pg_dump file
//! topcat import pg-dump database.sql ./output/
//!
//! # Preview what would be created
//! topcat import pg-dump database.sql ./output/ --dry-run
//!
//! # Use a custom schema pattern
//! topcat import pg-dump database.sql ./output/ --schema-pattern "app_\\w+"
//! ```

// Core modules (existing)
mod dependencies;
mod object_types;
mod pg_dump;

// New layered architecture modules
pub mod handlers;
pub mod output;
pub mod sources;

use clap::{Args, Subcommand};

use topcat::cli::GlobalArgs;
use topcat::exceptions::TopCatError;

pub use pg_dump::PgDumpArgs;

/// Command-line arguments for the import command.
#[derive(Debug, Args)]
pub struct ImportArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    command: ImportCommand,
}

/// Available import subcommands.
#[derive(Debug, Subcommand)]
enum ImportCommand {
    /// Import a PostgreSQL pg_dump file and split into per-object files
    ///
    /// Parses a pg_dump SQL file and creates an organized directory structure
    /// with one file per database object. Each file includes topcat-compatible
    /// headers for dependency management.
    #[command(name = "pg-dump")]
    PgDump(PgDumpArgs),
}

impl ImportArgs {
    /// Execute the import command.
    pub fn execute(&self) -> Result<(), TopCatError> {
        match &self.command {
            ImportCommand::PgDump(args) => args.execute(),
        }
    }
}
