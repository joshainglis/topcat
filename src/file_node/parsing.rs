//! File parsing and header extraction for FileNode.

use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

use crate::exceptions::FileNodeError;

use super::{FileNode, NameSource};

/// Read header lines from a file.
///
/// Reads lines from the beginning of the file that start with the comment string,
/// stopping at the first non-comment, non-empty line.
pub fn get_file_headers(path: &PathBuf, comment_str: &str) -> io::Result<Vec<String>> {
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

/// Split a dependency line into individual dependencies.
pub fn split_dependencies(line: &str) -> Vec<String> {
    line.split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|x| {
            let x = x.trim().to_string();
            if !x.is_empty() { Some(x) } else { None }
        })
        .collect()
}

/// Split dependencies and identify which ones have the override prefix (!)
/// Returns (normal_deps, override_deps)
pub fn split_dependencies_with_overrides(line: &str) -> (Vec<String>, Vec<String>) {
    let mut normal_deps = Vec::new();
    let mut override_deps = Vec::new();

    for item in split_dependencies(line) {
        if let Some(stripped) = item.strip_prefix('!') {
            override_deps.push(stripped.to_string());
        } else {
            normal_deps.push(item);
        }
    }

    (normal_deps, override_deps)
}

/// Parse a file and create a FileNode from its headers.
///
/// # Arguments
///
/// * `comment_str` - The comment prefix for the file type (e.g., "--" for SQL)
/// * `path` - Path to the file to parse
/// * `layers` - Valid layer names
/// * `fallback_layer` - Default layer if none specified
/// * `layer_mapper` - Optional automatic layer mapping based on node name patterns
///
/// # Returns
///
/// A `FileNode` with parsed metadata, or an error if parsing fails.
pub fn from_file(
    comment_str: &str,
    path: &PathBuf,
    layers: &[String],
    fallback_layer: &str,
    layer_mapper: Option<&crate::layer_mapper::LayerMapper>,
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
            let (normal, overrides) = split_dependencies_with_overrides(&line[prefix_len..]);
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
            for item in split_dependencies(&line[ensure_exists_str.len()..]) {
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

    // Apply automatic layer mapping if:
    // 1. Layer is still at fallback (no explicit layer header)
    // 2. Layer mapper is configured
    // 3. Node name matches a pattern
    if layer_is_fallback
        && let Some(mapper) = layer_mapper
        && let Some(mapped_layer) = mapper.map_node_to_layer(&name)
    {
        layer = mapped_layer;
        layer_is_fallback = false;
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
    file_node.name_source = NameSource::Header;
    // Only preserve original headers if manual flag is set
    file_node.original_headers = if manual { original_headers_text } else { None };
    Ok(file_node)
}
