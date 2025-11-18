use log::{debug, warn};
use regex::{Regex, RegexBuilder};
use std::collections::HashSet;

use crate::sql_config::SqlDiscoveryConfig;

/// Maximum allowed length for user-supplied regex patterns to prevent complexity issues
const MAX_PATTERN_LENGTH: usize = 500;

/// Validate a user-supplied regex pattern for basic safety checks
fn validate_regex_pattern(pattern: &str) -> Result<(), String> {
    // Check pattern length to prevent excessive complexity
    if pattern.len() > MAX_PATTERN_LENGTH {
        return Err(format!(
            "Regex pattern too long ({} chars). Maximum allowed is {} characters.",
            pattern.len(),
            MAX_PATTERN_LENGTH
        ));
    }

    // Check for empty pattern
    if pattern.trim().is_empty() {
        return Err("Regex pattern cannot be empty".to_string());
    }

    // Try to compile the pattern to ensure it's valid
    // Rust's regex crate is resistant to ReDoS, but we still validate for correctness
    Regex::new(pattern).map_err(|e| format!("Invalid regex pattern: {e}"))?;

    Ok(())
}

/// Result of analyzing SQL content
#[derive(Debug, Clone)]
pub struct SqlAnalysisResult {
    /// Discovered node name from CREATE statements
    pub node_name: Option<String>,
    /// Discovered dependencies from SQL content
    pub dependencies: HashSet<String>,
    /// Sub-objects created in this file (to exclude from dependencies)
    pub subobjects: HashSet<String>,
    /// Whether the file creates implicitly referenced objects (CAST, OPERATOR)
    pub has_implicit: bool,
}

impl SqlAnalysisResult {
    pub fn new() -> Self {
        Self {
            node_name: None,
            dependencies: HashSet::new(),
            subobjects: HashSet::new(),
            has_implicit: false,
        }
    }
}

impl Default for SqlAnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}

/// SQL content analyzer
pub struct SqlAnalyzer {
    /// Regex for matching schema.object patterns
    dependency_pattern: Option<Regex>,
    /// Regex for matching CREATE statements to extract node names
    create_pattern: Regex,
    /// Regex for model generation patterns
    model_gen_patterns: Vec<Regex>,
    /// Regex for detecting implicit objects (CAST, OPERATOR)
    implicit_pattern: Regex,
    /// Config for transformations and mappings
    config: SqlDiscoveryConfig,
}

impl SqlAnalyzer {
    /// Create a new SQL analyzer with the given configuration
    pub fn new(config: SqlDiscoveryConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Validate schema pattern if provided
        if let Some(ref schema_pat) = config.schema_pattern {
            validate_regex_pattern(schema_pat)?;
        }

        // Build dependency pattern if schema pattern is provided
        let dependency_pattern = if let Some(ref schema_pat) = config.schema_pattern {
            // Pattern to match schema.object or "schema"."object" or just schema
            let pattern = format!(
                r#"(?:\"?(?P<schema>{schema_pat})\"?\.\"?(?P<name>\w+)\"?|\"?(?P<schema_only>{schema_pat})\"?)"#
            );
            Some(RegexBuilder::new(&pattern).case_insensitive(true).build()?)
        } else {
            None
        };

        // Pattern for CREATE statements
        let create_pattern = if let Some(ref schema_pat) = config.schema_pattern {
            RegexBuilder::new(&format!(
                r#"^\s*CREATE(?:\s+OR\s+REPLACE)?\s+\w+\s+\"?(?P<schema>{schema_pat})\"?(?:\.\"?(?P<name>\w+)\"?)?"#
            ))
            .case_insensitive(true)
            .build()?
        } else {
            // Fallback pattern without schema restriction
            RegexBuilder::new(
                r#"^\s*CREATE(?:\s+OR\s+REPLACE)?\s+\w+\s+\"?(?P<schema>\w+)\"?(?:\.\"?(?P<name>\w+)\"?)?"#,
            )
            .case_insensitive(true)
            .build()?
        };

        // Build model generation patterns with validation
        let model_gen_patterns = config
            .model_gen_patterns
            .iter()
            .filter_map(|pat| {
                // Validate pattern first
                if let Err(e) = validate_regex_pattern(pat) {
                    warn!("Skipping invalid model generation pattern '{pat}': {e}");
                    return None;
                }

                RegexBuilder::new(pat)
                    .case_insensitive(true)
                    .build()
                    .map_err(|e| {
                        warn!("Failed to compile model generation pattern '{pat}': {e}");
                        e
                    })
                    .ok()
            })
            .collect();

        // Pattern for detecting implicit objects (CREATE CAST or CREATE OPERATOR)
        let implicit_pattern = RegexBuilder::new(r"^CREATE\s+(CAST|OPERATOR)\b")
            .case_insensitive(true)
            .build()?;

        Ok(Self {
            dependency_pattern,
            create_pattern,
            model_gen_patterns,
            implicit_pattern,
            config,
        })
    }

