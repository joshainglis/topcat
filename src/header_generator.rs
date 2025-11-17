use log::{debug, info};
use std::fs;
use std::path::Path;

use crate::exceptions::TopCatError;
use crate::file_node::FileNode;
use crate::sql_config::HeaderUpdateMode;

/// Generate updated file header with discovered dependencies
pub fn generate_header(file_node: &FileNode, comment_str: &str) -> String {
    let mut header = String::new();

    // Add node name
    header.push_str(&format!("{comment_str} name: {}\n", file_node.name));

    // Add schema requirement for object nodes (not schema nodes)
    if file_node.name.contains('.') {
        let schema_name = file_node
            .name
            .split('.')
            .next()
            .expect("split() always returns at least one element");
        header.push_str(&format!("{comment_str} requires: {schema_name}\n"));
    }

    // Add dependencies (sorted for consistency)
    let mut deps: Vec<_> = file_node.deps.iter().collect();
    deps.sort();
    for dep in deps {
        header.push_str(&format!("{comment_str} requires: {dep}\n"));
    }

    // Add override dependencies (with ! prefix)
    let mut override_deps: Vec<_> = file_node.override_deps.iter().collect();
    override_deps.sort();
    for dep in override_deps {
        header.push_str(&format!("{comment_str} requires: !{dep}\n"));
    }

    // Add layer if not the default
    if !file_node.layer.is_empty() {
        header.push_str(&format!("{comment_str} layer: {}\n", file_node.layer));
    }

    // Add exists dependencies
    let mut exists: Vec<_> = file_node.ensure_exists.iter().collect();
    exists.sort();
    for exist in exists {
        header.push_str(&format!("{comment_str} exists: {exist}\n"));
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

/// Update file headers in-place
fn update_headers_in_place(
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

        debug!("Updating headers for {:?}", file_node.path);

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

/// Generate files with updated headers to a new directory
fn generate_headers_to_dir(
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

        debug!("Generating updated file for {:?}", file_node.path);

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

/// Check for filename conflicts before renaming
fn check_rename_conflicts(
    file_nodes: &[FileNode],
    default_extension: &str,
) -> Result<(), TopCatError> {
    use std::collections::HashMap;
    use std::path::PathBuf;

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

/// Find where the actual content starts (after header comments)
fn find_content_start(content: &str, comment_str: &str) -> usize {
    let mut offset = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip header comment lines and empty lines
        if trimmed.starts_with(comment_str) || trimmed.is_empty() {
            offset += line.len() + 1; // +1 for newline
        } else {
            // Found first non-header line
            break;
        }
    }

    offset
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_generate_header() {
        let file_node = FileNode::new(
            "test_schema.test_table".to_string(),
            PathBuf::from("/tmp/test.sql"),
            HashSet::from([
                "test_schema.dep1".to_string(),
                "test_schema.dep2".to_string(),
            ]),
            "normal".to_string(),
            HashSet::new(),
        );

        let header = generate_header(&file_node, "--");

        assert!(header.contains("-- name: test_schema.test_table"));
        assert!(header.contains("-- requires: test_schema"));
        assert!(
            header.contains("-- requires: test_schema.dep1")
                || header.contains("-- requires: test_schema.dep2")
        );
    }

    #[test]
    fn test_generate_header_with_overrides() {
        let mut file_node = FileNode::new(
            "test_schema.test_table".to_string(),
            PathBuf::from("/tmp/test.sql"),
            HashSet::from(["test_schema.dep1".to_string()]),
            "normal".to_string(),
            HashSet::new(),
        );
        file_node.override_deps = HashSet::from(["test_schema.override_dep".to_string()]);

        let header = generate_header(&file_node, "--");

        assert!(header.contains("-- name: test_schema.test_table"));
        assert!(header.contains("-- requires: test_schema.dep1"));
        assert!(header.contains("-- requires: !test_schema.override_dep"));
    }

    #[test]
    fn test_find_content_start() {
        let content = "-- name: test\n-- requires: dep1\n\nSELECT 1;";
        let start = find_content_start(content, "--");

        assert!(content[start..].starts_with("SELECT"));
    }

    #[test]
    fn test_update_headers_in_place() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.sql");

        // Create a test file
        fs::write(
            &test_file,
            "-- name: old_name\n-- requires: old_dep\n\nSELECT 1;\n",
        )?;

        let mut file_node = FileNode::new(
            "new_name".to_string(),
            test_file.clone(),
            HashSet::from(["new_dep".to_string()]),
            "normal".to_string(),
            HashSet::new(),
        );
        file_node.discovered_deps = Some(HashSet::from(["new_dep".to_string()]));

        update_headers_in_place(&[file_node], "--", false, "sql")?;

        let updated_content = fs::read_to_string(&test_file)?;
        assert!(updated_content.contains("-- name: new_name"));
        assert!(updated_content.contains("-- requires: new_dep"));
        assert!(updated_content.contains("SELECT 1;"));

        Ok(())
    }

    #[test]
    fn test_file_renaming() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("old_name.sql");

        // Create a test file
        fs::write(
            &test_file,
            "-- name: old_name\n-- requires: old_dep\n\nCREATE TABLE test_schema.new_table (id INT);\n",
        )?;

        let mut file_node = FileNode::new(
            "test_schema.new_table".to_string(),
            test_file.clone(),
            HashSet::from(["test_schema".to_string()]),
            "normal".to_string(),
            HashSet::new(),
        );
        file_node.discovered_deps = Some(HashSet::from(["test_schema".to_string()]));

        // Update with renaming enabled
        update_headers_in_place(&[file_node], "--", true, "sql")?;

        // Old file should not exist
        assert!(!test_file.exists());

        // New file should exist with correct name
        let new_file = temp_dir.path().join("new_table.sql");
        assert!(new_file.exists());

        // Content should be updated
        let updated_content = fs::read_to_string(&new_file)?;
        assert!(updated_content.contains("-- name: test_schema.new_table"));
        assert!(updated_content.contains("-- requires: test_schema"));
        assert!(updated_content.contains("CREATE TABLE test_schema.new_table"));

        Ok(())
    }

    #[test]
    fn test_rename_conflict_detection() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let test_file1 = temp_dir.path().join("file1.sql");
        let test_file2 = temp_dir.path().join("file2.sql");

        // Create two test files that would want the same name
        fs::write(&test_file1, "CREATE TABLE test_schema.my_table (id INT);\n")?;
        fs::write(
            &test_file2,
            "CREATE TABLE test_schema.my_table (name TEXT);\n",
        )?;

        let mut file_node1 = FileNode::new(
            "test_schema.my_table".to_string(),
            test_file1.clone(),
            HashSet::new(),
            "normal".to_string(),
            HashSet::new(),
        );
        file_node1.discovered_deps = Some(HashSet::new());

        let mut file_node2 = FileNode::new(
            "test_schema.my_table".to_string(),
            test_file2.clone(),
            HashSet::new(),
            "normal".to_string(),
            HashSet::new(),
        );
        file_node2.discovered_deps = Some(HashSet::new());

        // This should fail due to conflict
        let result = update_headers_in_place(&[file_node1, file_node2], "--", true, "sql");
        assert!(result.is_err());

        Ok(())
    }
}
