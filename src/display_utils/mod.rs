//! Display and formatting utilities for CLI output.
//!
//! This module provides common utilities for formatted console output across
//! all Topcat commands. It includes table formatting helpers, path display
//! utilities, and tree rendering for dependency visualization.
//!
//! ## Module Structure
//!
//! - Basic utilities: table creation, path formatting, count formatting
//! - [`tree`] - Tree data structures and building
//! - [`render`] - Tree rendering functions

mod render;
mod tree;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use comfy_table::{Attribute, Cell, ContentArrangement, Table};

// Re-export tree types and functions
pub use render::{render_forest, render_forest_unified, render_tree, render_tree_minimal};
pub use tree::{build_tree_forest, TreeNode};

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
