//! Filename utilities for FileNode.

use super::FileNode;

impl FileNode {
    /// Calculate suggested filename from node name
    /// Follows the same logic as the Python script:
    /// - For schema.object nodes, use "object.ext"
    /// - For schema-only nodes, use "schema.ext"
    pub fn suggested_filename(&self, default_extension: &str) -> Option<String> {
        // Extract the extension from the current path if present
        let extension = self
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or(default_extension);

        // Parse the node name
        if let Some(dot_idx) = self.name.find('.') {
            // Schema.object pattern - use object name
            let object_name = &self.name[dot_idx + 1..];
            Some(format!("{}.{}", object_name.to_lowercase(), extension))
        } else {
            // Schema-only pattern - use schema name
            Some(format!("{}.{}", self.name.to_lowercase(), extension))
        }
    }

    /// Check if the file should be renamed based on suggested filename
    pub fn needs_rename(&self, default_extension: &str) -> bool {
        if let (Some(current_filename), Some(suggested)) = (
            self.path.file_name().and_then(|n| n.to_str()),
            self.suggested_filename(default_extension),
        ) {
            current_filename != suggested
        } else {
            false
        }
    }
}
