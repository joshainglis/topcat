//! Handlers for PostgreSQL operator and related objects.
//!
//! This module contains handlers for operator-related objects:
//! - [`OperatorHandler`]: Operators
//! - [`OperatorClassHandler`]: Operator classes
//! - [`OperatorFamilyHandler`]: Operator families
//! - [`AccessMethodHandler`]: Access methods
//! - [`CastHandler`]: Casts
//!
//! These objects are characterized by:
//! - `is_primary()` returns `true` (standalone objects)
//! - `layer()` returns `Layer::Prepend` (foundation objects)
//! - `attach_to_parent` is `false` in their config
//! - Output path uses `_global/operator/`, `_global/access_method/`, or `_global/cast/` structure

mod access_method;
mod cast;
mod operator;
mod operator_class;
mod operator_family;

pub use access_method::AccessMethodHandler;
pub use operator_class::OperatorClassHandler;
pub use operator_family::OperatorFamilyHandler;

use super::registry::HandlerRegistry;

/// Register all operator-related handlers with the registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(operator::create_handler());
    registry.register(operator_class::create_handler());
    registry.register(operator_family::create_handler());
    registry.register(access_method::create_handler());
    registry.register(cast::create_handler());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::object_types::ObjectType;

    #[test]
    fn test_register_handlers() {
        let mut registry = HandlerRegistry::new();
        register_handlers(&mut registry);

        assert!(registry.has_handler(&ObjectType::Operator));
        assert!(registry.has_handler(&ObjectType::OperatorClass));
        assert!(registry.has_handler(&ObjectType::OperatorFamily));
        assert!(registry.has_handler(&ObjectType::AccessMethod));
        assert!(registry.has_handler(&ObjectType::Cast));
    }
}
