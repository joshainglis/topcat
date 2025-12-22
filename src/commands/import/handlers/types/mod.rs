//! Handlers for PostgreSQL type system objects.
//!
//! This module contains handlers for type-related objects:
//! - [`TypeHandler`]: User-defined types (enum, composite, range, base)
//! - [`DomainHandler`]: Domain types (constrained base types)
//! - [`CollationHandler`]: Collation definitions

mod collation;
mod domain;
mod type_handler;

use super::registry::HandlerRegistry;

/// Register all type handlers with the registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(type_handler::create_handler());
    registry.register(domain::create_handler());
    registry.register(collation::create_handler());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::object_types::ObjectType;

    #[test]
    fn test_register_handlers() {
        let mut registry = HandlerRegistry::new();
        register_handlers(&mut registry);

        assert!(registry.has_handler(&ObjectType::Type));
        assert!(registry.has_handler(&ObjectType::Domain));
        assert!(registry.has_handler(&ObjectType::Collation));
    }
}
