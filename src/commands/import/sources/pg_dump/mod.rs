//! PostgreSQL pg_dump file parser.
//!
//! This module provides the [`PgDumpParser`] for extracting database objects
//! from pg_dump output files. It implements the [`ImportSource`] trait to
//! produce [`RawObject`] instances.
//!
//! # Architecture
//!
//! The parser uses regex patterns specific to pg_dump output format to:
//! 1. Parse metadata comments (`-- Name: ...; Type: ...; Schema: ...`)
//! 2. Extract SQL content for each object
//! 3. Identify structural dependencies (e.g., trigger → function)
//! 4. Collect security statements (GRANT/REVOKE/OWNER)
//!
//! Different PostgreSQL versions may produce slightly different output formats.
//! The patterns in this module target modern pg_dump output (PostgreSQL 10+).
//!
//! # Example
//!
//! ```ignore
//! use crate::commands::import::sources::pg_dump::PgDumpParser;
//! use crate::commands::import::sources::ImportSource;
//!
//! let mut parser = PgDumpParser::new(dump_file, schema_pattern);
//! let objects = parser.extract_objects()?;
//! let security = parser.collect_security()?;
//! ```

mod patterns;

pub use patterns::*;
