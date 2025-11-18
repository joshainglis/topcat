// Soft dependency matcher for automatic dependency type conversion
//
// This module provides functionality to automatically convert certain dependencies from
// `requires` to `exists` based on regex patterns matching both node names and dependency names.
// Both patterns must match (AND logic) for conversion to occur.

use indexmap::IndexMap;
use regex::Regex;

/// Matcher for automatically converting dependencies from `requires` to `exists`
/// based on node name and dependency name patterns
#[derive(Debug)]
pub struct SoftDepsMapper {
    /// Compiled regex patterns: (node_pattern, dependency_pattern)
    /// Using Vec to preserve insertion order for potential first-match-wins behavior
    patterns: Vec<(Regex, Regex)>,
}

impl SoftDepsMapper {
    /// Create a new SoftDepsMapper with the specified pattern mappings
    ///
    /// # Arguments
    /// * `pattern_mappings` - Map of node regex patterns to dependency regex patterns.
    ///   Both patterns must match for a dependency to be converted from `requires` to `exists`.
    ///
    /// # Returns
    /// A Result containing the SoftDepsMapper or an error message if pattern compilation fails
    ///
    /// # Example
    /// ```
    /// use indexmap::IndexMap;
    /// use topcat::soft_deps_matcher::SoftDepsMapper;
    ///
    /// let mut mappings = IndexMap::new();
    /// // Any node in codegen_tmf schema: dependencies on c_tmf become exists
    /// mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());
    ///
    /// let mapper = SoftDepsMapper::new(&mappings).unwrap();
    /// assert!(mapper.should_convert("codegen_tmf.my_func", "c_tmf.schema"));
    /// assert!(!mapper.should_convert("other.func", "c_tmf.schema"));
    /// ```
    pub fn new(pattern_mappings: &IndexMap<String, String>) -> Result<Self, String> {
        let mut patterns = Vec::new();

        for (node_pattern_str, dep_pattern_str) in pattern_mappings {
            let node_regex = Regex::new(node_pattern_str)
                .map_err(|e| format!("Invalid node regex pattern '{node_pattern_str}': {e}"))?;

            let dep_regex = Regex::new(dep_pattern_str).map_err(|e| {
                format!("Invalid dependency regex pattern '{dep_pattern_str}': {e}")
            })?;

            patterns.push((node_regex, dep_regex));
        }

        Ok(Self { patterns })
    }

    /// Check if a dependency should be converted from `requires` to `exists`
    ///
    /// # Arguments
    /// * `node_name` - The name of the node containing the dependency
    /// * `dep_name` - The name of the dependency
    ///
    /// # Returns
    /// `true` if BOTH the node pattern AND dependency pattern match, `false` otherwise
    ///
    /// # Example
    /// ```
    /// use indexmap::IndexMap;
    /// use topcat::soft_deps_matcher::SoftDepsMapper;
    ///
    /// let mut mappings = IndexMap::new();
    /// mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());
    /// let mapper = SoftDepsMapper::new(&mappings).unwrap();
    ///
    /// // Both patterns match
    /// assert!(mapper.should_convert("codegen_tmf.util_func", "c_tmf.ta_headers"));
    ///
    /// // Node matches but dependency doesn't
    /// assert!(!mapper.should_convert("codegen_tmf.util_func", "md_tmf.table"));
    ///
    /// // Dependency matches but node doesn't
    /// assert!(!mapper.should_convert("other.func", "c_tmf.ta_headers"));
    ///
    /// // Neither matches
    /// assert!(!mapper.should_convert("other.func", "md_tmf.table"));
    /// ```
    pub fn should_convert(&self, node_name: &str, dep_name: &str) -> bool {
        // Iterate through patterns in order
        for (node_regex, dep_regex) in &self.patterns {
            // Both patterns must match (AND logic)
            if node_regex.is_match(node_name) && dep_regex.is_match(dep_name) {
                return true;
            }
        }
        false
    }

