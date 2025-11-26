//! File node representation for dependency graph.
//!
//! This module provides the `FileNode` struct that represents a file in the dependency graph,
//! along with its metadata (name, dependencies, layer, etc.).
//!
//! ## Module Structure
//!
//! - [`FileNode`] - The main struct representing a file node
//! - [`NameSource`] - Enum indicating where the node name came from
//! - [`parsing`] - File parsing and header extraction
//! - [`soft_deps`] - Soft dependency handling and mapping
//! - [`filename`] - Filename suggestions and rename detection

mod filename;
mod parsing;
mod soft_deps;

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;

use crate::soft_deps_matcher::SoftDepsMapper;

// Re-export parsing function
pub use parsing::from_file;

/// A node in the dependency graph representing a single file.
#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub deps: HashSet<String>,
    pub layer: String,
    pub layer_is_fallback: bool,
    pub ensure_exists: HashSet<String>,
    /// Dependencies discovered from SQL content analysis
    pub discovered_deps: Option<HashSet<String>>,
    /// Manual dependencies that should override discovered ones (marked with ! prefix)
    pub override_deps: HashSet<String>,
    /// Source of the node name (manual header vs discovered from SQL)
    pub name_source: NameSource,
    /// Schema extracted from node name (e.g., "my_schema" from "my_schema.table")
    pub schema: Option<String>,
    /// Mark nodes that are implicitly referenced (e.g., CAST, OPERATOR objects)
    /// These nodes should be protected from dead-branch cleanup even when they have no explicit dependents
    pub implicit: bool,
    /// Manual header flag - when true, preserves original headers and uses ---tc: prefix for auto-generated
    pub manual: bool,
    /// Original headers to preserve (used when manual=true)
    pub original_headers: Option<String>,
    /// Soft-deps flag - when true, convert all dependencies to exists type
    pub soft_deps: bool,
    /// Soft dependency mapper for selective conversion of dependencies to exists type
    /// based on node name and dependency name patterns
    pub soft_deps_mapper: Option<Rc<SoftDepsMapper>>,
    /// Final or initial marker to preserve (e.g., "final", "initial")
    pub final_initial: Option<String>,
    /// Original node_name header line to preserve
    pub original_node_name_header: Option<String>,
}

/// Source of node name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameSource {
    /// Name from file header
    Header,
    /// Name discovered from SQL CREATE statement
    Discovered,
}

// Implementing PartialEq for equality comparisons
impl PartialEq for FileNode {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for FileNode {}

impl PartialOrd for FileNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl Hash for FileNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Display for FileNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl FileNode {
    pub fn new(
        name: String,
        path: PathBuf,
        deps: HashSet<String>,
        layer: String,
        layer_is_fallback: bool,
        ensure_exists: HashSet<String>,
    ) -> FileNode {
        // Extract schema from name if present (e.g., "schema.table" -> Some("schema"))
        let schema = Self::extract_schema(&name);

        FileNode {
            name,
            path,
            deps,
            layer,
            layer_is_fallback,
            ensure_exists,
            discovered_deps: None,
            override_deps: HashSet::new(),
            name_source: NameSource::Header,
            schema,
            implicit: false,
            manual: false,
            original_headers: None,
            soft_deps: false,
            soft_deps_mapper: None,
            final_initial: None,
            original_node_name_header: None,
        }
    }

    /// Extract schema name from node name
    /// Supports patterns like "schema.table", "schema_table", "schema::table"
    /// Returns None if no schema pattern is detected
    pub fn extract_schema(name: &str) -> Option<String> {
        if let Some(idx) = name.find('.') {
            return Some(name[..idx].to_string());
        }

        None
    }

    /// Parse a file and create a FileNode from its headers.
    ///
    /// This is a convenience method that delegates to [`from_file`].
    pub fn from_file(
        comment_str: &str,
        path: &PathBuf,
        layers: &[String],
        fallback_layer: &str,
        layer_mapper: Option<&crate::layer_mapper::LayerMapper>,
    ) -> Result<FileNode, crate::exceptions::FileNodeError> {
        parsing::from_file(comment_str, path, layers, fallback_layer, layer_mapper)
    }
}
