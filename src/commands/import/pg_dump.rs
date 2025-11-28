//! PostgreSQL dump file parser and splitter.
//!
//! This module provides the CLI interface for parsing pg_dump output files and
//! splitting them into organized per-object SQL files with topcat-compatible headers.
//!
//! # Supported Object Types
//!
//! The parser supports all PostgreSQL object types including:
//! - Schema-level: TABLE, VIEW, MATERIALIZED VIEW, FOREIGN TABLE, SEQUENCE
//! - Types: TYPE (enum, composite, domain, range), DOMAIN, COLLATION
//! - Routines: FUNCTION, PROCEDURE, AGGREGATE
//! - Full Text Search: TEXT SEARCH CONFIGURATION, DICTIONARY, PARSER, TEMPLATE
//! - Foreign Data: FOREIGN DATA WRAPPER, SERVER, USER MAPPING
//! - Operators: OPERATOR, OPERATOR CLASS, OPERATOR FAMILY
//! - Replication: PUBLICATION, SUBSCRIPTION
//! - Security: ACL, DEFAULT ACL, SECURITY LABEL
//! - And more...
//!
//! # Example
//!
//! ```bash
//! topcat import pg-dump database.sql ./output/ --schema-pattern "app_\\w+"
//! ```

use std::path::PathBuf;

use clap::Args;

use topcat::cli::GlobalArgs;
use topcat::exceptions::TopCatError;
use topcat::logging::{Logger, init_logging};
use topcat::settings::Settings;

use super::sources::ImportSource;
use super::sources::pg_dump::{ImportOrchestrator, OrchestratorConfig, PgDumpParser};

/// Command-line arguments for the pg-dump import subcommand.
#[derive(Debug, Args, Clone)]
pub struct PgDumpArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    /// Path to the pg_dump SQL file to import
    #[arg(value_name = "DUMP_FILE")]
    pub dump_file: PathBuf,

    /// Output directory for the split SQL files
    #[arg(value_name = "OUTPUT_DIR")]
    pub output_dir: PathBuf,

    /// Regex pattern for matching schema names (for CAST/OPERATOR parsing)
    #[arg(long, value_name = "PATTERN")]
    pub schema_pattern: Option<String>,

    /// Preview changes without writing files
    #[arg(long)]
    pub dry_run: bool,

    /// Include ACL (GRANT/REVOKE) statements with objects
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub include_acl: bool,

    /// Include OWNER statements with objects
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub include_owner: bool,

    /// Generate layer headers based on object type
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub generate_layers: bool,

    /// Auto-generate dependency headers from SQL content analysis
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub generate_deps: bool,
}

impl PgDumpArgs {
    /// Execute the pg-dump import command.
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Load settings
        let config_path = self.global.config_path();
        let mut settings = Settings::load(config_path)
            .map_err(|e| TopCatError::ConfigError(format!("Failed to load configuration: {e}")))?;

        // Apply CLI overrides
        self.global.apply_to_settings(&mut settings);

        // Initialize logging
        let quiet = settings.behavior.quiet;
        let verbose = settings.behavior.verbose;
        init_logging(verbose, quiet);
        let logger = Logger::new(quiet, verbose);

        // Validate input file
        if !self.dump_file.exists() {
            return Err(TopCatError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Dump file not found: {}", self.dump_file.display()),
            )));
        }

        logger.info(&format!("Parsing: {}", self.dump_file.display()));
        logger.info(&format!("Output:  {}", self.output_dir.display()));

        // Get schema pattern from CLI or config
        let schema_pattern = self
            .schema_pattern
            .clone()
            .or_else(|| settings.sql_discovery.schema_pattern.clone());

        if let Some(ref pattern) = schema_pattern {
            logger.debug(&format!("Using schema pattern: {pattern}"));
        }

        // Parse dump file using new parser
        let mut parser = PgDumpParser::from_file(self.dump_file.clone(), schema_pattern)?;

        let objects = parser.extract_objects()?;
        let security = parser.collect_security()?;
        let default_acl = parser.collect_default_acl();

        // Configure and run orchestrator
        let config = OrchestratorConfig::new(self.output_dir.clone())
            .with_dry_run(self.dry_run)
            .with_include_acl(self.include_acl)
            .with_include_owner(self.include_owner)
            .with_generate_layers(self.generate_layers)
            .with_generate_deps(self.generate_deps);

        let mut orchestrator = ImportOrchestrator::new(config, logger);
        orchestrator.process(objects, security, default_acl)
    }
}
