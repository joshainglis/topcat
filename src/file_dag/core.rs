//! Core TCGraph struct and fundamental graph operations.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;

use log::{debug, info, trace};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};

use crate::exceptions::TopCatError;
use crate::file_node::FileNode;
use crate::sql_parser::SqlAnalyzer;
use crate::stable_topo::StableTopo;
use crate::{config, io_utils};

use super::builder::{
    add_nodes_to_graphs, collect_files, filter_files, handle_file_node_error,
    perform_sql_discovery, string_slice_to_array,
};
use super::validation::{check_cyclic_dependencies, validate_dependencies};

/// Represents a graph structure for a set of files and their dependencies.
pub struct TCGraph {
    pub comment_str: String,
    pub file_dirs: Vec<PathBuf>,
    pub exclude_globs: Option<HashSet<PathBuf>>,
    pub include_globs: Option<HashSet<PathBuf>>,
    pub include_extensions: Option<HashSet<String>>,
    pub exclude_extensions: Option<HashSet<String>>,
    pub include_node_prefixes: Option<HashSet<String>>,
    pub exclude_node_prefixes: Option<HashSet<String>>,
    pub(super) layer_graphs: HashMap<String, DiGraph<Rc<FileNode>, ()>>,
    pub(super) layer_index_maps: HashMap<String, HashMap<String, NodeIndex>>,
    pub(super) layers: Vec<String>,
    pub(super) fallback_layer: String,
    pub(super) path_map: HashMap<PathBuf, Rc<FileNode>>,
    pub(super) name_map: HashMap<String, Rc<FileNode>>,
    pub(super) include_hidden: bool,
    pub(super) graph_is_built: bool,
    pub(super) subdir_filter: Option<PathBuf>,
    pub(super) sql_discovery: crate::sql_config::SqlDiscoveryConfig,
}

impl TCGraph {
    /// Create a new TCGraph from the given configuration.
    pub fn new(config: &config::Config) -> TCGraph {
        let include_globs = config
            .include_globs
            .map(|patterns| io_utils::glob_files(patterns).unwrap_or_default());
        let exclude_globs = config
            .exclude_globs
            .map(|patterns| io_utils::glob_files(patterns).unwrap_or_default());
        let include_extensions: Option<HashSet<String>> =
            string_slice_to_array(config.include_extensions);
        let exclude_extensions: Option<HashSet<String>> =
            string_slice_to_array(config.exclude_extensions);
        let include_node_prefixes: Option<HashSet<String>> =
            string_slice_to_array(config.include_node_prefixes);
        let exclude_node_prefixes: Option<HashSet<String>> =
            string_slice_to_array(config.exclude_node_prefixes);

        // Initialize graphs and index maps for each layer
        let mut layer_graphs = HashMap::new();
        let mut layer_index_maps = HashMap::new();
        for layer in &config.layers {
            layer_graphs.insert(layer.clone(), DiGraph::new());
            layer_index_maps.insert(layer.clone(), HashMap::new());
        }

        TCGraph {
            comment_str: config.comment_str.clone(),
            file_dirs: config.input_dirs.clone(),
            exclude_globs,
            include_globs,
            include_extensions,
            exclude_extensions,
            include_node_prefixes,
            exclude_node_prefixes,
            layer_graphs,
            layer_index_maps,
            layers: config.layers.clone(),
            fallback_layer: config.fallback_layer.clone(),
            path_map: HashMap::new(),
            name_map: HashMap::new(),
            include_hidden: config.include_hidden,
            graph_is_built: false,
            subdir_filter: config.subdir_filter.clone(),
            sql_discovery: config.sql_discovery.clone(),
        }
    }

    /// Load FileNodes from configured files into name_map and path_map.
    ///
    /// This is the common logic shared by build_graph() and validate_dependencies_only().
    /// It collects, filters, and parses files, optionally performing SQL discovery.
    fn load_file_nodes(&mut self) -> Result<(), TopCatError> {
        debug!("include globs: {:?}", self.include_globs);
        debug!("exclude globs: {:?}", self.exclude_globs);
        debug!("include extensions: {:?}", self.include_extensions);
        debug!("exclude extensions: {:?}", self.exclude_extensions);

        let files = collect_files(&self.file_dirs, self.include_hidden)?;
        let filtered_files = filter_files(
            &files,
            &self.include_globs,
            &self.exclude_globs,
            &self.include_extensions,
            &self.exclude_extensions,
        );

        // Create SQL analyzer if discovery is enabled
        let sql_analyzer = if self.sql_discovery.enabled {
            Some(SqlAnalyzer::new(self.sql_discovery.clone()).map_err(|e| {
                TopCatError::ConfigError(format!("Failed to create SQL analyzer: {e}"))
            })?)
        } else {
            None
        };

        for file in filtered_files {
            let mut file_node = match FileNode::from_file(
                &self.comment_str,
                file,
                &self.layers,
                &self.fallback_layer,
            ) {
                Ok(f) => f,
                Err(e) => {
                    handle_file_node_error(e)?;
                    continue;
                }
            };

            // Perform SQL discovery if enabled
            if let Some(ref analyzer) = sql_analyzer {
                match perform_sql_discovery(&mut file_node, analyzer) {
                    Ok(_) => {
                        // Merge discovered dependencies based on strategy
                        file_node.merge_dependencies(self.sql_discovery.merge_strategy);
                    }
                    Err(e) => {
                        info!("SQL discovery failed for {:?}: {}", file_node.path, e);
                    }
                }
            }

            if let Some(other_path) = self.name_map.get(&file_node.name) {
                return Err(TopCatError::NameClash(
                    file_node.name,
                    file_node.path,
                    other_path.path.clone(),
                ));
            }

            let file_node_rc = Rc::new(file_node);
            self.name_map
                .insert(file_node_rc.name.clone(), Rc::clone(&file_node_rc));
            self.path_map
                .insert(file_node_rc.path.clone(), file_node_rc);
        }

        Ok(())
    }

