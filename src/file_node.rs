use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::Display;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io;
use std::io::BufRead;
use std::path::PathBuf;

use crate::exceptions::FileNodeError;

fn get_file_headers(path: &PathBuf, comment_str: &str) -> Vec<String> {
    let file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", path.display(), why.to_string()),
        Ok(file) => file,
    };

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
        .collect::<io::Result<_>>()
        .unwrap_or_else(|_| vec![]);

    // remove any empty lines from the vector and return
    file_data
        .iter()
        .filter(|x| !x.is_empty())
        .map(|x| x.to_string())
        .collect()
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub deps: HashSet<String>,
    pub layer: String,
    pub ensure_exists: HashSet<String>,
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
        ensure_exists: HashSet<String>,
    ) -> FileNode {
        FileNode {
            name,
            path,
            deps,
            layer,
            ensure_exists,
        }
    }

    fn split_dependencies(line: &str) -> Vec<String> {
        line.split(|c: char| c.is_whitespace() || c == ',')
            .filter_map(|x| {
                let x = x.trim().to_string();
                if !x.is_empty() {
                    Some(x)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn from_file(
        comment_str: &str,
        path: &PathBuf,
        layers: &[String],
        fallback_layer: &str,
    ) -> Result<FileNode, FileNodeError> {
        let file_data = get_file_headers(&path, comment_str);
        let name_str = format!("{} name:", comment_str);
        let dep_str = format!("{} requires:", comment_str);
        let drop_str = format!("{} dropped_by:", comment_str);
        let layer_str = format!("{} layer:", comment_str);
        // Keep backward compatibility with old headers
        let prepend_str = format!("{} is_initial", comment_str);
        let append_str = format!("{} is_final", comment_str);
        let ensure_exists_str = format!("{} exists:", comment_str);

        let mut name = String::new();
        let mut deps = HashSet::new();
        let mut layer = fallback_layer.to_string();
        let mut ensure_exists = HashSet::new();

        for unprocessed_line in &file_data {
            let line = unprocessed_line.trim().to_lowercase();
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
            } else if line.starts_with(&dep_str) {
                // -- requires: tomato, potato orange -> ["tomato", "potato", "orange"]
                // Should split on comma or space and then trim. Don't insert empty strings
                for item in Self::split_dependencies(&line[dep_str.len()..]) {
                    deps.insert(item);
                }
            } else if line.starts_with(&drop_str) {
                // -- dropped_by: tomato, potato -> ["tomato", "potato"]
                for item in Self::split_dependencies(&line[drop_str.len()..]) {
                    deps.insert(item);
                }
            } else if line.starts_with(&layer_str) {
                // -- layer: prepend -> "prepend"
                let declared_layer = line[layer_str.len()..].trim();
                if !declared_layer.is_empty() {
                    layer = declared_layer.to_string();
                }
            } else if line.starts_with(&prepend_str) {
                // -- is_initial -> "prepend" (backward compatibility)
                layer = "prepend".to_string();
            } else if line.starts_with(&append_str) {
                // -- is_final -> "append" (backward compatibility)
                layer = "append".to_string();
            } else if line.starts_with(&ensure_exists_str) {
                // --exists: tomato, potato -> ["tomato", "potato"]
                for item in Self::split_dependencies(&line[ensure_exists_str.len()..]) {
                    ensure_exists.insert(item);
                }
            }
        }
        if name.is_empty() {
            return Err(FileNodeError::NoNameDefined(path.clone()));
        }

        // Validate that the declared layer exists in the configured layers
        if !layers.contains(&layer) {
            return Err(FileNodeError::InvalidLayer(path.clone(), layer));
        }

        Ok(FileNode::new(
            name,
            path.clone(),
            deps,
            layer,
            ensure_exists,
        ))
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
}
