//! Handlers for database-level global objects.
//!
//! This module contains handlers for PostgreSQL objects that operate at the
//! database level rather than within a schema:
//! - `language`: Procedural languages (plpgsql, plpython, etc.)
//! - `event_trigger`: Database event triggers
//! - `transform`: Type transforms for procedural languages
//! - `conversion`: Character set conversions
//! - `comment`: COMMENT ON statements

mod comment;
mod conversion;
mod event_trigger;
mod language;
mod transform;

use crate::commands::import::handlers::registry::HandlerRegistry;

/// Register all global handlers with the registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(language::create_handler());
    registry.register(event_trigger::create_handler());
    registry.register(transform::create_handler());
    registry.register(conversion::create_handler());
    registry.register(comment::create_handler());
}
