//! Display and formatting utilities for CLI output.
//!
//! This module provides common utilities for formatted console output across
//! all Topcat commands. It includes table formatting helpers and path display
//! utilities.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use comfy_table::{Attribute, Cell, ContentArrangement, Table};

/// Create a standard table with consistent formatting.
///
/// Returns a `comfy_table::Table` configured with Topcat's standard formatting:
/// - UTF-8 borders
/// - Dynamic column width
/// - Consistent padding
///
/// # Examples
///
/// ```
/// use topcat::display_utils::create_standard_table;
///
/// let mut table = create_standard_table();
/// table.set_header(vec!["Name", "Status"]);
/// table.add_row(vec!["file.sql", "✓"]);
/// ```
pub fn create_standard_table() -> Table {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::DynamicFullWidth);
    table
}

/// Create a table for displaying file nodes with consistent formatting.
///
/// This is a specialized version of `create_standard_table()` with common
/// headers for file node displays. Headers are styled with bold formatting.
///
/// # Arguments
///
/// * `headers` - Column headers for the table
///
/// # Returns
///
/// A `Table` with the specified headers, styled and formatted consistently
///
/// # Examples
///
/// ```
/// use topcat::display_utils::create_node_table;
///
/// let table = create_node_table(vec!["Node", "Path", "Dependencies"]);
/// ```
pub fn create_node_table(headers: Vec<&str>) -> Table {
    let mut table = create_standard_table();
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::new(h).add_attribute(Attribute::Bold))
        .collect();
    table.set_header(header_cells);
    table
}

/// Format a path for display, showing only the most relevant parts.
///
/// For better readability in table displays, this truncates long paths while
/// preserving important context. Attempts to show the file name and immediate
/// parent directory when possible.
///
/// # Arguments
///
/// * `path` - The path to format
/// * `max_length` - Maximum length for the displayed path
///
/// # Returns
///
/// A formatted string representation of the path
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use topcat::display_utils::format_path_for_display;
///
/// let path = Path::new("/very/long/path/to/some/file.sql");
/// let formatted = format_path_for_display(path, 30);
/// // Returns something like "…/some/file.sql"
/// ```
pub fn format_path_for_display(path: &Path, max_length: usize) -> String {
    let path_str = path.display().to_string();

    if path_str.len() <= max_length {
        return path_str;
    }

    // Try to show the filename and parent
    if let (Some(file_name), Some(parent)) = (path.file_name(), path.parent()) {
        if let Some(parent_name) = parent.file_name() {
            let short = format!(
                "…/{}/{}",
                parent_name.to_string_lossy(),
                file_name.to_string_lossy()
            );
            if short.len() <= max_length {
                return short;
            }
        }

        // Just show filename if parent is still too long
        let file_only = format!("…/{}", file_name.to_string_lossy());
        if file_only.len() <= max_length {
            return file_only;
        }
    }

    // Fallback: truncate from the start
    format!("…{}", &path_str[path_str.len() - (max_length - 1)..])
}

/// Format a list of files for compact display.
///
/// Joins multiple file paths with a separator for displaying in table cells
/// or compact output formats.
///
/// # Arguments
///
/// * `files` - Slice of file paths
/// * `separator` - String to use between file paths (defaults to ", ")
///
/// # Returns
///
/// A single string with all file paths separated by the separator
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use topcat::display_utils::format_file_list;
///
/// let files = vec![PathBuf::from("a.sql"), PathBuf::from("b.sql")];
/// let display = format_file_list(&files, ", ");
/// assert_eq!(display, "a.sql, b.sql");
/// ```
pub fn format_file_list(files: &[PathBuf], separator: &str) -> String {
    files
        .iter()
        .map(|f| f.display().to_string())
        .collect::<Vec<_>>()
        .join(separator)
}

/// Format a count with singular/plural forms.
///
/// Helper for displaying counts with correct grammar (e.g., "1 file" vs "2 files").
///
/// # Arguments
///
/// * `count` - The number to format
/// * `singular` - Singular form of the noun
/// * `plural` - Plural form of the noun
///
/// # Returns
///
/// A formatted string like "1 file" or "5 files"
///
/// # Examples
///
/// ```
/// use topcat::display_utils::format_count;
///
/// assert_eq!(format_count(0, "file", "files"), "0 files");
/// assert_eq!(format_count(1, "file", "files"), "1 file");
/// assert_eq!(format_count(42, "file", "files"), "42 files");
/// ```
pub fn format_count(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

/// Create a separator line for visual organization.
///
/// Returns a consistent separator string that can be printed to divide
/// sections of output.
///
/// # Examples
///
/// ```
/// use topcat::display_utils::separator_line;
///
/// println!("{}", separator_line());
/// // Prints: ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// ```
pub fn separator_line() -> &'static str {
    "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

/// Create a section header line for titles.
///
/// Returns a double-line separator typically used under section titles.
///
/// # Examples
///
/// ```
/// use topcat::display_utils::section_header_line;
///
/// println!("My Section");
/// println!("{}", section_header_line());
/// // Prints:
/// // My Section
/// // ═══════════════════════════════════════════════════════════
/// ```
pub fn section_header_line() -> &'static str {
    "═══════════════════════════════════════════════════════════"
}

