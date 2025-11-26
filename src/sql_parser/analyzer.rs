//! SQL content analyzer implementation.

use log::{debug, warn};
use regex::{Regex, RegexBuilder};
use std::collections::{HashMap, HashSet};

use super::{validate_regex_pattern, SqlAnalysisResult};
use crate::sql_config::SqlDiscoveryConfig;

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
    /// Regex for matching type casts (::TYPE)
    cast_pattern: Option<Regex>,
    /// Pre-compiled patterns for model generation parameter extraction
    model_gen_param_patterns: HashMap<String, Regex>,
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

        // Pre-compile cast pattern if type mappings are configured
        let cast_pattern = if !config.type_mappings.is_empty() {
            Some(Regex::new(r"::(\w+)")?)
        } else {
            None
        };

        // Pre-compile model generation parameter patterns
        let mut model_gen_param_patterns = HashMap::new();
        for param_name in &["p_schema", "p_name"] {
            let pattern = format!(r#"{param_name}\s*(?::=|=>)\s*['"]([^'"]+)['"]"#);
            model_gen_param_patterns.insert(param_name.to_string(), Regex::new(&pattern)?);
        }

        Ok(Self {
            dependency_pattern,
            create_pattern,
            model_gen_patterns,
            implicit_pattern,
            cast_pattern,
            model_gen_param_patterns,
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

                // Skip dependency extraction from COMMENT ON statements and string literals
                // COMMENT ON statements often contain documentation that mentions tables/schemas
                // but these are not actual execution dependencies
                let is_comment_statement = clean_line
                    .trim_start()
                    .to_uppercase()
                    .starts_with("COMMENT ON");

                if !is_comment_statement {
                    // Extract dependencies from the line
                    if let Some(deps) = self.extract_dependencies(clean_line) {
                        result.dependencies.extend(deps);
                    }

                    // Check for type cast dependencies
                    if let Some(type_deps) = self.extract_type_dependencies(clean_line) {
                        result.dependencies.extend(type_deps);
                    }
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
        // Early return if no type mappings configured
        let cast_pattern = self.cast_pattern.as_ref()?;

        // Early return if line doesn't contain any casts
        if !line.contains("::") {
            return None;
        }

        let mut deps = HashSet::new();

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
        let re = self.model_gen_param_patterns.get(param_name)?;
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
