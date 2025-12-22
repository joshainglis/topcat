//! Handlers for PostgreSQL callable objects.
//!
//! This module contains handlers for routine objects:
//! - [`FunctionHandler`]: User-defined functions
//! - [`ProcedureHandler`]: Stored procedures (PostgreSQL 11+)
//! - [`AggregateHandler`]: User-defined aggregate functions

// Allow unused - scaffolded for future phases
#![allow(unused_imports)]

mod aggregate;
mod function;
mod procedure;

pub use aggregate::AggregateHandler;
pub use function::FunctionHandler;
pub use procedure::ProcedureHandler;

use super::registry::HandlerRegistry;

/// Register all routine handlers with the registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(function::create_handler());
    registry.register(procedure::create_handler());
    registry.register(aggregate::create_handler());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::object_types::ObjectType;

    #[test]
    fn test_register_handlers() {
        let mut registry = HandlerRegistry::new();
        register_handlers(&mut registry);

        assert!(registry.has_handler(&ObjectType::Function));
        assert!(registry.has_handler(&ObjectType::Procedure));
        assert!(registry.has_handler(&ObjectType::Aggregate));
    }
}
