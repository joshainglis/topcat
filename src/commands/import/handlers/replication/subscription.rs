//! Handler for PostgreSQL SUBSCRIPTION objects.

use std::path::{Path, PathBuf};

use crate::commands::import::handlers::registry::RegisteredHandler;
use crate::commands::import::handlers::traits::{
    Categorizer, Configurable, DependencyExtractor, OutputConfig, PatternProvider, RelatedObjects,
    Renderer,
};
use crate::commands::import::object_types::{Layer, ObjectCategory, ObjectType, ObjectTypeConfig};
use crate::commands::import::sources::RawObject;
use crate::commands::import::sources::pg_dump::SUBSCRIPTION_PATTERN;

/// Handler for PostgreSQL SUBSCRIPTION objects.
///
/// Subscriptions connect to remote publications for logical replication.
/// They are global objects (no schema) that depend on publications
/// and belong to the Normal layer.
pub struct SubscriptionHandler;

impl PatternProvider for SubscriptionHandler {
    // Subscriptions are identified by metadata, not content patterns
}

impl DependencyExtractor for SubscriptionHandler {
    fn implicit_dependency_types() -> Vec<ObjectType> {
        // Subscriptions depend on their publication
        vec![ObjectType::Publication]
    }

    fn extract_pattern_dependencies(content: &str) -> Vec<(String, ObjectType)> {
        let mut deps = vec![];

        // Extract publication dependency from CREATE SUBSCRIPTION ... PUBLICATION pub_name
        if let Some(caps) = SUBSCRIPTION_PATTERN.captures(content) {
            if let Some(pub_name) = caps.name("publication") {
                deps.push((pub_name.as_str().to_string(), ObjectType::Publication));
            }
        }

        deps
    }
}

impl Categorizer for SubscriptionHandler {
    fn category() -> ObjectCategory {
        ObjectCategory::Replication
    }

    fn subcategory(_content: &str) -> Option<String> {
        Some("subscription".to_string())
    }

    fn output_path(obj: &RawObject, base_dir: &Path) -> PathBuf {
        // Global object: _global/replication/subscription/{name}.sql
        base_dir
            .join("_global")
            .join("replication")
            .join("subscription")
            .join(format!("{}.sql", obj.name))
    }
}

impl Configurable for SubscriptionHandler {
    fn default_config() -> ObjectTypeConfig {
        ObjectTypeConfig::default_for(ObjectType::Subscription)
    }

    fn layer() -> Layer {
        Layer::Normal
    }

    fn is_primary() -> bool {
        true
    }
}

impl Renderer for SubscriptionHandler {
    fn render(obj: &RawObject, related: &RelatedObjects, config: &OutputConfig) -> String {
        let mut parts = vec![obj.content.clone()];

        let related_content = related.render_all(config);
        if !related_content.is_empty() {
            parts.push(related_content);
        }

        parts.join("\n\n")
    }
}

/// Extract pattern-based dependencies from Subscription content.
fn extract_subscription_dependencies(content: &str) -> Vec<(String, ObjectType)> {
    SubscriptionHandler::extract_pattern_dependencies(content)
}

/// Create a registered handler for Subscription objects.
pub fn create_handler() -> RegisteredHandler {
    RegisteredHandler::with_pattern_deps(
        ObjectType::Subscription,
        extract_subscription_dependencies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_handler_layer() {
        assert_eq!(SubscriptionHandler::layer(), Layer::Normal);
    }

    #[test]
    fn test_subscription_handler_is_primary() {
        assert!(SubscriptionHandler::is_primary());
    }

    #[test]
    fn test_subscription_handler_category() {
        assert_eq!(SubscriptionHandler::category(), ObjectCategory::Replication);
    }

    #[test]
    fn test_subscription_subcategory() {
        let content =
            r#"CREATE SUBSCRIPTION "my_sub" CONNECTION 'host=localhost' PUBLICATION "my_pub";"#;
        assert_eq!(
            SubscriptionHandler::subcategory(content),
            Some("subscription".to_string())
        );
    }

    #[test]
    fn test_subscription_output_path() {
        let obj = RawObject::new(
            ObjectType::Subscription,
            None, // Global object - no schema
            "my_sub".to_string(),
            r#"CREATE SUBSCRIPTION "my_sub" CONNECTION 'host=localhost' PUBLICATION "my_pub";"#
                .to_string(),
        );
        let path = SubscriptionHandler::output_path(&obj, Path::new("/output"));
        assert_eq!(
            path,
            PathBuf::from("/output/_global/replication/subscription/my_sub.sql")
        );
    }

    #[test]
    fn test_subscription_implicit_dependencies() {
        let deps = SubscriptionHandler::implicit_dependency_types();
        assert!(deps.contains(&ObjectType::Publication));
    }

    #[test]
    fn test_subscription_extract_pattern_dependencies() {
        let content =
            r#"CREATE SUBSCRIPTION "my_sub" CONNECTION 'host=localhost' PUBLICATION "my_pub";"#;
        let deps = SubscriptionHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "my_pub");
        assert_eq!(deps[0].1, ObjectType::Publication);
    }

    #[test]
    fn test_subscription_extract_pattern_dependencies_unquoted() {
        let content =
            r#"CREATE SUBSCRIPTION my_sub CONNECTION 'host=localhost' PUBLICATION my_pub;"#;
        let deps = SubscriptionHandler::extract_pattern_dependencies(content);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "my_pub");
        assert_eq!(deps[0].1, ObjectType::Publication);
    }
}
