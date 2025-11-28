//! Handlers for PostgreSQL replication objects.
//!
//! This module provides handlers for logical replication objects:
//! - Publication: Defines which tables are published for replication
//! - Subscription: Subscribes to publications from other databases
//!
//! Both are global objects (not schema-qualified) and belong to the Normal layer.

mod publication;
mod subscription;

use super::registry::HandlerRegistry;

/// Register all replication handlers with the registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(publication::create_handler());
    registry.register(subscription::create_handler());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::object_types::ObjectType;

    #[test]
    fn test_register_handlers() {
        let mut registry = HandlerRegistry::new();
        register_handlers(&mut registry);

        // Both handlers should be registered
        assert!(registry.has_handler(&ObjectType::Publication));
        assert!(registry.has_handler(&ObjectType::Subscription));
    }
}
