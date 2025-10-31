// Root node matcher for protecting entry points from dead branch analysis
//
// This module provides functionality to mark certain files as "root nodes" or "entry points"
// that should never be considered dead, regardless of whether other files depend on them.

use glob::Pattern as GlobPattern;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Matcher for identifying root nodes (entry points) that should be protected
/// from dead branch analysis
#[derive(Debug)]
pub struct RootNodeMatcher {
    /// Specific node names to treat as roots (exact matches)
    specific_nodes: HashSet<String>,

    /// Glob patterns for file paths (e.g., "**/api/*.sql")
    glob_patterns: Vec<(String, GlobPattern)>,

    /// Compiled regex patterns for node names (e.g., "^api_.*")
    regex_patterns: Vec<(String, Regex)>,

    /// Directory prefixes - all files under these paths are roots
    directory_roots: Vec<PathBuf>,
}

impl RootNodeMatcher {
    /// Create a new RootNodeMatcher with the specified patterns
    ///
    /// # Arguments
    /// * `specific_nodes` - Exact node names to protect
    /// * `glob_patterns` - Glob patterns for file paths
    /// * `regex_patterns` - Regex patterns for node names
    /// * `directory_roots` - Directories where all files are roots
    ///
    /// # Returns
    /// A Result containing the RootNodeMatcher or an error message if pattern compilation fails
    pub fn new(
        specific_nodes: Vec<String>,
        glob_patterns: Vec<String>,
        regex_patterns: Vec<String>,
        directory_roots: Vec<PathBuf>,
    ) -> Result<Self, String> {
        // Compile glob patterns
        let compiled_globs: Result<Vec<(String, GlobPattern)>, _> = glob_patterns
            .iter()
            .map(|pattern| {
                GlobPattern::new(pattern)
                    .map(|compiled| (pattern.clone(), compiled))
                    .map_err(|e| format!("Invalid glob pattern '{pattern}': {e}"))
            })
            .collect();

        let compiled_globs = compiled_globs?;

        // Compile regex patterns
        let compiled_regex: Result<Vec<(String, Regex)>, _> = regex_patterns
            .iter()
            .map(|pattern| {
                Regex::new(pattern)
                    .map(|compiled| (pattern.clone(), compiled))
                    .map_err(|e| format!("Invalid regex pattern '{pattern}': {e}"))
            })
            .collect();

        let compiled_regex = compiled_regex?;

        Ok(Self {
            specific_nodes: specific_nodes.into_iter().collect(),
            glob_patterns: compiled_globs,
            regex_patterns: compiled_regex,
            directory_roots,
        })
    }

    /// Check if a node should be treated as a root (protected from deletion)
    ///
    /// # Arguments
    /// * `node_name` - The name of the node to check
    /// * `file_path` - The file path of the node
    ///
    /// # Returns
    /// `true` if the node should be protected, `false` otherwise
    pub fn is_root(&self, node_name: &str, file_path: &Path) -> bool {
        // Check specific nodes (exact match)
        if self.specific_nodes.contains(node_name) {
            return true;
        }

        // Check regex patterns on node name
        for (_pattern_str, regex) in &self.regex_patterns {
            if regex.is_match(node_name) {
                return true;
            }
        }

        // Check glob patterns on file path
        for (_pattern_str, glob_matcher) in &self.glob_patterns {
            if glob_matcher.matches_path(file_path) {
                return true;
            }
        }

        // Check directory roots
        for root_dir in &self.directory_roots {
            if file_path.starts_with(root_dir) {
                return true;
            }
        }

        false
    }

    /// Filter a set of nodes to only include non-root nodes
    ///
    /// # Arguments
    /// * `nodes` - Set of node names to filter
    /// * `node_to_path` - Mapping from node names to file paths
    ///
    /// # Returns
    /// A new HashSet containing only the non-root nodes
    pub fn filter_non_roots(
        &self,
        nodes: &HashSet<String>,
        node_to_path: &HashMap<String, PathBuf>,
    ) -> HashSet<String> {
        nodes
            .iter()
            .filter(|node_name| {
                if let Some(path) = node_to_path.get(*node_name) {
                    !self.is_root(node_name, path)
                } else {
                    // Keep if path not found (shouldn't happen in practice)
                    true
                }
            })
            .cloned()
            .collect()
    }

    /// Check if this matcher has any patterns configured
    ///
    /// # Returns
    /// `true` if at least one pattern type is configured, `false` otherwise
    pub fn is_empty(&self) -> bool {
        self.specific_nodes.is_empty()
            && self.glob_patterns.is_empty()
            && self.regex_patterns.is_empty()
            && self.directory_roots.is_empty()
    }

