//! Handler and attachment registries.
//!
//! This module provides registries for managing type-specific handlers and
//! attachment relationships between object types.

use std::collections::HashMap;
use std::sync::Arc;

use crate::commands::import::dependencies::implicit_dependencies_for_type;
use crate::commands::import::handlers::schema_objects;
use crate::commands::import::object_types::{Layer, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;

/// Central registry for all object handlers.
///
/// The handler registry provides type-safe access to handlers for each
/// PostgreSQL object type. Handlers are registered during initialization
/// and can be looked up by object type at runtime.
///
/// # Example
///
/// ```ignore
/// let registry = HandlerRegistry::new();
///
/// // Get handler for a specific type
/// if let Some(handler) = registry.get(&ObjectType::Table) {
///     let deps = handler.implicit_dep_types();
/// }
/// ```
pub struct HandlerRegistry {
    /// Registered handler metadata by object type.
    handlers: HashMap<ObjectType, RegisteredHandler>,

    /// Registry for attachment relationships.
    attachment_registry: AttachmentRegistry,
}

/// Metadata for a registered handler.
///
/// This stores the object type and provides access to type-specific
/// functionality through the existing ObjectType methods and helpers.
#[derive(Clone)]
pub struct RegisteredHandler {
    /// The object type this handler is for.
    obj_type: ObjectType,

    /// Optional custom pattern dependency extractor.
    /// If None, returns empty vec.
    extract_pattern_deps: Option<fn(&str) -> Vec<(String, ObjectType)>>,
}

impl RegisteredHandler {
    /// Create a new registered handler for an object type.
    pub fn new(obj_type: ObjectType) -> Self {
        Self {
            obj_type,
            extract_pattern_deps: None,
        }
    }

    /// Create a handler with a custom pattern dependency extractor.
    pub fn with_pattern_deps(
        obj_type: ObjectType,
        extractor: fn(&str) -> Vec<(String, ObjectType)>,
    ) -> Self {
        Self {
            obj_type,
            extract_pattern_deps: Some(extractor),
        }
    }

    /// Get the object type.
    pub fn object_type(&self) -> ObjectType {
        self.obj_type
    }

    /// Extract pattern-based dependencies from content.
    pub fn extract_pattern_deps(&self, content: &str) -> Vec<(String, ObjectType)> {
        match self.extract_pattern_deps {
            Some(f) => f(content),
            None => vec![],
        }
    }

    /// Get implicit dependency types for this object type.
    pub fn implicit_dep_types(&self) -> Vec<ObjectType> {
        implicit_dependencies_for_type(self.obj_type)
    }

    /// Get the default configuration for this type.
    pub fn default_config(&self) -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(self.obj_type)
    }

    /// Get the layer for this type.
    pub fn layer(&self) -> Layer {
        self.obj_type.layer()
    }

    /// Check if this is a primary type.
    pub fn is_primary(&self) -> bool {
        self.obj_type.is_primary()
    }

    /// Check if this is a table attachment.
    pub fn is_table_attachment(&self) -> bool {
        self.obj_type.is_table_attachment()
    }

    /// Check if this is a global object.
    pub fn is_global(&self) -> bool {
        self.obj_type.is_global()
    }
}

