//! Tree rendering functions for dependency visualization.

use super::tree::TreeNode;

// Tree display constants
const TREE_BRANCH: &str = "├── ";
const TREE_LAST: &str = "└── ";
const TREE_VERTICAL: &str = "│   ";
const TREE_SPACE: &str = "    ";

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
pub(crate) fn count_nodes(node: &TreeNode) -> usize {
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