    /// Build the dependency graph from the configured files.
    ///
    /// This will:
    /// 1. Collect and filter files from the input directories
    /// 2. Parse file headers to extract dependency metadata
    /// 3. Optionally perform SQL discovery to extract additional dependencies
    /// 4. Build layer graphs with nodes and edges
    /// 5. Validate dependencies and check for cycles
    pub fn build_graph(&mut self) -> Result<(), TopCatError> {
        self.load_file_nodes()?;

        add_nodes_to_graphs(
            &mut self.layer_graphs,
            &mut self.layer_index_maps,
            &self.name_map,
        );

        validate_dependencies(
            &self.name_map,
            &mut self.layer_graphs,
            &self.layer_index_maps,
            &self.layers,
        )?;

        check_cyclic_dependencies(&self.layer_graphs)?;

        self.graph_is_built = true;
        Ok(())
    }

    /// Validate all dependencies without building the graph.
    ///
    /// Returns a list of (file_name, missing_dependency) tuples for all missing dependencies.
    /// This allows reporting all missing dependencies at once instead of failing on the first one.
    pub fn validate_dependencies_only(&mut self) -> Result<Vec<(String, String)>, TopCatError> {
        // Load all files into name_map (same as build_graph)
        self.load_file_nodes()?;

        // Now collect all missing dependencies instead of failing on the first one
        let mut missing_deps = Vec::new();

        for file_node in self.name_map.values() {
            for dep in &file_node.deps {
                if !self.name_map.contains_key(dep) {
                    missing_deps.push((file_node.name.clone(), dep.clone()));
                }
            }
        }

        Ok(missing_deps)
    }

    /// Find all nodes required by the given initial nodes (transitive closure).
    fn find_required_nodes(
        &self,
        initial_nodes: &HashSet<String>,
    ) -> Result<HashSet<String>, TopCatError> {
        let mut required = HashSet::new();
        let mut queue: VecDeque<String> = initial_nodes.iter().cloned().collect();
        required.extend(initial_nodes.iter().cloned());

        while let Some(node_name) = queue.pop_front() {
            let file_node = self.name_map.get(&node_name).ok_or_else(|| {
                TopCatError::UnknownError(format!(
                    "Node '{node_name}' not found in name_map during dependency traversal."
                ))
            })?;

            for dep_name in &file_node.deps {
                if !self.name_map.contains_key(dep_name) {
                    return Err(TopCatError::MissingDependency(
                        node_name.clone(),
                        dep_name.clone(),
                    ));
                }
                if required.insert(dep_name.clone()) {
                    queue.push_back(dep_name.clone());
                }
            }
        }
        Ok(required)
    }