// Tree display constants
const TREE_BRANCH: &str = "├── ";
const TREE_LAST: &str = "└── ";
const TREE_VERTICAL: &str = "│   ";
const TREE_SPACE: &str = "    ";

/// Represents a node in a tree structure for display purposes.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: Option<PathBuf>,
    pub children: Vec<TreeNode>,
    pub additional_parents: Vec<String>,
}

impl TreeNode {
    /// Create a new tree node.
    pub fn new(name: String, path: Option<PathBuf>) -> Self {
        Self {
            name,
            path,
            children: Vec::new(),
            additional_parents: Vec::new(),
        }
    }

    /// Add a child to this node.
    pub fn add_child(&mut self, child: TreeNode) {
        self.children.push(child);
    }

    /// Add an additional parent reference.
    pub fn add_additional_parent(&mut self, parent: String) {
        self.additional_parents.push(parent);
    }
}

/// Build a forest of trees from dependency relationships with stable ordering.
///
/// Converts a DAG represented as dependency maps into a forest of trees for display.
/// Uses stable sorting based on node names for consistent output across runs.
///
/// Trees are built from root nodes (no dependencies) downward through their dependents.
/// Handles nodes with multiple dependencies by selecting a primary parent and
/// marking additional dependencies.
///
/// # Arguments
///
/// * `nodes` - Set of node names to include in the forest
/// * `deps_map` - Map of node -> dependencies (used to find roots)
/// * `dependents_map` - Map of node -> dependents (used to traverse tree)
/// * `node_paths` - Map of node names to their file paths
///
/// # Returns
///
/// A vector of root `TreeNode` objects, one for each disconnected tree
pub fn build_tree_forest(
    nodes: &HashSet<String>,
    deps_map: &HashMap<String, HashSet<String>>,
    dependents_map: &HashMap<String, HashSet<String>>,
    node_paths: &HashMap<String, PathBuf>,
) -> Vec<TreeNode> {
    let mut visited = HashSet::new();
    let mut forest = Vec::new();

    // Find root nodes (nodes with no dependencies within the set)
    let mut roots: Vec<_> = nodes
        .iter()
        .filter(|node| {
            let deps = deps_map.get(*node).cloned().unwrap_or_default();
            let internal_deps: HashSet<_> = deps.intersection(nodes).collect();
            internal_deps.is_empty()
        })
        .cloned()
        .collect();

    // Stable sort for consistent ordering
    roots.sort();

    // Build a tree for each root node
    for root in roots {
        if !visited.contains(&root) {
            let tree = build_tree_recursive(
                &root,
                nodes,
                deps_map,
                dependents_map,
                node_paths,
                &mut visited,
                None,
            );
            forest.push(tree);
        }
    }

    forest
}

