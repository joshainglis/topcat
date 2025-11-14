//! Dependency graph management and analysis.
//!
//! This module provides the core `TCGraph` structure for building and analyzing
//! file dependency graphs with topological sorting and layer constraints.

mod builder;
mod core;
mod filters;
mod schema;
mod validation;

// Re-export the main TCGraph type
pub use core::TCGraph;