    /// Check if this mapper has any patterns configured
    ///
    /// # Returns
    /// `true` if at least one pattern is configured, `false` otherwise
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Get the number of configured pattern pairs
    ///
    /// # Returns
    /// The number of node-to-dependency pattern mappings
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mappings() -> IndexMap<String, String> {
        let mut mappings = IndexMap::new();
        // codegen_tmf nodes: c_tmf dependencies become exists
        mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());
        // md_tmf nodes: e_extensions dependencies become exists
        mappings.insert(r"^md_tmf\b".to_string(), r"^e_extensions\b".to_string());
        // c_api nodes: ALL e_* dependencies become exists
        mappings.insert(r"^c_api\b".to_string(), r"^e_\w+".to_string());
        mappings
    }

    #[test]
    fn test_basic_conversion() {
        let mapper = SoftDepsMapper::new(&create_test_mappings()).unwrap();

        // codegen_tmf -> c_tmf: should convert
        assert!(mapper.should_convert("codegen_tmf.my_func", "c_tmf.schema"));
        assert!(mapper.should_convert("codegen_tmf.util_generate", "c_tmf.ta_headers"));

        // md_tmf -> e_extensions: should convert
        assert!(mapper.should_convert("md_tmf.table", "e_extensions.ltree"));

        // c_api -> e_*: should convert
        assert!(mapper.should_convert("c_api.endpoint", "e_extensions.pgcrypto"));
        assert!(mapper.should_convert("c_api.handler", "e_uuid"));
    }

    #[test]
    fn test_both_must_match() {
        let mapper = SoftDepsMapper::new(&create_test_mappings()).unwrap();

        // Node matches but dependency doesn't
        assert!(!mapper.should_convert("codegen_tmf.func", "md_tmf.table"));
        assert!(!mapper.should_convert("md_tmf.table", "c_tmf.schema"));

        // Dependency matches but node doesn't
        assert!(!mapper.should_convert("other.func", "c_tmf.schema"));
        assert!(!mapper.should_convert("dp_users", "e_extensions.ltree"));

        // Neither matches
        assert!(!mapper.should_convert("other.func", "other.dep"));
    }

    #[test]
    fn test_schema_qualified_names() {
        let mapper = SoftDepsMapper::new(&create_test_mappings()).unwrap();

        // Schema-qualified names should work
        assert!(mapper.should_convert(
            "codegen_tmf.util_generate_tmf_list_function",
            "c_tmf.ta_headers"
        ));
    }

    #[test]
    fn test_pattern_order() {
        let mut mappings = IndexMap::new();
        // More specific pattern first
        mappings.insert(r"^codegen_tmf\.util".to_string(), r"^c_tmf\.t".to_string());
        // More general pattern second
        mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());

        let mapper = SoftDepsMapper::new(&mappings).unwrap();

        // Should match first pattern
        assert!(mapper.should_convert("codegen_tmf.util_func", "c_tmf.ta_headers"));

        // Should match second pattern (first doesn't match)
        assert!(mapper.should_convert("codegen_tmf.other_func", "c_tmf.schema"));

        // Node matches first pattern but dep doesn't match first pattern's dep
        // Should still try second pattern and match
        assert!(mapper.should_convert("codegen_tmf.util_func", "c_tmf.schema"));
    }

    #[test]
    fn test_invalid_node_regex() {
        let mut mappings = IndexMap::new();
        mappings.insert(r"(?P<invalid".to_string(), r"^valid".to_string());

        let result = SoftDepsMapper::new(&mappings);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid node regex pattern"));
    }

    #[test]
    fn test_invalid_dep_regex() {
        let mut mappings = IndexMap::new();
        mappings.insert(r"^valid".to_string(), r"(?P<invalid".to_string());

        let result = SoftDepsMapper::new(&mappings);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Invalid dependency regex pattern")
        );
    }

    #[test]
    fn test_empty_mapper() {
        let mappings = IndexMap::new();
        let mapper = SoftDepsMapper::new(&mappings).unwrap();

        assert!(mapper.is_empty());
        assert_eq!(mapper.pattern_count(), 0);
        assert!(!mapper.should_convert("any_node", "any_dep"));
    }

    #[test]
    fn test_pattern_count() {
        let mapper = SoftDepsMapper::new(&create_test_mappings()).unwrap();
        assert_eq!(mapper.pattern_count(), 3);
        assert!(!mapper.is_empty());
    }

    #[test]
    fn test_multiple_patterns_same_node() {
        let mut mappings = IndexMap::new();
        // When same node pattern is used twice, second one replaces first (IndexMap behavior)
        mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());
        mappings.insert(r"^codegen_tmf\b".to_string(), r"^md_tmf\b".to_string());

        let mapper = SoftDepsMapper::new(&mappings).unwrap();

        // Only one pattern exists (second replaced first)
        assert_eq!(mapper.pattern_count(), 1);

        // Should NOT match first pattern (it was replaced)
        assert!(!mapper.should_convert("codegen_tmf.func", "c_tmf.schema"));
        // Should match second pattern (the one that replaced the first)
        assert!(mapper.should_convert("codegen_tmf.func", "md_tmf.table"));
        // Should not match
        assert!(!mapper.should_convert("codegen_tmf.func", "dp_users"));
    }

    #[test]
    fn test_partial_node_name_match() {
        let mut mappings = IndexMap::new();
        // Pattern requires word boundary
        mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());

        let mapper = SoftDepsMapper::new(&mappings).unwrap();

        // Should match: codegen_tmf followed by word boundary (.)
        assert!(mapper.should_convert("codegen_tmf.util", "c_tmf.schema"));

        // Should NOT match: codegen_tmf not followed by word boundary
        // (though this is unlikely in practice with schema.name format)
        assert!(!mapper.should_convert("codegen_tmfx", "c_tmf.schema"));
    }

    #[test]
    fn test_case_sensitivity() {
        let mut mappings = IndexMap::new();
        mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());

        let mapper = SoftDepsMapper::new(&mappings).unwrap();

        // Patterns are case-sensitive by default
        assert!(mapper.should_convert("codegen_tmf.func", "c_tmf.schema"));
        assert!(!mapper.should_convert("Codegen_tmf.func", "c_tmf.schema")); // Capital C
        assert!(!mapper.should_convert("codegen_tmf.func", "C_tmf.schema")); // Capital C
    }
}