    /// Analyze SQL content and extract dependencies
    pub fn analyze(&self, content: &str) -> SqlAnalysisResult {
        let mut result = SqlAnalysisResult::new();

        // Track if we're in a model generation block
        let mut in_model_gen = false;
        let mut model_gen_schema = String::new();
        let mut model_gen_name = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comment lines for dependency extraction
            if !trimmed.starts_with("--") {
                // Remove inline comments for processing
                let clean_line = line.split("--").next().unwrap_or(line);

                // Check for model generation patterns
                for pattern in &self.model_gen_patterns {
                    if pattern.is_match(clean_line) {
                        in_model_gen = true;
                        break;
                    }
                }

                // Extract model generation schema/name if we're in that block
                if in_model_gen {
                    if let Some(schema) = self.extract_model_gen_param(clean_line, "p_schema") {
                        model_gen_schema = schema;
                    }
                    if let Some(name) = self.extract_model_gen_param(clean_line, "p_name") {
                        model_gen_name = name;
                    }

                    // If we have both schema and name, construct the node name
                    if !model_gen_schema.is_empty() && !model_gen_name.is_empty() {
                        result.node_name = Some(format!("{model_gen_schema}.{model_gen_name}"));
                        in_model_gen = false; // Reset for potential next block
                    }
                }

                // Extract node name from CREATE statements
                if result.node_name.is_none() {
                    if let Some(captures) = self.create_pattern.captures(clean_line)
                        && let Some(node_name) = self.node_name_from_captures(&captures)
                    {
                        result.node_name = Some(node_name.clone());
                        // Don't add this as a subobject if it's the first node
                    }
                }
                // If we already have a node name, subsequent CREATEs are subobjects
                else if let Some(captures) = self.create_pattern.captures(clean_line)
                    && let Some(subobject_name) = self.node_name_from_captures(&captures)
                {
                    result.subobjects.insert(subobject_name);
                }

                // Extract dependencies from the line
                if let Some(deps) = self.extract_dependencies(clean_line) {
                    result.dependencies.extend(deps);
                }

                // Check for type cast dependencies
                if let Some(type_deps) = self.extract_type_dependencies(clean_line) {
                    result.dependencies.extend(type_deps);
                }

                // Check for implicit objects (CREATE CAST, CREATE OPERATOR)
                if self.implicit_pattern.is_match(clean_line.trim()) {
                    result.has_implicit = true;
                }
            }
        }

        // Add implicit schema dependency when node name contains a schema component
        // E.g., if node_name is "md_tmf.util_to_snake_case", add "md_tmf" as a dependency
        // because you can't create objects in a schema that doesn't exist yet
        if let Some(ref node_name) = result.node_name
            && let Some(dot_pos) = node_name.find('.')
        {
            let schema = &node_name[..dot_pos];
            result.dependencies.insert(schema.to_string());
        }