    /// Generate a DOT representation of a layer graph for visualization.
    pub fn graph_as_dot(
        &self,
        layer_name: &str,
    ) -> Result<Dot<'_, &DiGraph<Rc<FileNode>, ()>>, TopCatError> {
        if !self.graph_is_built {
            return Err(TopCatError::GraphMissing);
        }
        let graph = self
            .layer_graphs
            .get(layer_name)
            .ok_or_else(|| TopCatError::UnknownError(format!("Layer '{layer_name}' not found")))?;
        let dot = Dot::with_attr_getters(
            graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, _| String::new(),
            &|_, (_, f)| format!("label=\"{}\"", f.name),
        );
        Ok(dot)
    }

    /// Get all file nodes from the graph.
    ///
    /// Note: This clones all FileNodes. For read-only iteration, prefer `nodes()`.
    pub fn get_all_nodes(&self) -> Vec<FileNode> {
        self.name_map.values().map(|arc| (**arc).clone()).collect()
    }

    /// Get an iterator over all file nodes (Rc references).
    ///
    /// This is more efficient than `get_all_nodes()` as it avoids cloning.
    pub fn nodes(&self) -> impl Iterator<Item = &Rc<FileNode>> {
        self.name_map.values()
    }

    /// Apply subdirectory filter to find nodes within subdirectory and their dependencies.
    ///
    /// Returns None if no subdirectory filter is set, or Some(HashSet) of required node names.
    fn apply_subdirectory_filter(&self) -> Result<Option<HashSet<String>>, TopCatError> {
        let Some(subdir_path) = &self.subdir_filter else {
            return Ok(None);
        };

        info!("Applying subdirectory filter: {subdir_path:?}");
        let canonical_subdir_path = subdir_path.canonicalize().map_err(TopCatError::Io)?;

        let initial_nodes: HashSet<String> = self
            .name_map
            .values()
            .filter_map(|node| {
                node.path
                    .canonicalize()
                    .ok()
                    .and_then(|canonical_node_path| {
                        if canonical_node_path.starts_with(&canonical_subdir_path) {
                            Some(node.name.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect();

        if initial_nodes.is_empty() {
            info!("No files are found within the specified subdirectory filter: {subdir_path:?}");
            return Ok(Some(HashSet::new()));
        }

        debug!("Initial nodes from subdir: {initial_nodes:?}");
        let required = self.find_required_nodes(&initial_nodes)?;
        debug!("Total required nodes (including dependencies): {required:?}");
        Ok(Some(required))
    }

    /// Check if a node should be included based on prefix filters.
    fn matches_prefix_filters(&self, node_name: &str) -> bool {
        match (&self.include_node_prefixes, &self.exclude_node_prefixes) {
            (Some(include), Some(exclude)) => {
                include.iter().any(|p| node_name.starts_with(p))
                    && !exclude.iter().any(|p| node_name.starts_with(p))
            }
            (Some(include), None) => include.iter().any(|p| node_name.starts_with(p)),
            (None, Some(exclude)) => !exclude.iter().any(|p| node_name.starts_with(p)),
            (None, None) => true,
        }
    }

    /// Check if a node should be included in the output based on all filters.
    fn should_include_node(
        &self,
        file_node: &FileNode,
        required_node_names: &Option<HashSet<String>>,
    ) -> bool {
        // Check subdirectory filter
        if let Some(required) = required_node_names {
            if required.is_empty() {
                // Empty set means no files in subdirectory
                return false;
            }
            if !required.contains(&file_node.name) {
                trace!(
                    "Excluding node '{}' (not required by subdir filter)",
                    file_node.name
                );
                return false;
            }
        }

        // Check prefix filters
        if !self.matches_prefix_filters(&file_node.name) {
            trace!("Excluding node '{}' by prefix filter", file_node.name);
            return false;
        }

        true
    }

    /// Get files in topologically sorted order, respecting layer constraints.
    ///
    /// This returns file paths in an order where all dependencies come before their dependents.
    /// Layer ordering is enforced: all files in earlier layers come before later layers.
    ///
    /// Applies subdirectory and prefix filters if configured.
    pub fn get_sorted_files(&self) -> Result<Vec<PathBuf>, TopCatError> {
        if !self.graph_is_built {
            return Err(TopCatError::GraphMissing);
        }
        info!("Getting sorted files");

        let required_node_names = self.apply_subdirectory_filter()?;

        // Early return if subdirectory filter resulted in empty set
        if let Some(ref required) = required_node_names
            && required.is_empty()
        {
            return Ok(Vec::new());
        }

        let mut sorted_files = Vec::new();

        for layer_name in &self.layers {
            let graph = self
                .layer_graphs
                .get(layer_name)
                .expect("Layer graph should exist for configured layer");

            debug!(
                "{} graph: {:?} nodes and {:?} edges",
                layer_name,
                graph.node_count(),
                graph.edge_count()
            );

            let topo = StableTopo::new(graph);
            for node_idx in topo {
                let file_node = match graph.node_weight(node_idx) {
                    Some(x) => x,
                    None => return Err(TopCatError::UnknownError("Node not found".to_string())),
                };
                trace!("{} node: {:?}", layer_name, file_node.name);

                if self.should_include_node(file_node, &required_node_names) {
                    sorted_files.push(file_node.path.clone());
                }
            }
        }
        Ok(sorted_files)
    }

    /// Get the layer graph for a given layer name.
    ///
    /// Returns a reference to the petgraph DiGraph for the specified layer.
    /// Useful for direct graph operations like tree building.
    pub fn get_layer_graph(&self, layer: &str) -> Option<&DiGraph<Rc<FileNode>, ()>> {
        self.layer_graphs.get(layer)
    }

    /// Get the layer index map for a given layer name.
    ///
    /// Returns a reference to the map from node names to their indices in the layer graph.
    pub fn get_layer_index_map(&self, layer: &str) -> Option<&HashMap<String, NodeIndex>> {
        self.layer_index_maps.get(layer)
    }

    /// Get all layer graphs.
    ///
    /// Returns a reference to all layer graphs.
    pub fn get_all_layer_graphs(&self) -> &HashMap<String, DiGraph<Rc<FileNode>, ()>> {
        &self.layer_graphs
    }

    /// Get all layer index maps.
    ///
    /// Returns a reference to all layer index maps.
    pub fn get_all_layer_index_maps(&self) -> &HashMap<String, HashMap<String, NodeIndex>> {
        &self.layer_index_maps
    }
}