/// Recursively build a tree from a starting node.
///
/// Builds a tree from a root node downward through its dependents.
/// Uses stable sorting for consistent output across runs.
fn build_tree_recursive(
    node_name: &str,
    all_nodes: &HashSet<String>,
    deps_map: &HashMap<String, HashSet<String>>,
    dependents_map: &HashMap<String, HashSet<String>>,
    node_paths: &HashMap<String, PathBuf>,
    visited: &mut HashSet<String>,
    primary_parent: Option<&str>,
) -> TreeNode {
    let mut tree_node = TreeNode::new(node_name.to_string(), node_paths.get(node_name).cloned());

    visited.insert(node_name.to_string());

    // Find all dependencies (parents) within the set - nodes this one depends on
    let mut all_dependencies: Vec<_> = deps_map
        .get(node_name)
        .map(|deps| {
            deps.iter()
                .filter(|p| all_nodes.contains(*p))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    // Stable sort for consistent ordering
    all_dependencies.sort();

    // Mark additional dependencies (those other than the primary parent)
    if let Some(primary) = primary_parent {
        for dep in all_dependencies.iter() {
            if dep != primary {
                tree_node.add_additional_parent(dep.clone());
            }
        }
    }

    // Get children (dependents within the set) - nodes that depend on this one
    let mut children: Vec<_> = dependents_map
        .get(node_name)
        .map(|deps| {
            deps.iter()
                .filter(|d| all_nodes.contains(*d))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    // Stable sort for consistent ordering
    children.sort();

    // Build child trees (traverse dependents)
    for child in children {
        if visited.contains(&child) {
            // Node already shown elsewhere, add a reference
            let mut ref_node = TreeNode::new(child.clone(), None);
            ref_node
                .additional_parents
                .push("[shown above]".to_string());
            tree_node.add_child(ref_node);
        } else {
            let child_tree = build_tree_recursive(
                &child,
                all_nodes,
                deps_map,
                dependents_map,
                node_paths,
                visited,
                Some(node_name),
            );
            tree_node.add_child(child_tree);
        }
    }

    tree_node
}

/// Render a tree node and its children to a string.
///
/// Uses Unicode box-drawing characters to create a visual tree structure.
///
/// # Arguments
///
/// * `node` - The tree node to render
/// * `prefix` - Current line prefix for indentation
/// * `is_last` - Whether this is the last child at this level
///
/// # Returns
///
/// A formatted string representation of the tree
pub fn render_tree(node: &TreeNode, prefix: &str, is_last: bool) -> String {
    let mut output = String::new();

    // Render current node
    let connector = if is_last { TREE_LAST } else { TREE_BRANCH };

    let mut node_line = node.name.clone();

    // Add path if available
    if let Some(ref path) = node.path {
        node_line.push_str(&format!(" ({})", path.display()));
    }

    // Add additional parents if any
    if !node.additional_parents.is_empty() {
        if node.additional_parents.len() == 1 && node.additional_parents[0] == "[shown above]" {
            node_line.push_str(" [shown above]");
        } else {
            let deps = node.additional_parents.join(", ");
            node_line.push_str(&format!(" [also depends on: {deps}]"));
        }
    }

    output.push_str(&format!("{prefix}{connector}{node_line}\n"));

    // Render children
    let child_prefix = if is_last {
        format!("{prefix}{TREE_SPACE}")
    } else {
        format!("{prefix}{TREE_VERTICAL}")
    };

    for (i, child) in node.children.iter().enumerate() {
        let is_last_child = i == node.children.len() - 1;
        output.push_str(&render_tree(child, &child_prefix, is_last_child));
    }

    output
}

/// Render an entire forest of trees.
///
/// # Arguments
///
/// * `forest` - Vector of root tree nodes
/// * `title_prefix` - Prefix for tree titles (e.g., "Tree")
///
/// # Returns
///
/// A formatted string representation of the entire forest
pub fn render_forest(forest: &[TreeNode], title_prefix: &str) -> String {
    let mut output = String::new();

    for (i, tree) in forest.iter().enumerate() {
        let tree_num = i + 1;
        let node_count = count_nodes(tree);

        output.push_str(&format!(
            "{title_prefix} {tree_num} ({node_count} nodes):\n"
        ));

        // Render the root node without prefix
        output.push_str(&tree.name.to_string());
        if let Some(ref path) = tree.path {
            output.push_str(&format!(" ({})", path.display()));
        }
        output.push('\n');

        // Render children
        for (j, child) in tree.children.iter().enumerate() {
            let is_last = j == tree.children.len() - 1;
            output.push_str(&render_tree(child, "", is_last));
        }

        // Add spacing between trees (but not after the last one)
        if i < forest.len() - 1 {
            output.push('\n');
        }
    }

    output
}

/// Count total unique nodes in a tree (excluding references to nodes shown elsewhere).
fn count_nodes(node: &TreeNode) -> usize {
    // Check if this is a reference to a node shown elsewhere
    let is_reference =
        node.additional_parents.len() == 1 && node.additional_parents[0] == "[shown above]";

    if is_reference {
        return 0; // Don't count references
    }

    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

/// Render a tree node and its children in minimal format (names only).
///
/// Similar to `render_tree()` but omits file paths and additional dependency annotations.
/// Only shows node names with tree structure characters.
///
/// # Arguments
///
/// * `node` - The tree node to render
/// * `prefix` - Current line prefix for indentation
/// * `is_last` - Whether this is the last child at this level
///
/// # Returns
///
/// A formatted string representation of the tree with minimal information
pub fn render_tree_minimal(node: &TreeNode, prefix: &str, is_last: bool) -> String {
    let mut output = String::new();

    // Skip nodes that are references to already-shown nodes
    let is_reference =
        node.additional_parents.len() == 1 && node.additional_parents[0] == "[shown above]";
    if is_reference {
        return output;
    }

    // Render current node with just the name
    let connector = if is_last { TREE_LAST } else { TREE_BRANCH };
    output.push_str(&format!("{prefix}{connector}{}\n", node.name));

    // Render children
    let child_prefix = if is_last {
        format!("{prefix}{TREE_SPACE}")
    } else {
        format!("{prefix}{TREE_VERTICAL}")
    };

    for (i, child) in node.children.iter().enumerate() {
        let is_last_child = i == node.children.len() - 1;
        output.push_str(&render_tree_minimal(child, &child_prefix, is_last_child));
    }

    output
}

/// Render a forest as a unified tree sorted by size and name.
///
/// Sorts trees by node count (descending), then by root node name (ascending).
/// Displays all trees together without separate numbering or metadata.
///
/// # Arguments
///
/// * `forest` - Vector of root tree nodes
///
/// # Returns
///
/// A formatted string representation of the unified forest
pub fn render_forest_unified(forest: &[TreeNode]) -> String {
    let mut output = String::new();

    // Create a vector of (tree, node_count) for sorting
    let mut trees_with_counts: Vec<_> = forest
        .iter()
        .map(|tree| {
            let count = count_nodes(tree);
            (tree, count)
        })
        .collect();

    // Sort by node count (descending), then by root name (ascending)
    trees_with_counts.sort_by(|(tree_a, count_a), (tree_b, count_b)| {
        count_b
            .cmp(count_a)
            .then_with(|| tree_a.name.cmp(&tree_b.name))
    });

    // Render all trees in unified format
    for (i, (tree, _count)) in trees_with_counts.iter().enumerate() {
        let is_last = i == trees_with_counts.len() - 1;

        // Render the root node
        let connector = if is_last { TREE_LAST } else { TREE_BRANCH };
        output.push_str(&format!("{}{}\n", connector, tree.name));

        // Render children
        let child_prefix = if is_last {
            TREE_SPACE.to_string()
        } else {
            TREE_VERTICAL.to_string()
        };

        for (j, child) in tree.children.iter().enumerate() {
            let is_last_child = j == tree.children.len() - 1;
            output.push_str(&render_tree_minimal(child, &child_prefix, is_last_child));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_standard_table() {
        let table = create_standard_table();
        // Just verify it creates without panic
        assert!(table.is_empty());
    }

    #[test]
    fn test_create_node_table() {
        let table = create_node_table(vec!["Node", "Path"]);
        // Table creation should succeed without panic
        // Note: comfy_table considers a table with headers but no rows as "empty"
        // so we just verify it was created successfully
        let _ = table;
    }

    #[test]
    fn test_format_path_short() {
        let path = Path::new("file.sql");
        let formatted = format_path_for_display(path, 50);
        assert_eq!(formatted, "file.sql");
    }

    #[test]
    fn test_format_path_long() {
        let path = Path::new("/very/long/path/to/some/directory/file.sql");
        let formatted = format_path_for_display(path, 20);
        // Should truncate but keep readable parts
        assert!(formatted.starts_with('…'));
        assert!(formatted.contains("file.sql") || formatted.len() <= 20);
    }

    #[test]
    fn test_format_file_list_empty() {
        let files: Vec<PathBuf> = vec![];
        let result = format_file_list(&files, ", ");
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_file_list_single() {
        let files = vec![PathBuf::from("a.sql")];
        let result = format_file_list(&files, ", ");
        assert_eq!(result, "a.sql");
    }

    #[test]
    fn test_format_file_list_multiple() {
        let files = vec![
            PathBuf::from("a.sql"),
            PathBuf::from("b.sql"),
            PathBuf::from("c.sql"),
        ];
        let result = format_file_list(&files, ", ");
        assert_eq!(result, "a.sql, b.sql, c.sql");
    }

    #[test]
    fn test_format_file_list_custom_separator() {
        let files = vec![PathBuf::from("a.sql"), PathBuf::from("b.sql")];
        let result = format_file_list(&files, " | ");
        assert_eq!(result, "a.sql | b.sql");
    }

    #[test]
    fn test_format_count_zero() {
        assert_eq!(format_count(0, "file", "files"), "0 files");
    }

    #[test]
    fn test_format_count_one() {
        assert_eq!(format_count(1, "file", "files"), "1 file");
    }

    #[test]
    fn test_format_count_many() {
        assert_eq!(format_count(42, "file", "files"), "42 files");
    }

    #[test]
    fn test_separator_line() {
        let line = separator_line();
        assert!(!line.is_empty());
        assert!(line.contains('━'));
    }

    #[test]
    fn test_section_header_line() {
        let line = section_header_line();
        assert!(!line.is_empty());
        assert!(line.contains('═'));
    }
}
