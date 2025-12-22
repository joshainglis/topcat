//! Handlers for PostgreSQL foreign data wrapper objects.
//!
//! This module contains handlers for FDW objects:
//! - [`WrapperHandler`]: Foreign data wrappers
//! - [`ServerHandler`]: Foreign servers
//! - [`UserMappingHandler`]: User mappings
//!
//! FDW objects are characterized by:
//! - `is_primary()` returns `true` (standalone objects)
//! - `layer()` returns `Layer::Prepend` (foundation objects)
//! - `attach_to_parent` is `false` in their config
//! - Output path uses `_global/fdw/` structure (global objects, no schema)

mod server;
mod user_mapping;
mod wrapper;

use super::registry::HandlerRegistry;

/// Register all FDW handlers with the registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(wrapper::create_handler());
    registry.register(server::create_handler());
    registry.register(user_mapping::create_handler());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::object_types::ObjectType;

    #[test]
    fn test_register_handlers() {
        let mut registry = HandlerRegistry::new();
        register_handlers(&mut registry);

        assert!(registry.has_handler(&ObjectType::ForeignDataWrapper));
        assert!(registry.has_handler(&ObjectType::Server));
        assert!(registry.has_handler(&ObjectType::UserMapping));
    }
}
