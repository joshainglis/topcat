//! Header generation and updating for file nodes.
//!
//! This module handles:
//! - Generating headers with discovered dependencies
//! - Updating file headers in-place
//! - Generating files with updated headers to a new directory
//! - File renaming based on node names

mod generate;
mod in_place;

#[cfg(test)]
mod tests;

use std::path::Path;

use crate::exceptions::TopCatError;
use crate::file_node::FileNode;
use crate::sql_config::HeaderUpdateMode;

pub use generate::generate_headers_to_dir;
pub use in_place::{check_rename_conflicts, update_headers_in_place};

/// Generate updated file header with discovered dependencies
pub fn generate_header(file_node: &FileNode, comment_str: &str) -> String {
    let mut header = String::new();

    // Preserve original manual headers if manual flag is set
    if file_node.manual {
        if let Some(ref original_headers) = file_node.original_headers {
            header.push_str(original_headers);
            header.push_str("\n\n");
        }
    }

    // Determine comment prefix based on manual flag
    let cmt = if file_node.manual {
        "---tc:"
    } else {
        comment_str
    };

    // Add node name
    header.push_str(&format!("{cmt} name: {}\n", file_node.name));

    // Add layer immediately after name (if not the default)
    if (!file_node.layer.is_empty()) && (!file_node.layer_is_fallback) {
        header.push_str(&format!("{cmt} layer: {}\n", file_node.layer));
    }

    // Extract schema name for prioritized output (schema dependency always comes first)
    let schema_name = if file_node.name.contains('.') {
        file_node.name.split('.').next()
    } else {
        None
    };

    // Add schema dependency first (if it exists in deps)
    // This matches Python behavior: schema always comes before other deps
    // Schema dependency is always "requires" (never converted to exists)
    if let Some(schema) = schema_name {
        if file_node.deps.contains(schema) {
            header.push_str(&format!("{cmt} requires: {schema}\n"));
        }
    }

    // Add remaining dependencies (sorted, excluding schema which we already added)
    // Determine type individually based on soft_deps flag and pattern matching
    let mut deps: Vec<_> = file_node.deps.iter().collect();
    deps.sort();
    for dep in deps {
        // Skip schema dependency as we already wrote it
        if let Some(schema) = schema_name {
            if dep == schema {
                continue;
            }
        }

        // Determine dependency type:
        // 1. If soft_deps is true, ALL dependencies use "exists"
        // 2. Otherwise, check if this specific dependency should be converted
        let dep_type = if file_node.soft_deps || file_node.should_convert_dep_to_exists(dep) {
            "exists"
        } else {
            "requires"
        };

        header.push_str(&format!("{cmt} {dep_type}: {dep}\n"));
    }

    // Add override dependencies (with ! prefix)
    // Override deps always use "requires" even with soft_deps
    let mut override_deps: Vec<_> = file_node.override_deps.iter().collect();
    override_deps.sort();
    for dep in override_deps {
        header.push_str(&format!("{cmt} requires: !{dep}\n"));
    }

    // Add soft-deps marker if set
    if file_node.soft_deps {
        header.push_str(&format!("{cmt} soft-deps\n"));
    }

    // Add final/initial marker if present
    if let Some(ref final_initial) = file_node.final_initial {
        header.push_str(&format!("{cmt} {final_initial}\n"));
    }

    // Add implicit marker if set
    if file_node.implicit {
        header.push_str(&format!("{cmt} implicit\n"));
    }

    // Add exists dependencies (only if not using soft_deps)
    if !file_node.soft_deps {
        let mut exists: Vec<_> = file_node.ensure_exists.iter().collect();
        exists.sort();
        for exist in exists {
            header.push_str(&format!("{cmt} exists: {exist}\n"));
        }
    }

    // Preserve original node_name header if present
    if let Some(ref node_name_header) = file_node.original_node_name_header {
        header.push_str(&format!("{}\n", node_name_header.trim()));
    }

    header
}

/// Update file headers with discovered dependencies
pub fn update_headers(
    file_nodes: &[FileNode],
    comment_str: &str,
    mode: HeaderUpdateMode,
    output_dir: Option<&Path>,
    rename_files: bool,
    default_extension: &str,
) -> Result<(), TopCatError> {
    match mode {
        HeaderUpdateMode::Never => Ok(()),
        HeaderUpdateMode::InPlace => {
            update_headers_in_place(file_nodes, comment_str, rename_files, default_extension)?;
            Ok(())
        }
        HeaderUpdateMode::Generate => {
            let output_dir = output_dir.ok_or_else(|| {
                TopCatError::ConfigError(
                    "Output directory must be specified for generate mode".to_string(),
                )
            })?;
            generate_headers_to_dir(
                file_nodes,
                comment_str,
                output_dir,
                rename_files,
                default_extension,
            )?;
            Ok(())
        }
    }
}

/// Find where the actual content starts (after header comments)
pub(crate) fn find_content_start(content: &str, comment_str: &str) -> usize {
    let mut offset = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip header comment lines (including ---tc: prefix), and empty lines
        if trimmed.starts_with(comment_str) || trimmed.starts_with("---tc:") || trimmed.is_empty() {
            offset += line.len() + 1; // +1 for newline
        } else {
            // Found first non-header line
            break;
        }
    }

    offset
}
