// Layer mapper for automatic layer assignment based on node name patterns
//
// This module provides functionality to automatically assign layers to nodes based on
// regex patterns matching their names. Patterns are evaluated in order (first match wins).

use indexmap::IndexMap;
use regex::Regex;

/// Mapper for automatically assigning layers to nodes based on name patterns
#[derive(Debug)]
pub struct LayerMapper {
    /// Compiled regex patterns mapped to layer names
    /// Using Vec to preserve insertion order for first-match-wins behavior
    patterns: Vec<(Regex, String)>,
}

impl LayerMapper {
    /// Create a new LayerMapper with the specified pattern-to-layer mappings
    ///
    /// # Arguments
    /// * `pattern_mappings` - Map of regex patterns to layer names. Order matters: first match wins.
    ///
    /// # Returns
    /// A Result containing the LayerMapper or an error message if pattern compilation fails
    pub fn new(pattern_mappings: &IndexMap<String, String>) -> Result<Self, String> {
        let mut patterns = Vec::new();

        for (pattern_str, layer_name) in pattern_mappings {
            let compiled = Regex::new(pattern_str)
                .map_err(|e| format!("Invalid regex pattern '{pattern_str}': {e}"))?;
            patterns.push((compiled, layer_name.clone()));
        }

        Ok(Self { patterns })
    }

    /// Map a node name to a layer using the configured patterns
    ///
    /// # Arguments
    /// * `node_name` - The name of the node to map
    ///
    /// # Returns
    /// `Some(layer_name)` if a pattern matches, `None` if no patterns match
    pub fn map_node_to_layer(&self, node_name: &str) -> Option<String> {
        // Iterate through patterns in order (first match wins)
        for (regex, layer_name) in &self.patterns {
            if regex.is_match(node_name) {
                return Some(layer_name.clone());
            }
        }
        None
    }

    /// Check if this mapper has any patterns configured
    ///
    /// # Returns
    /// `true` if at least one pattern is configured, `false` otherwise
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Get the number of configured patterns
    ///
    /// # Returns
    /// The number of pattern-to-layer mappings
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mappings() -> IndexMap<String, String> {
        let mut mappings = IndexMap::new();
        // Order matters: first match wins
        mappings.insert(r"^\w+\.grants$".to_string(), "grants".to_string());
        mappings.insert(r"^e_extensions\b".to_string(), "init".to_string());
        mappings.insert(r"^codegen_\w+\b".to_string(), "codegen".to_string());
        mappings.insert(r"^d[ipo]_\w+\b".to_string(), "persistence".to_string());
        mappings.insert(r"^(c|md)_\w+\b".to_string(), "contract".to_string());
        mappings
    }

    #[test]
    fn test_basic_mapping() {
        let mapper = LayerMapper::new(&create_test_mappings()).unwrap();

        assert_eq!(
            mapper.map_node_to_layer("e_extensions"),
            Some("init".to_string())
        );
        assert_eq!(
            mapper.map_node_to_layer("codegen_models"),
            Some("codegen".to_string())
        );
        assert_eq!(
            mapper.map_node_to_layer("dp_users"),
            Some("persistence".to_string())
        );
        assert_eq!(
            mapper.map_node_to_layer("di_inventory"),
            Some("persistence".to_string())
        );
        assert_eq!(
            mapper.map_node_to_layer("do_orders"),
            Some("persistence".to_string())
        );
        assert_eq!(
            mapper.map_node_to_layer("c_api"),
            Some("contract".to_string())
        );
        assert_eq!(
            mapper.map_node_to_layer("md_tmf"),
            Some("contract".to_string())
        );
    }

    #[test]
    fn test_first_match_wins() {
        let mapper = LayerMapper::new(&create_test_mappings()).unwrap();

        // "md_tmf.grants" should match "grants" pattern first, not "contract" pattern
        assert_eq!(
            mapper.map_node_to_layer("md_tmf.grants"),
            Some("grants".to_string())
        );

        // "c_api.grants" should also match "grants" first
        assert_eq!(
            mapper.map_node_to_layer("c_api.grants"),
            Some("grants".to_string())
        );
    }

    #[test]
    fn test_no_match() {
        let mapper = LayerMapper::new(&create_test_mappings()).unwrap();

        // Node that doesn't match any pattern
        assert_eq!(mapper.map_node_to_layer("unknown_node"), None);
        assert_eq!(mapper.map_node_to_layer("x_something"), None);
    }

    #[test]
    fn test_invalid_regex() {
        let mut mappings = IndexMap::new();
        mappings.insert(r"(?P<invalid".to_string(), "layer".to_string());

        let result = LayerMapper::new(&mappings);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid regex pattern"));
    }

    #[test]
    fn test_empty_mapper() {
        let mappings = IndexMap::new();
        let mapper = LayerMapper::new(&mappings).unwrap();

        assert!(mapper.is_empty());
        assert_eq!(mapper.pattern_count(), 0);
        assert_eq!(mapper.map_node_to_layer("any_node"), None);
    }

    #[test]
    fn test_pattern_count() {
        let mapper = LayerMapper::new(&create_test_mappings()).unwrap();
        assert_eq!(mapper.pattern_count(), 5);
        assert!(!mapper.is_empty());
    }

    #[test]
    fn test_schema_qualified_names() {
        let mapper = LayerMapper::new(&create_test_mappings()).unwrap();

        // Test schema-qualified names with various patterns
        assert_eq!(
            mapper.map_node_to_layer("schema.grants"),
            Some("grants".to_string())
        );
        assert_eq!(
            mapper.map_node_to_layer("my_schema.grants"),
            Some("grants".to_string())
        );
    }

    #[test]
    fn test_case_sensitivity() {
        let mapper = LayerMapper::new(&create_test_mappings()).unwrap();

        // Patterns are case-sensitive by default
        assert_eq!(
            mapper.map_node_to_layer("e_extensions"),
            Some("init".to_string())
        );
        assert_eq!(mapper.map_node_to_layer("E_extensions"), None); // Capital E doesn't match
    }

    #[test]
    fn test_pattern_order_matters() {
        let mut mappings1 = IndexMap::new();
        mappings1.insert(r"^\w+\.grants$".to_string(), "grants".to_string());
        mappings1.insert(r"^md_\w+".to_string(), "contract".to_string());

        let mapper1 = LayerMapper::new(&mappings1).unwrap();
        assert_eq!(
            mapper1.map_node_to_layer("md_api.grants"),
            Some("grants".to_string())
        );

        // Reverse the order
        let mut mappings2 = IndexMap::new();
        mappings2.insert(r"^md_\w+".to_string(), "contract".to_string());
        mappings2.insert(r"^\w+\.grants$".to_string(), "grants".to_string());

        let mapper2 = LayerMapper::new(&mappings2).unwrap();
        // Now "md_api.grants" matches "contract" pattern first (even though it has .grants)
        assert_eq!(
            mapper2.map_node_to_layer("md_api.grants"),
            Some("contract".to_string())
        );
    }
}
