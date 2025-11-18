use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::Display;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io;
use std::io::BufRead;
use std::path::PathBuf;

use crate::exceptions::FileNodeError;

fn get_file_headers(path: &PathBuf, comment_str: &str) -> io::Result<Vec<String>> {
    let file = File::open(path)?;

    // fill a vector with the first lines of the file starting with the comment string ignoring empty lines. Stop on the first line without the comment string.
    let reader = io::BufReader::new(file);
    let file_data: Vec<_> = reader
        .lines()
        .take_while(|x| {
            let x = match x.as_ref() {
                Ok(x) => x,
                Err(_) => return false,
            };
            x.starts_with(comment_str) || x.is_empty()
        })
        .collect::<io::Result<_>>()?;

    // remove any empty lines from the vector and return
    Ok(file_data
        .iter()
        .filter(|x| !x.is_empty())
        .map(|x| x.to_string())
        .collect())
}

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

    fn split_dependencies(line: &str) -> Vec<String> {
        line.split(|c: char| c.is_whitespace() || c == ',')
            .filter_map(|x| {
                let x = x.trim().to_string();
                if !x.is_empty() { Some(x) } else { None }
            })
            .collect()
    }

    /// Split dependencies and identify which ones have the override prefix (!)
    /// Returns (normal_deps, override_deps)
    fn split_dependencies_with_overrides(line: &str) -> (Vec<String>, Vec<String>) {
        let mut normal_deps = Vec::new();
        let mut override_deps = Vec::new();

        for item in Self::split_dependencies(line) {
            if let Some(stripped) = item.strip_prefix('!') {
                override_deps.push(stripped.to_string());
            } else {
                normal_deps.push(item);
            }
        }

        (normal_deps, override_deps)
    }
    pub fn from_file(
        comment_str: &str,
        path: &PathBuf,
        layers: &[String],
        fallback_layer: &str,
    ) -> Result<FileNode, FileNodeError> {
        let file_data = get_file_headers(path, comment_str)
            .map_err(|err| FileNodeError::FileOpen(path.clone(), err))?;

        // Store original headers for later preservation if needed
        let original_headers_text = if !file_data.is_empty() {
            Some(file_data.join("\n"))
        } else {
            None
        };
        let name_str = format!("{comment_str} name:");
        let dep_str = format!("{comment_str} requires:");
        let drop_str = format!("{comment_str} dropped_by:");
        let layer_str = format!("{comment_str} layer:");
        // Keep backward compatibility with old headers
        let prepend_str = format!("{comment_str} is_initial");
        let append_str = format!("{comment_str} is_final");
        let ensure_exists_str = format!("{comment_str} exists:");
        let implicit_str = format!("{comment_str} implicit");
        let manual_str = format!("{comment_str} manual");
        let soft_deps_str = format!("{comment_str} soft-deps");
        let final_str = format!("{comment_str} final");
        let initial_str = format!("{comment_str} initial");
        let node_name_str = format!("{comment_str} node_name:");

        let mut name = String::new();
        let mut deps = HashSet::new();
        let mut layer = fallback_layer.to_string();
        let mut layer_is_fallback = true;
        let mut ensure_exists = HashSet::new();
        let mut override_deps = HashSet::new();
        let mut implicit = false;
        let mut manual = false;
        let mut soft_deps = false;
        let mut final_initial: Option<String> = None;
        let mut original_node_name_header: Option<String> = None;

        for unprocessed_line in &file_data {
            let line = unprocessed_line.trim().to_lowercase();
            // Check for standard "name:" header
            if line.starts_with(&name_str) {
                if name.is_empty() {
                    name = line[name_str.len()..].trim().to_string();
                } else {
                    // raise an error that a file has more than one name declared
                    return Err(FileNodeError::TooManyNames(
                        path.clone(),
                        vec![name, line[name_str.len()..].trim().to_string()],
                    ));
                }
            }
            // Check for alternative "node_name:" header (for compatibility with Python script)
            else if line.starts_with(&node_name_str) && name.is_empty() {
                // Extract node name from node_name header (format: "-- node_name: schema.object")
                let node_name_value = line[node_name_str.len()..].trim().to_string();
                if !node_name_value.is_empty() {
                    name = node_name_value;
                    // Store the original header line for preservation
                    original_node_name_header = Some(unprocessed_line.trim().to_string());
                }
            } else if line.starts_with(&dep_str) || line.starts_with(&drop_str) {
                // Both "requires:" and "dropped_by:" are dependencies with override support
                // -- requires: tomato, !potato, orange -> normal: ["tomato", "orange"], override: ["potato"]
                // -- dropped_by: tomato, !potato -> normal: ["tomato"], override: ["potato"]
                let prefix_len = if line.starts_with(&dep_str) {
                    dep_str.len()
                } else {
                    drop_str.len()
                };
                let (normal, overrides) =
                    Self::split_dependencies_with_overrides(&line[prefix_len..]);
                deps.extend(normal);
                override_deps.extend(overrides);
            } else if line.starts_with(&layer_str) {
                // -- layer: prepend -> "prepend"
                let declared_layer = line[layer_str.len()..].trim();
                if !declared_layer.is_empty() {
                    layer = declared_layer.to_string();
                    layer_is_fallback = layer.eq(fallback_layer);
                }
            } else if line.starts_with(&prepend_str) {
                // -- is_initial -> "prepend" (backward compatibility)
                layer = "prepend".to_string();
                layer_is_fallback = false;
            } else if line.starts_with(&append_str) {
                // -- is_final -> "append" (backward compatibility)
                layer = "append".to_string();
                layer_is_fallback = false;
            } else if line.starts_with(&ensure_exists_str) {
                // --exists: tomato, potato -> ["tomato", "potato"]
                for item in Self::split_dependencies(&line[ensure_exists_str.len()..]) {
                    ensure_exists.insert(item);
                }
            } else if line.starts_with(&implicit_str) {
                // -- implicit -> mark node as implicitly referenced
                implicit = true;
            } else if line.starts_with(&manual_str) {
                // -- manual -> preserve original headers
                manual = true;
            } else if line.starts_with(&soft_deps_str) {
                // -- soft-deps -> convert all deps to exists
                soft_deps = true;
            } else if line.starts_with(&final_str) {
                // -- final -> preserve marker
                final_initial = Some("final".to_string());
            } else if line.starts_with(&initial_str) {
                // -- initial -> preserve marker
                final_initial = Some("initial".to_string());
            }
            // Note: node_name_str is handled above with name extraction
        }
        if name.is_empty() {
            return Err(FileNodeError::NoNameDefined(path.clone()));
        }

        // Validate that the declared layer exists in the configured layers
        if !layers.contains(&layer) {
            return Err(FileNodeError::InvalidLayer(path.clone(), layer));
        }

        let mut file_node = FileNode::new(
            name,
            path.clone(),
            deps,
            layer,
            layer_is_fallback,
            ensure_exists,
        );
        file_node.override_deps = override_deps;
        file_node.implicit = implicit;
        file_node.manual = manual;
        file_node.soft_deps = soft_deps;
        file_node.final_initial = final_initial;
        file_node.original_node_name_header = original_node_name_header;
        // Only preserve original headers if manual flag is set
        file_node.original_headers = if manual { original_headers_text } else { None };
        Ok(file_node)
    }

    /// Merge discovered dependencies with manual dependencies based on the strategy
    pub fn merge_dependencies(&mut self, merge_strategy: crate::sql_config::MergeStrategy) {
        use crate::sql_config::MergeStrategy;

        let Some(ref discovered) = self.discovered_deps else {
            // No discovered dependencies, nothing to merge
            return;
        };

        match merge_strategy {
            MergeStrategy::HeaderOnly => {
                // Keep only manual dependencies
            }
            MergeStrategy::DiscoveryOnly => {
                // Replace deps with discovered, but keep override_deps
                self.deps = discovered.clone();
                // Add back override deps
                self.deps.extend(self.override_deps.clone());
            }
            MergeStrategy::Union => {
                // Combine both
                self.deps.extend(discovered.clone());
            }
            MergeStrategy::HeaderWithFallback => {
                // If deps is empty (no manual deps), use discovered
                if self.deps.is_empty() {
                    self.deps = discovered.clone();
                }
                // Always add override deps
                self.deps.extend(self.override_deps.clone());
            }
            MergeStrategy::Validate => {
                // Check for discrepancies
                let manual_only: HashSet<_> = self.deps.difference(discovered).collect();
                let discovered_only: HashSet<_> = discovered.difference(&self.deps).collect();

                if !manual_only.is_empty() || !discovered_only.is_empty() {
                    log::warn!(
                        "Dependency mismatch in {}: manual-only={:?}, discovered-only={:?}",
                        self.path.display(),
                        manual_only,
                        discovered_only
                    );
                }
                // For validate mode, we'll use union to include everything
                self.deps.extend(discovered.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_layer_header_format() {
        let layers = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];
        let fallback_layer = "second";

        // Create a temporary file with new layer format
        let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(&temp_file, "-- name: test_node\n-- layer: first\nSELECT 1;").unwrap();

        let file_node = FileNode::from_file(
            "--",
            &temp_file.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node.name, "test_node");
        assert_eq!(file_node.layer, "first");
    }

    #[test]
    fn test_backward_compatibility_is_initial() {
        let layers = vec![
            "prepend".to_string(),
            "normal".to_string(),
            "append".to_string(),
        ];
        let fallback_layer = "normal";

        let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(&temp_file, "-- name: test_node\n-- is_initial\nSELECT 1;").unwrap();

        let file_node = FileNode::from_file(
            "--",
            &temp_file.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node.name, "test_node");
        assert_eq!(file_node.layer, "prepend");
    }

    #[test]
    fn test_backward_compatibility_is_final() {
        let layers = vec![
            "prepend".to_string(),
            "normal".to_string(),
            "append".to_string(),
        ];
        let fallback_layer = "normal";

        let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(&temp_file, "-- name: test_node\n-- is_final\nSELECT 1;").unwrap();

        let file_node = FileNode::from_file(
            "--",
            &temp_file.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node.name, "test_node");
        assert_eq!(file_node.layer, "append");
    }

    #[test]
    fn test_fallback_layer() {
        let layers = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];
        let fallback_layer = "second";

        let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(&temp_file, "-- name: test_node\nSELECT 1;").unwrap();

        let file_node = FileNode::from_file(
            "--",
            &temp_file.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node.name, "test_node");
        assert_eq!(file_node.layer, "second");
    }

    #[test]
    fn test_invalid_layer_error() {
        let layers = vec!["first".to_string(), "second".to_string()];
        let fallback_layer = "first";

        let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(
            &temp_file,
            "-- name: test_node\n-- layer: invalid\nSELECT 1;",
        )
        .unwrap();

        let result = FileNode::from_file(
            "--",
            &temp_file.path().to_path_buf(),
            &layers,
            fallback_layer,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            FileNodeError::InvalidLayer(_, layer) => assert_eq!(layer, "invalid"),
            _ => panic!("Expected InvalidLayer error"),
        }
    }

    #[test]
    fn test_dependencies_parsing() {
        let layers = vec!["first".to_string(), "second".to_string()];
        let fallback_layer = "first";

        let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(&temp_file, "-- name: test_node\n-- layer: first\n-- requires: dep1, dep2\n-- dropped_by: dep3\nSELECT 1;").unwrap();

        let file_node = FileNode::from_file(
            "--",
            &temp_file.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node.name, "test_node");
        assert_eq!(file_node.layer, "first");
        assert!(file_node.deps.contains("dep1"));
        assert!(file_node.deps.contains("dep2"));
        assert!(file_node.deps.contains("dep3"));
        assert_eq!(file_node.deps.len(), 3);
    }

    #[test]
    fn test_schema_extraction_dot_separator() {
        assert_eq!(
            FileNode::extract_schema("my_schema.table_name"),
            Some("my_schema".to_string())
        );
        assert_eq!(
            FileNode::extract_schema("public.users"),
            Some("public".to_string())
        );
    }

    #[test]
    fn test_schema_extraction_no_schema() {
        assert_eq!(FileNode::extract_schema("table_name"), None);
        assert_eq!(FileNode::extract_schema("simple_table"), None);
    }

    #[test]
    fn test_schema_in_file_node() {
        let layers = vec!["first".to_string()];
        let fallback_layer = "first";

        let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(&temp_file, "-- name: my_schema.my_table\nSELECT 1;").unwrap();

        let file_node = FileNode::from_file(
            "--",
            &temp_file.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node.name, "my_schema.my_table");
        assert_eq!(file_node.schema, Some("my_schema".to_string()));
    }

    #[test]
    fn test_manual_header_parsing() {
        let layers = vec!["normal".to_string()];
        let fallback_layer = "normal";

        let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(
            &temp_file,
            "-- My custom header\n-- manual\n-- name: test_node\nSELECT 1;",
        )
        .unwrap();

        let file_node = FileNode::from_file(
            "--",
            &temp_file.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node.name, "test_node");
        assert!(file_node.manual);
        assert!(file_node.original_headers.is_some());
    }

    #[test]
    fn test_soft_deps_parsing() {
        let layers = vec!["normal".to_string()];
        let fallback_layer = "normal";

        let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(
            &temp_file,
            "-- name: test_node\n-- soft-deps\n-- exists: dep1\nSELECT 1;",
        )
        .unwrap();

        let file_node = FileNode::from_file(
            "--",
            &temp_file.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node.name, "test_node");
        assert!(file_node.soft_deps);
    }

    #[test]
    fn test_final_initial_parsing() {
        let layers = vec!["prepend".to_string(), "normal".to_string()];
        let fallback_layer = "normal";

        // Test "final" marker
        let temp_file1 = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(&temp_file1, "-- name: test_final\n-- final\nSELECT 1;").unwrap();

        let file_node1 = FileNode::from_file(
            "--",
            &temp_file1.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node1.final_initial, Some("final".to_string()));

        // Test "initial" marker
        let temp_file2 = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(&temp_file2, "-- name: test_initial\n-- initial\nSELECT 1;").unwrap();

        let file_node2 = FileNode::from_file(
            "--",
            &temp_file2.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node2.final_initial, Some("initial".to_string()));
    }

    #[test]
    fn test_node_name_header_parsing() {
        let layers = vec!["normal".to_string()];
        let fallback_layer = "normal";

        let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
        std::fs::write(
            &temp_file,
            "-- node_name: my_schema.my_table\nCREATE TABLE test (id INT);",
        )
        .unwrap();

        let file_node = FileNode::from_file(
            "--",
            &temp_file.path().to_path_buf(),
            &layers,
            fallback_layer,
        )
        .unwrap();

        assert_eq!(file_node.name, "my_schema.my_table");
        assert!(file_node.original_node_name_header.is_some());
        assert_eq!(
            file_node.original_node_name_header.unwrap(),
            "-- node_name: my_schema.my_table"
        );
    }
}
