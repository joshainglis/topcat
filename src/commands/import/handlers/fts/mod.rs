//! Handlers for PostgreSQL full-text search objects.
//!
//! This module contains handlers for FTS objects:
//! - [`ConfigurationHandler`]: Text search configurations
//! - [`DictionaryHandler`]: Text search dictionaries
//! - [`ParserHandler`]: Text search parsers
//! - [`TemplateHandler`]: Text search templates
//!
//! FTS objects are characterized by:
//! - `is_primary()` returns `true` (standalone objects)
//! - `layer()` returns `Layer::Prepend` (foundation objects)
//! - `attach_to_parent` is `false` in their config
//! - Output path uses `fts/{subcategory}/` structure

mod configuration;
mod dictionary;
mod parser;
mod template;

pub use configuration::ConfigurationHandler;
pub use dictionary::DictionaryHandler;
pub use parser::ParserHandler;
pub use template::TemplateHandler;

use super::registry::HandlerRegistry;

/// Register all FTS handlers with the registry.
pub fn register_handlers(registry: &mut HandlerRegistry) {
    registry.register(configuration::create_handler());
    registry.register(dictionary::create_handler());
    registry.register(parser::create_handler());
    registry.register(template::create_handler());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::object_types::ObjectType;

    #[test]
    fn test_register_handlers() {
        let mut registry = HandlerRegistry::new();
        register_handlers(&mut registry);

        assert!(registry.has_handler(&ObjectType::TextSearchConfiguration));
        assert!(registry.has_handler(&ObjectType::TextSearchDictionary));
        assert!(registry.has_handler(&ObjectType::TextSearchParser));
        assert!(registry.has_handler(&ObjectType::TextSearchTemplate));
    }
}