impl HandlerRegistry {
    /// Create a new handler registry with all handlers registered.
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
            attachment_registry: AttachmentRegistry::new(),
        };
        registry.register_default_handlers();
        registry
    }

    /// Get handler for an object type.
    pub fn get(&self, obj_type: &ObjectType) -> Option<&RegisteredHandler> {
        self.handlers.get(obj_type)
    }

    /// Get the attachment registry.
    pub fn attachment_registry(&self) -> &AttachmentRegistry {
        &self.attachment_registry
    }

    /// Get a mutable reference to the attachment registry.
    pub fn attachment_registry_mut(&mut self) -> &mut AttachmentRegistry {
        &mut self.attachment_registry
    }

    /// Register a handler for an object type.
    pub fn register(&mut self, handler: RegisteredHandler) {
        self.handlers.insert(handler.obj_type, handler);
    }

    /// Register all default handlers.
    ///
    /// This registers handlers for all known object types using the
    /// default behavior from ObjectType methods.
    fn register_default_handlers(&mut self) {
        let all_types = [
            ObjectType::Schema,
            ObjectType::Extension,
            ObjectType::Table,
            ObjectType::View,
            ObjectType::MaterializedView,
            ObjectType::ForeignTable,
            ObjectType::Sequence,
            ObjectType::Type,
            ObjectType::Domain,
            ObjectType::Collation,
            ObjectType::Function,
            ObjectType::Procedure,
            ObjectType::Aggregate,
            ObjectType::Index,
            ObjectType::Constraint,
            ObjectType::FkConstraint,
            ObjectType::Trigger,
            ObjectType::Policy,
            ObjectType::RowSecurity,
            ObjectType::Default,
            ObjectType::Statistics,
            ObjectType::Rule,
            ObjectType::TextSearchConfiguration,
            ObjectType::TextSearchDictionary,
            ObjectType::TextSearchParser,
            ObjectType::TextSearchTemplate,
            ObjectType::ForeignDataWrapper,
            ObjectType::Server,
            ObjectType::UserMapping,
            ObjectType::Operator,
            ObjectType::OperatorClass,
            ObjectType::OperatorFamily,
            ObjectType::AccessMethod,
            ObjectType::Cast,
            ObjectType::Publication,
            ObjectType::Subscription,
            ObjectType::EventTrigger,
            ObjectType::Language,
            ObjectType::Transform,
            ObjectType::Conversion,
            ObjectType::Acl,
            ObjectType::DefaultAcl,
            ObjectType::SecurityLabel,
            ObjectType::Comment,
            ObjectType::Unknown,
        ];

        for obj_type in all_types {
            self.register(RegisteredHandler::new(obj_type));
        }

        // Override with specialized handlers from category modules
        schema_objects::register_handlers(self);
    }

    /// Check if a handler is registered for a type.
    pub fn has_handler(&self, obj_type: &ObjectType) -> bool {
        self.handlers.contains_key(obj_type)
    }

    /// Get all registered object types.
    pub fn registered_types(&self) -> Vec<ObjectType> {
        self.handlers.keys().copied().collect()
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for attachment relationships between object types.
///
/// Attachments are objects that belong to a parent object (e.g., indexes
/// belong to tables, triggers belong to functions). This registry tracks
/// those relationships and provides methods to resolve parent objects.
pub struct AttachmentRegistry {
    /// Map from attachment type to rules for finding parent.
    rules: HashMap<ObjectType, Vec<AttachmentRule>>,
}

/// A rule for attaching an object to its parent.
pub struct AttachmentRule {
    /// Possible parent types for this attachment.
    pub parent_types: Vec<ObjectType>,

    /// Function to extract parent identity from attachment content.
    ///
    /// This function analyzes the attachment's content and returns the
    /// identity of its parent object, if determinable.
    pub extract_parent: Arc<dyn Fn(&RawObject) -> Option<ParentIdentity> + Send + Sync>,
}

/// Identity of a parent object for attachment resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentIdentity {
    /// Schema of the parent (None for global objects).
    pub schema: Option<String>,

    /// Name of the parent.
    pub name: String,

    /// Type of the parent.
    pub obj_type: ObjectType,
}

impl ParentIdentity {
    /// Create a new parent identity.
    pub fn new(schema: Option<String>, name: String, obj_type: ObjectType) -> Self {
        Self {
            schema,
            name,
            obj_type,
        }
    }

    /// Get the fully qualified name of the parent.
    pub fn qualified_name(&self) -> String {
        match &self.schema {
            Some(s) => format!("{}.{}", s, self.name),
            None => self.name.clone(),
        }
    }
}

impl AttachmentRegistry {
    /// Create a new attachment registry with default rules.
    pub fn new() -> Self {
        let mut registry = Self {
            rules: HashMap::new(),
        };
        registry.register_default_rules();
        registry
    }

