//! In-place header updating functionality.

use log::info;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::{find_content_start, generate_header};
use crate::exceptions::TopCatError;
use crate::file_node::FileNode;

/// Update file headers in-place
pub fn update_headers_in_place(
    file_nodes: &[FileNode],
    comment_str: &str,
    rename_files: bool,
    default_extension: &str,
) -> Result<(), TopCatError> {
    // Check for filename conflicts before making any changes
    if rename_files {
        check_rename_conflicts(file_nodes, default_extension)?;
    }

    for file_node in file_nodes {
        // Only update if we have discovered dependencies
        if file_node.discovered_deps.is_none() {
            continue;
        }

        log::debug!("Updating headers for {:?}", file_node.path);

        // Read the original file
        let original_content = fs::read_to_string(&file_node.path)?;

        // Generate new header
        let new_header = generate_header(file_node, comment_str);

        // Find where the original content starts (after headers)
        let content_start = find_content_start(&original_content, comment_str);
        let original_body = &original_content[content_start..];

        // Write updated file
        let updated_content = format!("{new_header}\n{original_body}");

        // Handle file renaming if enabled and needed
        if rename_files && file_node.needs_rename(default_extension) {
            if let Some(suggested_filename) = file_node.suggested_filename(default_extension) {
                let parent_dir = file_node
                    .path
                    .parent()
                    .ok_or_else(|| TopCatError::UnknownError("No parent directory".to_string()))?;
                let new_path = parent_dir.join(&suggested_filename);

                // Write to new location
                fs::write(&new_path, &updated_content)?;

                // Remove old file
                fs::remove_file(&file_node.path)?;

                info!(
                    "Renamed {:?} to {}",
                    file_node.path.file_name(),
                    suggested_filename
                );
            }
        } else {
            // Just update the existing file
            fs::write(&file_node.path, updated_content)?;
            info!("Updated headers in {:?}", file_node.path);
        }
    }

    Ok(())
}

/// Check for filename conflicts before renaming
pub fn check_rename_conflicts(
    file_nodes: &[FileNode],
    default_extension: &str,
) -> Result<(), TopCatError> {
    // Build a map of (parent_dir, filename) to original paths
    // This ensures we only detect conflicts within the same directory
    let mut filename_map: HashMap<(PathBuf, String), Vec<&FileNode>> = HashMap::new();

    for file_node in file_nodes {
        if file_node.discovered_deps.is_none() {
            continue;
        }

        if let (Some(parent_dir), Some(suggested)) = (
            file_node.path.parent(),
            file_node.suggested_filename(default_extension),
        ) {
            let key = (parent_dir.to_path_buf(), suggested);
            filename_map.entry(key).or_default().push(file_node);
        }
    }

    // Check for conflicts (multiple files wanting the same name in the same directory)
    let mut conflicts = Vec::new();
    for ((parent_dir, filename), nodes) in filename_map.iter() {
        if nodes.len() > 1 {
            let mut conflict_lines = vec![
                format!("  Directory: {}", parent_dir.display()),
                format!("  Target filename: {}", filename),
                "  Conflicting files:".to_string(),
            ];
            for node in nodes {
                conflict_lines.push(format!("    - {}", node.path.display()));
            }
            conflicts.push(conflict_lines.join("\n"));
        }
    }

    // Also check if any suggested filename already exists as a different file
    for file_node in file_nodes {
        if file_node.discovered_deps.is_none() {
            continue;
        }

        if let (Some(parent_dir), Some(suggested)) = (
            file_node.path.parent(),
            file_node.suggested_filename(default_extension),
        ) {
            let target_path = parent_dir.join(&suggested);

            // Check if target exists and is a different file
            if target_path.exists() && target_path != file_node.path {
                // Check if this target is one of our managed files
                let is_managed = file_nodes.iter().any(|n| n.path == target_path);

                if !is_managed {
                    conflicts.push(format!(
                        "  Cannot rename: {}\n  Target name: {}\n  Blocked by existing file: {}",
                        file_node.path.display(),
                        suggested,
                        target_path.display()
                    ));
                }
            }
        }
    }

    if !conflicts.is_empty() {
        let error_msg = format!(
            "\nFile rename conflicts detected:\n\n{}",
            conflicts.join("\n\n")
        );
        return Err(TopCatError::ConfigError(error_msg));
    }

    Ok(())
}
