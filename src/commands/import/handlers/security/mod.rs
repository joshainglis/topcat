//! Handlers for PostgreSQL security objects.
//!
//! This module provides handlers for security-related objects:
//! - Acl: GRANT/REVOKE statements that control access to objects
//! - DefaultAcl: ALTER DEFAULT PRIVILEGES statements
//! - SecurityLabel: SECURITY LABEL statements for SELinux/similar systems
//!
//! All security objects belong to the Append layer (applied after objects are created).
//! Acl and SecurityLabel are attachments that get appended to their target objects,
//! while DefaultAcl creates standalone files.

mod acl;
mod default_acl;
mod security_label;

use super::registry::HandlerRegistry;

/// Register all security handlers with the registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(acl::create_handler());
    registry.register(default_acl::create_handler());
    registry.register(security_label::create_handler());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::object_types::ObjectType;

    #[test]
    fn test_register_handlers() {
        let mut registry = HandlerRegistry::new();
        register_handlers(&mut registry);

        // All handlers should be registered
        assert!(registry.has_handler(&ObjectType::Acl));
        assert!(registry.has_handler(&ObjectType::DefaultAcl));
        assert!(registry.has_handler(&ObjectType::SecurityLabel));
    }
}