    /// Find the parent for an attachment object.
    ///
    /// Tries each rule for the attachment's type until one matches.
    /// Returns `None` if no parent can be determined.
    pub fn find_parent(&self, attachment: &RawObject) -> Option<ParentIdentity> {
        let rules = self.rules.get(&attachment.obj_type)?;

        for rule in rules {
            if let Some(parent) = (rule.extract_parent)(attachment) {
                return Some(parent);
            }
        }

        None
    }

    /// Register an attachment rule.
    pub fn register(&mut self, attachment_type: ObjectType, rule: AttachmentRule) {
        self.rules.entry(attachment_type).or_default().push(rule);
    }

    /// Get rules for an attachment type.
    pub fn get_rules(&self, attachment_type: &ObjectType) -> Option<&Vec<AttachmentRule>> {
        self.rules.get(attachment_type)
    }

    /// Register default attachment rules.
    ///
    /// These rules define the standard PostgreSQL attachment relationships.
    fn register_default_rules(&mut self) {
        // TODO: Implement default rules using patterns from patterns.rs
        // For now, rules are empty and will be populated as handlers are migrated.

        // Index → Table
        self.rules.insert(ObjectType::Index, vec![]);

        // Constraint → Table
        self.rules.insert(ObjectType::Constraint, vec![]);

        // FK Constraint → Table
        self.rules.insert(ObjectType::FkConstraint, vec![]);

        // Trigger → Function
        self.rules.insert(ObjectType::Trigger, vec![]);

        // Policy → Table
        self.rules.insert(ObjectType::Policy, vec![]);

        // Row Security → Table
        self.rules.insert(ObjectType::RowSecurity, vec![]);

        // Default → Table
        self.rules.insert(ObjectType::Default, vec![]);

        // Statistics → Table
        self.rules.insert(ObjectType::Statistics, vec![]);

        // Rule → Table/View
        self.rules.insert(ObjectType::Rule, vec![]);
    }

    /// Check if a type is an attachment type.
    pub fn is_attachment(&self, obj_type: &ObjectType) -> bool {
        self.rules.contains_key(obj_type)
    }
}

impl Default for AttachmentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_registry_creation() {
        let registry = HandlerRegistry::new();
        assert!(registry.has_handler(&ObjectType::Table));
        assert!(registry.has_handler(&ObjectType::Function));
        assert!(registry.has_handler(&ObjectType::Unknown));
    }

    #[test]
    fn test_handler_registry_get() {
        let registry = HandlerRegistry::new();
        let handler = registry.get(&ObjectType::Table);
        assert!(handler.is_some());

        let handler = handler.unwrap();
        assert_eq!(handler.layer(), Layer::Normal);
        assert!(handler.is_primary());
        assert!(!handler.is_table_attachment());
    }

    #[test]
    fn test_handler_implicit_deps() {
        let registry = HandlerRegistry::new();
        let handler = registry.get(&ObjectType::View).unwrap();
        let deps = handler.implicit_dep_types();
        assert!(!deps.is_empty());
        assert!(deps.contains(&ObjectType::Table));
    }

    #[test]
    fn test_attachment_registry_creation() {
        let registry = AttachmentRegistry::new();
        assert!(registry.is_attachment(&ObjectType::Index));
        assert!(registry.is_attachment(&ObjectType::Trigger));
        assert!(!registry.is_attachment(&ObjectType::Table));
    }

    #[test]
    fn test_parent_identity_qualified_name() {
        let parent = ParentIdentity::new(
            Some("public".to_string()),
            "users".to_string(),
            ObjectType::Table,
        );
        assert_eq!(parent.qualified_name(), "public.users");

        let global_parent = ParentIdentity::new(None, "plpgsql".to_string(), ObjectType::Language);
        assert_eq!(global_parent.qualified_name(), "plpgsql");
    }

    #[test]
    fn test_registered_types() {
        let registry = HandlerRegistry::new();
        let types = registry.registered_types();
        assert!(types.contains(&ObjectType::Table));
        assert!(types.contains(&ObjectType::Function));
        assert!(types.len() >= 40); // We have 40+ types
    }

    #[test]
    fn test_registered_handler_default_config() {
        let handler = RegisteredHandler::new(ObjectType::Index);
        let config = handler.default_config();
        assert!(config.attach_to_parent);
        assert!(!config.skip);
    }
}
