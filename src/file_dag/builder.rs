//! Graph building utilities and helper functions.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::PathBuf;
use std::rc::Rc;

use log::{debug, info, trace};
use petgraph::graph::{DiGraph, NodeIndex};

use crate::exceptions::{FileNodeError, TopCatError};
use crate::file_node::FileNode;
use crate::io_utils;
use crate::sql_parser::SqlAnalyzer;

/// Convert an optional slice to a HashSet.
pub(super) fn string_slice_to_array<T: Hash + Eq + Clone>(
    option: Option<&[T]>,
) -> Option<HashSet<T>> {
    option.map(|arr| arr.iter().cloned().collect())
}

/// Collect all files from the specified directories.
pub(super) fn collect_files(
    file_dirs: &[PathBuf],
    include_hidden: bool,
) -> Result<HashSet<PathBuf>, TopCatError> {
    let mut files = HashSet::new();
    for dir in file_dirs {
        for f in io_utils::walk_dir(dir, include_hidden)? {
            files.insert(f);
        }
    }
    Ok(files)
}

/// Filter files based on include/exclude patterns and extensions.
pub(super) fn filter_files<'a>(
    files: &'a HashSet<PathBuf>,
    include_file_set: &'a Option<HashSet<PathBuf>>,
    exclude_file_set: &'a Option<HashSet<PathBuf>>,
    include_extensions: &'a Option<HashSet<String>>,
    exclude_extensions: &'a Option<HashSet<String>>,
) -> impl Iterator<Item = &'a PathBuf> + 'a {
    debug!("files: {files:?}");
    debug!("include files: {include_file_set:?}");
    debug!("exclude files: {exclude_file_set:?}");
    debug!("include extensions: {include_extensions:?}");
    debug!("exclude extensions: {exclude_extensions:?}");
    files.iter().filter(move |path| {
        trace!("checking filters for path: {path:?}");
        if let Some(include) = include_extensions
            && !include.is_empty()
        {
            let ext = match path.extension() {
                Some(e) => e.to_string_lossy().to_lowercase(),
                None => return false,
            };
            if !include.contains(&ext) {
                debug!(
                    "Excluding file {path:?} as its extension {ext:?} isn't in the include set: {include:?}"
                );
                return false;
            }
        }
        if let Some(exclude) = exclude_extensions
            && !exclude.is_empty()
        {
            let ext = match path.extension() {
                Some(e) => e.to_string_lossy().to_lowercase(),
                None => return false,
            };
            if exclude.contains(&ext) {
                debug!(
                    "Excluding file {path:?} as its extension '{ext:?}' is in the exclude set: {exclude:?}"
                );
                return false;
            }
        }
        if let Some(include) = include_file_set
            && !include.is_empty() && !include.contains::<PathBuf>(path)
        {
            debug!("Excluding file as it isn't in the include set: {path:?}");
            return false;
        }
        if let Some(exclude) = exclude_file_set
            && !exclude.is_empty() && exclude.contains::<PathBuf>(path)
        {
            debug!("Excluding file as it is in the exclude set: {path:?}");
            return false;
        }
        true
    })
}

/// Handle FileNode errors, converting them to TopCatError or logging as info.
pub(super) fn handle_file_node_error(e: FileNodeError) -> Result<(), TopCatError> {
    match e {
        FileNodeError::NoNameDefined(p) => {
            info!("Ignoring {p:?}: No name defined in file header");
            Ok(())
        }
        FileNodeError::TooManyNames(p, s) => Err(TopCatError::InvalidFileHeader(
            p,
            format!("Too many names declared: {}", s.join(", ")),
        )),
        FileNodeError::InvalidLayer(p, layer) => Err(TopCatError::InvalidFileHeader(
            p,
            format!("Invalid layer '{layer}' declared"),
        )),
        FileNodeError::FileOpen(p, err) => Err(TopCatError::InvalidFileHeader(
            p,
            format!("Failed to open file: {err}"),
        )),
    }
}

/// Perform SQL discovery on a file node.
///
/// This analyzes SQL content to discover node names and dependencies automatically.
pub(super) fn perform_sql_discovery(
    file_node: &mut FileNode,
    analyzer: &SqlAnalyzer,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the file content
    let content = std::fs::read_to_string(&file_node.path)?;

    // Analyze the SQL content
    let analysis = analyzer.analyze(&content);

    // Update the node name if discovered and not already set
    if let Some(discovered_name) = analysis.node_name
        && (file_node.name_source == crate::file_node::NameSource::Discovered
            || file_node.name.is_empty())
    {
        debug!(
            "Discovered node name: {} for file {:?}",
            discovered_name, file_node.path
        );
        file_node.name = discovered_name;
        file_node.name_source = crate::file_node::NameSource::Discovered;
    }

    // Remove self-dependencies and subobjects
    let mut discovered_deps = analysis.dependencies;
    discovered_deps.remove(&file_node.name);
    for subobj in &analysis.subobjects {
        discovered_deps.remove(subobj);
    }

    // Store the discovered dependencies
    file_node.discovered_deps = Some(discovered_deps);

    // Set implicit flag if CREATE CAST/OPERATOR was detected
    if analysis.has_implicit {
        file_node.implicit = true;
    }

    debug!(
        "SQL discovery for {:?}: discovered {} dependencies{}",
        file_node.path,
        file_node
            .discovered_deps
            .as_ref()
            .map(|d| d.len())
            .unwrap_or(0),
        if file_node.implicit {
            " (implicit)"
        } else {
            ""
        }
    );

    Ok(())
}

/// Add FileNode instances to their corresponding layer graphs.
pub(super) fn add_nodes_to_graphs(
    layer_graphs: &mut HashMap<String, DiGraph<Rc<FileNode>, ()>>,
    layer_index_maps: &mut HashMap<String, HashMap<String, NodeIndex>>,
    name_map: &HashMap<String, Rc<FileNode>>,
) {
    for file_node in name_map.values() {
        let layer = &file_node.layer;
        let graph = layer_graphs
            .get_mut(layer)
            .expect("Layer graph should exist");
        let index_map = layer_index_maps
            .get_mut(layer)
            .expect("Layer index map should exist");

        let idx = graph.add_node(Rc::clone(file_node));
        index_map.insert(file_node.name.clone(), idx);
    }
}
