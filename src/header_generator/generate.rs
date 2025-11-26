//! Generate headers to a separate directory.

use log::info;
use std::fs;
use std::path::Path;

use super::{check_rename_conflicts, find_content_start, generate_header};
use crate::exceptions::TopCatError;
use crate::file_node::FileNode;

/// Generate files with updated headers to a new directory
pub fn generate_headers_to_dir(
    file_nodes: &[FileNode],
    comment_str: &str,
    output_dir: &Path,
    rename_files: bool,
    default_extension: &str,
) -> Result<(), TopCatError> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    // Check for filename conflicts before making any changes
    if rename_files {
        check_rename_conflicts(file_nodes, default_extension)?;
    }

    for file_node in file_nodes {
        // Only generate if we have discovered dependencies
        if file_node.discovered_deps.is_none() {
            continue;
        }

        log::debug!("Generating updated file for {:?}", file_node.path);

        // Read the original file
        let original_content = fs::read_to_string(&file_node.path)?;

        // Generate new header
        let new_header = generate_header(file_node, comment_str);

        // Find where the original content starts (after headers)
        let content_start = find_content_start(&original_content, comment_str);
        let original_body = &original_content[content_start..];

        // Determine output filename (use suggested name if renaming, otherwise original)
        let file_name = if rename_files {
            file_node
                .suggested_filename(default_extension)
                .unwrap_or_else(|| {
                    file_node
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown.sql")
                        .to_string()
                })
        } else {
            file_node
                .path
                .file_name()
                .ok_or_else(|| {
                    TopCatError::UnknownError(format!("Invalid file path: {:?}", file_node.path))
                })?
                .to_string_lossy()
                .to_string()
        };

        let output_path = output_dir.join(&file_name);

        let updated_content = format!("{new_header}\n{original_body}");
        fs::write(&output_path, updated_content)?;

        if rename_files && file_node.needs_rename(default_extension) {
            info!(
                "Generated file {:?} with new name: {}",
                file_node.path.file_name(),
                file_name
            );
        } else {
            info!("Generated updated file at {output_path:?}");
        }
    }

    Ok(())
}
