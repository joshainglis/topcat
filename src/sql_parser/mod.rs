//! SQL content analyzer for dependency discovery.
//!
//! This module analyzes SQL files to extract:
//! - Node names from CREATE statements
//! - Dependencies from schema.object references
//! - Type cast dependencies
//! - Subobjects created within a file

mod analyzer;

#[cfg(test)]
mod tests;

use regex::Regex;
use std::collections::HashSet;

pub use analyzer::SqlAnalyzer;

/// Maximum allowed length for user-supplied regex patterns to prevent complexity issues
const MAX_PATTERN_LENGTH: usize = 500;

/// Validate a user-supplied regex pattern for basic safety checks
pub(crate) fn validate_regex_pattern(pattern: &str) -> Result<(), String> {
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