        debug!(
            "SQL analysis result: node={:?}, deps={:?}, subobjects={:?}",
            result.node_name, result.dependencies, result.subobjects
        );

        result
    }

    /// Extract dependencies using the configured pattern
    fn extract_dependencies(&self, line: &str) -> Option<HashSet<String>> {
        let pattern = self.dependency_pattern.as_ref()?;

        let mut deps = HashSet::new();

        for captures in pattern.captures_iter(line) {
            if let Some(node_name) = self.node_name_from_captures(&captures) {
                deps.insert(node_name);
            }
        }

        if deps.is_empty() { None } else { Some(deps) }
    }

    /// Extract type cast dependencies (e.g., ::TSTZRANGE)
    fn extract_type_dependencies(&self, line: &str) -> Option<HashSet<String>> {
        if self.config.type_mappings.is_empty() {
            return None;
        }

        let mut deps = HashSet::new();
        let cast_pattern = Regex::new(r"::(\w+)").ok()?;

        for captures in cast_pattern.captures_iter(line) {
            if let Some(type_name) = captures.get(1) {
                let type_name = type_name.as_str().to_uppercase();
                if let Some(mapped_name) = self.config.type_mappings.get(&type_name) {
                    deps.insert(mapped_name.clone());
                }
            }
        }

        if deps.is_empty() { None } else { Some(deps) }
    }

    /// Extract a parameter value from a model generation call
    /// Example: p_schema := 'c_tmf' -> returns "c_tmf"
    fn extract_model_gen_param(&self, line: &str, param_name: &str) -> Option<String> {
        let pattern = format!(r#"{param_name}\s*(?::=|=>)\s*['"]([^'"]+)['"]"#);
        let re = Regex::new(&pattern).ok()?;
        re.captures(line)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// Convert regex captures to a node name
    fn node_name_from_captures(&self, captures: &regex::Captures) -> Option<String> {
        // Try to get schema.name format
        if let (Some(schema), Some(name)) = (captures.name("schema"), captures.name("name")) {
            let schema = schema.as_str().to_lowercase();
            let mut name = name.as_str().to_lowercase();

            // Apply transformations
            name = self.apply_transformations(&schema, &name);

            return Some(format!("{schema}.{name}"));
        }

        // Try schema-only format
        if let Some(schema_only) = captures.name("schema_only") {
            return Some(schema_only.as_str().to_lowercase());
        }

        // Fallback: just schema name
        if let Some(schema) = captures.name("schema") {
            return Some(schema.as_str().to_lowercase());
        }

        None
    }

    /// Apply configured transformations to object names
    fn apply_transformations(&self, schema: &str, name: &str) -> String {
        let mut result = name.to_string();

        // Strip configured suffixes
        for suffix in &self.config.strip_suffixes {
            if result.ends_with(suffix) {
                result = result[..result.len() - suffix.len()].to_string();
            }
        }

        // Apply extension mappings
        if let Some(extension) = self.config.extension_mappings.get(&result) {
            return extension.clone();
        }

        // Special case transformations can be added here
        // Example from the Python script: c_tmf.tstzrange -> c_tmf.t_time_period
        if schema == "c_tmf" && result == "tstzrange" {
            return "t_time_period".to_string();
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_config() -> SqlDiscoveryConfig {
        SqlDiscoveryConfig {
            enabled: true,
            schema_pattern: Some(r"(?:test|e|c|d[pio])_\w+".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_extract_create_statement() {
        let config = make_simple_config();
        let analyzer = SqlAnalyzer::new(config).unwrap();

        let sql = "CREATE TABLE c_test.my_table (id INT);";
        let result = analyzer.analyze(sql);

        assert_eq!(result.node_name, Some("c_test.my_table".to_string()));
    }

    #[test]
    fn test_extract_dependencies() {
        let config = make_simple_config();
        let analyzer = SqlAnalyzer::new(config).unwrap();

        let sql = "SELECT * FROM c_test.table1 JOIN c_test.table2 ON true;";
        let result = analyzer.analyze(sql);

        assert!(result.dependencies.contains("c_test.table1"));
        assert!(result.dependencies.contains("c_test.table2"));
    }

    #[test]
    fn test_subobjects() {
        let config = make_simple_config();
        let analyzer = SqlAnalyzer::new(config).unwrap();

        let sql = r#"
CREATE SCHEMA c_test;
CREATE TABLE c_test.table1 (id INT);
CREATE TABLE c_test.table2 (id INT);
"#;
        let result = analyzer.analyze(sql);

        assert_eq!(result.node_name, Some("c_test".to_string()));
        assert!(result.subobjects.contains("c_test.table1"));
        assert!(result.subobjects.contains("c_test.table2"));
    }

    #[test]
    fn test_type_mappings() {
        let mut config = make_simple_config();
        config
            .type_mappings
            .insert("TSTZRANGE".to_string(), "c_tmf.t_time_period".to_string());

        let analyzer = SqlAnalyzer::new(config).unwrap();

        let sql = "SELECT '2024-01-01'::TSTZRANGE;";
        let result = analyzer.analyze(sql);

        assert!(result.dependencies.contains("c_tmf.t_time_period"));
    }

    #[test]
    fn test_strip_suffixes() {
        let mut config = make_simple_config();
        config.strip_suffixes.push("_or_ref".to_string());

        let analyzer = SqlAnalyzer::new(config).unwrap();

        let sql = "CREATE TABLE c_test.my_table_or_ref (id INT);";
        let result = analyzer.analyze(sql);

        // The suffix should be stripped from the node name
        assert_eq!(result.node_name, Some("c_test.my_table".to_string()));
    }

    #[test]
    fn test_ignore_comments() {
        let config = make_simple_config();
        let analyzer = SqlAnalyzer::new(config).unwrap();

        let sql = r#"
-- This references c_test.fake_table but it's in a comment
SELECT * FROM c_test.real_table;
"#;
        let result = analyzer.analyze(sql);

        assert!(!result.dependencies.contains("c_test.fake_table"));
        assert!(result.dependencies.contains("c_test.real_table"));
    }

    #[test]
    fn test_model_generation() {
        let mut config = make_simple_config();
        config
            .model_gen_patterns
            .push(r"codegen_tmf\.proc_(?:make_model|combine_enums)".to_string());

        let analyzer = SqlAnalyzer::new(config).unwrap();

        let sql = r#"
CALL codegen_tmf.proc_make_model(
    p_schema := 'c_test',
    p_name := 'my_model'
);
"#;
        let result = analyzer.analyze(sql);

        assert_eq!(result.node_name, Some("c_test.my_model".to_string()));
    }

    #[test]
    fn test_extension_mappings() {
        let mut config = make_simple_config();
        // Configure extension mappings similar to the Python script
        config
            .extension_mappings
            .insert("nlevel".to_string(), "ltree".to_string());
        config
            .extension_mappings
            .insert("lca".to_string(), "ltree".to_string());
        config
            .extension_mappings
            .insert("index".to_string(), "ltree".to_string());
        config
            .extension_mappings
            .insert("digest".to_string(), "pgcrypto".to_string());

        let analyzer = SqlAnalyzer::new(config).unwrap();

        // Test that e_extensions.nlevel gets mapped to e_extensions.ltree
        let sql = "SELECT e_extensions.nlevel(path) FROM my_table;";
        let result = analyzer.analyze(sql);

        assert!(
            result.dependencies.contains("e_extensions.ltree"),
            "Expected dependency on e_extensions.ltree, got {:?}",
            result.dependencies
        );
        assert!(
            !result.dependencies.contains("e_extensions.nlevel"),
            "Should not have dependency on e_extensions.nlevel, got {:?}",
            result.dependencies
        );

        // Test digest -> pgcrypto mapping
        let sql2 = "SELECT e_extensions.digest('test', 'sha256');";
        let result2 = analyzer.analyze(sql2);

        assert!(
            result2.dependencies.contains("e_extensions.pgcrypto"),
            "Expected dependency on e_extensions.pgcrypto, got {:?}",
            result2.dependencies
        );
    }

    #[test]
    fn test_implicit_schema_dependency() {
        let config = make_simple_config();
        let analyzer = SqlAnalyzer::new(config).unwrap();

        // Test that creating a function in a schema adds the schema as a dependency
        let sql = r#"
CREATE OR REPLACE FUNCTION c_test.my_function(param1 INT)
RETURNS INT AS $$
BEGIN
    RETURN param1 + 1;
END;
$$ LANGUAGE plpgsql;
"#;
        let result = analyzer.analyze(sql);

        assert_eq!(
            result.node_name,
            Some("c_test.my_function".to_string()),
            "Node name should be c_test.my_function"
        );
        assert!(
            result.dependencies.contains("c_test"),
            "Should have implicit dependency on c_test schema, got {:?}",
            result.dependencies
        );
    }

    #[test]
    fn test_implicit_schema_dependency_for_table() {
        let config = make_simple_config();
        let analyzer = SqlAnalyzer::new(config).unwrap();

        // Test that creating a table in a schema adds the schema as a dependency
        let sql = "CREATE TABLE c_test.my_table (id INT, name TEXT);";
        let result = analyzer.analyze(sql);

        assert_eq!(
            result.node_name,
            Some("c_test.my_table".to_string()),
            "Node name should be c_test.my_table"
        );
        assert!(
            result.dependencies.contains("c_test"),
            "Should have implicit dependency on c_test schema, got {:?}",
            result.dependencies
        );
    }

    #[test]
    fn test_create_cast_detection() {
        let config = make_simple_config();
        let analyzer = SqlAnalyzer::new(config).unwrap();

        let sql = r#"
CREATE CAST (text AS c_test.my_type)
WITH FUNCTION c_test.text_to_my_type(text);
"#;
        let result = analyzer.analyze(sql);

        assert!(
            result.has_implicit,
            "Should detect CREATE CAST as implicit object"
        );
    }

    #[test]
    fn test_create_operator_detection() {
        let config = make_simple_config();
        let analyzer = SqlAnalyzer::new(config).unwrap();

        let sql = r#"
CREATE OPERATOR === (
    LEFTARG = text,
    RIGHTARG = text,
    FUNCTION = texteq
);
"#;
        let result = analyzer.analyze(sql);

        assert!(
            result.has_implicit,
            "Should detect CREATE OPERATOR as implicit object"
        );
    }

    #[test]
    fn test_create_cast_case_insensitive() {
        let config = make_simple_config();
        let analyzer = SqlAnalyzer::new(config).unwrap();

        // Test with lowercase
        let sql_lower = "create cast (text as c_test.my_type) with function c_test.converter;";
        let result_lower = analyzer.analyze(sql_lower);
        assert!(
            result_lower.has_implicit,
            "Should detect lowercase CREATE CAST"
        );

        // Test with mixed case
        let sql_mixed = "Create Cast (text as c_test.my_type) With Function c_test.converter;";
        let result_mixed = analyzer.analyze(sql_mixed);
        assert!(
            result_mixed.has_implicit,
            "Should detect mixed case CREATE CAST"
        );
    }

    #[test]
    fn test_no_implicit_for_regular_create() {
        let config = make_simple_config();
        let analyzer = SqlAnalyzer::new(config).unwrap();

        // Regular CREATE TABLE should not be marked as implicit
        let sql = "CREATE TABLE c_test.my_table (id INT);";
        let result = analyzer.analyze(sql);

        assert!(
            !result.has_implicit,
            "Regular CREATE TABLE should not be marked as implicit"
        );
    }
}