    /// Get statistics about configured patterns
    ///
    /// # Returns
    /// A tuple of (specific_nodes_count, glob_patterns_count, regex_patterns_count, directory_roots_count)
    pub fn pattern_counts(&self) -> (usize, usize, usize, usize) {
        (
            self.specific_nodes.len(),
            self.glob_patterns.len(),
            self.regex_patterns.len(),
            self.directory_roots.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_specific_nodes() {
        let matcher = RootNodeMatcher::new(
            vec!["api_main".to_string(), "worker_main".to_string()],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        assert!(matcher.is_root("api_main", Path::new("sql/api_main.sql")));
        assert!(matcher.is_root("worker_main", Path::new("sql/worker_main.sql")));
        assert!(!matcher.is_root("other", Path::new("sql/other.sql")));
    }

    #[test]
    fn test_glob_pattern() {
        let matcher =
            RootNodeMatcher::new(vec![], vec!["**/api/*.sql".to_string()], vec![], vec![]).unwrap();

        assert!(matcher.is_root("foo", Path::new("sql/api/endpoint.sql")));
        assert!(matcher.is_root("bar", Path::new("nested/api/handler.sql")));
        assert!(!matcher.is_root("baz", Path::new("sql/other/file.sql")));
    }

    #[test]
    fn test_regex_pattern() {
        let matcher = RootNodeMatcher::new(
            vec![],
            vec![],
            vec!["^api_.*".to_string(), "^worker_.*".to_string()],
            vec![],
        )
        .unwrap();

        assert!(matcher.is_root("api_endpoint", Path::new("sql/file.sql")));
        assert!(matcher.is_root("worker_job", Path::new("sql/file.sql")));
        assert!(!matcher.is_root("helper_function", Path::new("sql/file.sql")));
    }

    #[test]
    fn test_directory_roots() {
        let matcher = RootNodeMatcher::new(
            vec![],
            vec![],
            vec![],
            vec![PathBuf::from("sql/entry_points")],
        )
        .unwrap();

        assert!(matcher.is_root("foo", Path::new("sql/entry_points/main.sql")));
        assert!(matcher.is_root("bar", Path::new("sql/entry_points/subdir/file.sql")));
        assert!(!matcher.is_root("baz", Path::new("sql/other/file.sql")));
    }

    #[test]
    fn test_combined_patterns() {
        let matcher = RootNodeMatcher::new(
            vec!["explicit_root".to_string()],
            vec!["**/migrations/*.sql".to_string()],
            vec!["^api_.*".to_string()],
            vec![PathBuf::from("sql/entry_points")],
        )
        .unwrap();

        // Test each pattern type
        assert!(matcher.is_root("explicit_root", Path::new("anywhere.sql")));
        assert!(matcher.is_root("anything", Path::new("sql/migrations/001.sql")));
        assert!(matcher.is_root("api_endpoint", Path::new("sql/endpoints.sql")));
        assert!(matcher.is_root("anything", Path::new("sql/entry_points/main.sql")));

        // Test non-matching
        assert!(!matcher.is_root("helper", Path::new("sql/helpers/util.sql")));
    }

    #[test]
    fn test_filter_non_roots() {
        let matcher =
            RootNodeMatcher::new(vec!["protected".to_string()], vec![], vec![], vec![]).unwrap();

        let nodes: HashSet<String> = vec![
            "protected".to_string(),
            "unprotected1".to_string(),
            "unprotected2".to_string(),
        ]
        .into_iter()
        .collect();

        let node_to_path: HashMap<String, PathBuf> = vec![
            ("protected".to_string(), PathBuf::from("sql/protected.sql")),
            (
                "unprotected1".to_string(),
                PathBuf::from("sql/unprotected1.sql"),
            ),
            (
                "unprotected2".to_string(),
                PathBuf::from("sql/unprotected2.sql"),
            ),
        ]
        .into_iter()
        .collect();

        let filtered = matcher.filter_non_roots(&nodes, &node_to_path);

        assert_eq!(filtered.len(), 2);
        assert!(!filtered.contains("protected"));
        assert!(filtered.contains("unprotected1"));
        assert!(filtered.contains("unprotected2"));
    }

    #[test]
    fn test_invalid_glob_pattern() {
        let result = RootNodeMatcher::new(
            vec![],
            vec!["[invalid".to_string()], // Invalid glob
            vec![],
            vec![],
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid glob pattern"));
    }

    #[test]
    fn test_invalid_regex_pattern() {
        let result = RootNodeMatcher::new(
            vec![],
            vec![],
            vec!["(?P<invalid".to_string()], // Invalid regex
            vec![],
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid regex pattern"));
    }

    #[test]
    fn test_is_empty() {
        let empty_matcher = RootNodeMatcher::new(vec![], vec![], vec![], vec![]).unwrap();
        assert!(empty_matcher.is_empty());

        let non_empty_matcher =
            RootNodeMatcher::new(vec!["node".to_string()], vec![], vec![], vec![]).unwrap();
        assert!(!non_empty_matcher.is_empty());
    }

    #[test]
    fn test_pattern_counts() {
        let matcher = RootNodeMatcher::new(
            vec!["node1".to_string(), "node2".to_string()],
            vec!["*.sql".to_string()],
            vec!["^api_.*".to_string(), "^worker_.*".to_string()],
            vec![PathBuf::from("sql/")],
        )
        .unwrap();

        let (specific, glob, regex, dirs) = matcher.pattern_counts();
        assert_eq!(specific, 2);
        assert_eq!(glob, 1);
        assert_eq!(regex, 2);
        assert_eq!(dirs, 1);
    }
}
