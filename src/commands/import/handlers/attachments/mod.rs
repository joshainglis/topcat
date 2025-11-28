//! Handlers for PostgreSQL table attachment objects.
//!
//! This module contains handlers for objects that attach to parent objects:
//! - [`IndexHandler`]: Index objects (attach to Table)
//! - [`ConstraintHandler`]: Constraint objects (attach to Table)
//! - [`FkConstraintHandler`]: Foreign key constraints (attach to Table)
//! - [`TriggerHandler`]: Trigger objects (attach to Function)
//! - [`PolicyHandler`]: Row-level security policies (attach to Table)
//! - [`RowSecurityHandler`]: Row security enable/disable (attach to Table)
//! - [`DefaultHandler`]: Default value objects (attach to Table)
//! - [`StatisticsHandler`]: Statistics objects (attach to Table)
//! - [`RuleHandler`]: Rule objects (attach to Table/View)
//!
//! Attachments are characterized by:
//! - `is_primary()` returns `false`
//! - `layer()` returns `Layer::Append`
//! - `attach_to_parent` is `true` in their config

mod constraint;
mod default;
mod fk_constraint;
mod index;
mod policy;
mod row_security;
mod rule;
mod statistics;
mod trigger;

pub use constraint::ConstraintHandler;
pub use default::DefaultHandler;
pub use fk_constraint::FkConstraintHandler;
pub use index::IndexHandler;
pub use policy::PolicyHandler;
pub use row_security::RowSecurityHandler;
pub use rule::RuleHandler;
pub use statistics::StatisticsHandler;
pub use trigger::TriggerHandler;

use super::registry::HandlerRegistry;

/// Register all attachment handlers with the registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(index::create_handler());
    registry.register(constraint::create_handler());
    registry.register(fk_constraint::create_handler());
    registry.register(trigger::create_handler());
    registry.register(policy::create_handler());
    registry.register(row_security::create_handler());
    registry.register(default::create_handler());
    registry.register(statistics::create_handler());
    registry.register(rule::create_handler());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::object_types::ObjectType;

    #[test]
    fn test_register_handlers() {
        let mut registry = HandlerRegistry::new();
        register_handlers(&mut registry);

        assert!(registry.has_handler(&ObjectType::Index));
        assert!(registry.has_handler(&ObjectType::Constraint));
        assert!(registry.has_handler(&ObjectType::FkConstraint));
        assert!(registry.has_handler(&ObjectType::Trigger));
        assert!(registry.has_handler(&ObjectType::Policy));
        assert!(registry.has_handler(&ObjectType::RowSecurity));
        assert!(registry.has_handler(&ObjectType::Default));
        assert!(registry.has_handler(&ObjectType::Statistics));
        assert!(registry.has_handler(&ObjectType::Rule));
    }
}
