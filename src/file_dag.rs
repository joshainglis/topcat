use graph_cycles::Cycles;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::path::PathBuf;

use log::{debug, info, trace};
use petgraph::algo::is_cyclic_directed;
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use petgraph::{Directed, Graph};

use crate::exceptions::{FileNodeError, TopCatError};
use crate::file_node::FileNode;
use crate::schema_utils::SchemaFilter;
use crate::sql_parser::SqlAnalyzer;
use crate::stable_topo::StableTopo;
use crate::{config, io_utils};

fn string_slice_to_array<T: Hash + Eq + Clone>(option: Option<&[T]>) -> Option<HashSet<T>> {
    option.map(|arr| arr.iter().cloned().collect())
}

fn collect_files(
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

fn filter_files<'a>(
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
        if let Some(include) = include_extensions {
            if !include.is_empty() {
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
        }
        if let Some(exclude) = exclude_extensions {
            if !exclude.is_empty() {
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
        }
        if let Some(include) = include_file_set {
            if !include.is_empty() && !include.contains::<PathBuf>(path) {
                debug!("Excluding file as it isn't in the include set: {path:?}");
                return false;
            }
        }
        if let Some(exclude) = exclude_file_set {
            if !exclude.is_empty() && exclude.contains::<PathBuf>(path) {
                debug!("Excluding file as it is in the exclude set: {path:?}");
                return false;
            }
        }
        true
    })
}

fn handle_file_node_error(e: FileNodeError) -> Result<(), TopCatError> {
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
    }
}

/// Perform SQL discovery on a file node
fn perform_sql_discovery(
    file_node: &mut FileNode,
    analyzer: &SqlAnalyzer,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the file content
    let content = std::fs::read_to_string(&file_node.path)?;

    // Analyze the SQL content
    let analysis = analyzer.analyze(&content);

    // Update the node name if discovered and not already set
    if let Some(discovered_name) = analysis.node_name {
        if file_node.name_source == crate::file_node::NameSource::Discovered
            || file_node.name.is_empty()
        {
            debug!(
                "Discovered node name: {} for file {:?}",
                discovered_name, file_node.path
            );
            file_node.name = discovered_name;
            file_node.name_source = crate::file_node::NameSource::Discovered;
        }
    }

    // Remove self-dependencies and subobjects
    let mut discovered_deps = analysis.dependencies;
    discovered_deps.remove(&file_node.name);
    for subobj in &analysis.subobjects {
        discovered_deps.remove(subobj);
    }

    // Store the discovered dependencies
    file_node.discovered_deps = Some(discovered_deps);

    debug!(
        "SQL discovery for {:?}: discovered {} dependencies",
        file_node.path,
        file_node
            .discovered_deps
            .as_ref()
            .map(|d| d.len())
            .unwrap_or(0)
    );

    Ok(())
}

fn add_nodes_to_graphs(
    layer_graphs: &mut HashMap<String, DiGraph<FileNode, ()>>,
    layer_index_maps: &mut HashMap<String, HashMap<String, NodeIndex>>,
    name_map: &HashMap<String, FileNode>,
) {
    for file_node in name_map.values() {
        let layer = &file_node.layer;
        let graph = layer_graphs
            .get_mut(layer)
            .expect("Layer graph should exist");
        let index_map = layer_index_maps
            .get_mut(layer)
            .expect("Layer index map should exist");

        let idx = graph.add_node(file_node.clone());
        index_map.insert(file_node.name.clone(), idx);
    }
}

fn validate_dependencies(
    name_map: &HashMap<String, FileNode>,
    layer_graphs: &mut HashMap<String, DiGraph<FileNode, ()>>,
    layer_index_maps: &HashMap<String, HashMap<String, NodeIndex>>,
    layers: &[String],
) -> Result<(), TopCatError> {
    // Create a map from layer name to its index for dependency validation
    let layer_indices: HashMap<String, usize> = layers
        .iter()
        .enumerate()
        .map(|(i, layer)| (layer.clone(), i))
        .collect();

    for file_node in name_map.values() {
        for ensure in &file_node.ensure_exists {
            if !name_map.contains_key(ensure) {
                return Err(TopCatError::MissingExist(
                    file_node.name.clone(),
                    ensure.clone(),
                ));
            }
        }

        for dep in &file_node.deps {
            let dep_node = name_map.get(dep).ok_or_else(|| {
                TopCatError::MissingDependency(file_node.name.clone(), dep.clone())
            })?;

            let file_layer_idx = layer_indices.get(&file_node.layer).unwrap();
            let dep_layer_idx = layer_indices.get(&dep_node.layer).unwrap();

            // Enforce layer ordering: lower index layers cannot depend on higher index layers
            if file_layer_idx < dep_layer_idx {
                return Err(TopCatError::InvalidDependency(
                    file_node.name.clone(),
                    format!(
                        "Node in layer '{}' (index {}) cannot depend on node '{}' in layer '{}' (index {})",
                        file_node.layer,
                        file_layer_idx,
                        dep.clone(),
                        dep_node.layer,
                        dep_layer_idx
                    ),
                ));
            }

            // Only add edges within the same layer
            if file_node.layer == dep_node.layer {
                let graph = layer_graphs.get_mut(&file_node.layer).unwrap();
                let index_map = layer_index_maps.get(&file_node.layer).unwrap();
                graph.add_edge(
                    *index_map.get(dep).unwrap(),
                    *index_map.get(&file_node.name).unwrap(),
                    (),
                );
            }
        }
    }
    Ok(())
}

fn extract_cycle_nodes(
    cycle: Vec<NodeIndex>,
    graph: &Graph<FileNode, (), Directed>,
) -> Vec<FileNode> {
    cycle
        .iter()
        .map(|n| graph.node_weight(*n).unwrap().clone())
        .collect()
}

fn convert_cycle_indexes_to_cycle_nodes(
    cycles: Vec<Vec<NodeIndex>>,
    graph: &Graph<FileNode, (), Directed>,
) -> Vec<Vec<FileNode>> {
    cycles
        .iter()
        .map(|c| extract_cycle_nodes(c.clone(), graph))
        .collect()
}
fn check_cyclic_dependencies(
    layer_graphs: &HashMap<String, DiGraph<FileNode, ()>>,
) -> Result<(), TopCatError> {
    let mut cycles: Vec<Vec<FileNode>> = Vec::new();

    for graph in layer_graphs.values() {
        if is_cyclic_directed(graph) {
            cycles.extend(convert_cycle_indexes_to_cycle_nodes(graph.cycles(), graph));
        }
    }

    if !cycles.is_empty() {
        return Err(TopCatError::CyclicDependency(cycles));
    }
    Ok(())
}

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
    layer_graphs: HashMap<String, DiGraph<FileNode, ()>>,
    layer_index_maps: HashMap<String, HashMap<String, NodeIndex>>,
    layers: Vec<String>,
    fallback_layer: String,
    path_map: HashMap<PathBuf, FileNode>,
    name_map: HashMap<String, FileNode>,
    include_hidden: bool,
    graph_is_built: bool,
    subdir_filter: Option<PathBuf>,
    sql_discovery: crate::sql_config::SqlDiscoveryConfig,
}

impl TCGraph {
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

    pub fn build_graph(&mut self) -> Result<(), TopCatError> {
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

            self.name_map
                .insert(file_node.name.clone(), file_node.clone());
            self.path_map.insert(file_node.path.clone(), file_node);
        }

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
    /// Returns a list of (file_name, missing_dependency) tuples for all missing dependencies.
    /// This allows reporting all missing dependencies at once instead of failing on the first one.
    pub fn validate_dependencies_only(&mut self) -> Result<Vec<(String, String)>, TopCatError> {
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

        // Load all files into name_map (same as build_graph)
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

            self.name_map
                .insert(file_node.name.clone(), file_node.clone());
            self.path_map.insert(file_node.path.clone(), file_node);
        }

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

    pub fn graph_as_dot(
        &self,
        layer_name: &str,
    ) -> Result<Dot<&DiGraph<FileNode, ()>>, TopCatError> {
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

    /// Get all file nodes from the graph
    pub fn get_all_nodes(&self) -> Vec<FileNode> {
        self.name_map.values().cloned().collect()
    }

    pub fn get_sorted_files(&self) -> Result<Vec<PathBuf>, TopCatError> {
        if !self.graph_is_built {
            return Err(TopCatError::GraphMissing);
        }
        info!("Getting sorted files");

        let required_node_names: Option<HashSet<String>> = if let Some(subdir_path) =
            &self.subdir_filter
        {
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
                info!(
                    "No files are found within the specified subdirectory filter: {subdir_path:?}"
                );
                return Ok(Vec::new());
            }

            debug!("Initial nodes from subdir: {initial_nodes:?}");
            Some(self.find_required_nodes(&initial_nodes)?)
        } else {
            None
        };

        if let Some(required) = &required_node_names {
            debug!("Total required nodes (including dependencies): {required:?}");
        }

        let mut sorted_files = Vec::new();

        for layer_name in &self.layers {
            let graph = self.layer_graphs.get(layer_name).unwrap();

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

                let mut should_include = true;

                if let Some(required) = &required_node_names {
                    if !required.contains(&file_node.name) {
                        trace!(
                            "Excluding node '{}' (not required by subdir filter)",
                            file_node.name
                        );
                        should_include = false;
                    }
                }

                if should_include {
                    should_include =
                        match (&self.include_node_prefixes, &self.exclude_node_prefixes) {
                            (Some(include), Some(exclude)) => {
                                include.iter().any(|p| file_node.name.starts_with(p))
                                    && !exclude.iter().any(|p| file_node.name.starts_with(p))
                            }
                            (Some(include), None) => {
                                include.iter().any(|p| file_node.name.starts_with(p))
                            }
                            (None, Some(exclude)) => {
                                !exclude.iter().any(|p| file_node.name.starts_with(p))
                            }
                            (None, None) => true,
                        };
                    if !should_include {
                        trace!("Excluding node '{}' by prefix filter", file_node.name);
                    }
                }

                if should_include {
                    sorted_files.push(file_node.path.clone());
                }
            }
        }
        Ok(sorted_files)
    }

    /// Get all schemas with their nodes grouped together
    /// Returns a HashMap where keys are schema names and values are vectors of nodes in that schema
    pub fn get_schemas(&self) -> HashMap<String, Vec<&FileNode>> {
        let mut schemas: HashMap<String, Vec<&FileNode>> = HashMap::new();

        for node in self.name_map.values() {
            let schema_name = node
                .schema
                .clone()
                .unwrap_or_else(|| "no_schema".to_string());
            schemas.entry(schema_name).or_default().push(node);
        }

        schemas
    }

    /// Get all unique schema names
    pub fn get_schema_names(&self) -> Vec<String> {
        let mut schema_names: HashSet<String> = HashSet::new();

        for node in self.name_map.values() {
            if let Some(schema) = &node.schema {
                schema_names.insert(schema.clone());
            }
        }

        let mut names: Vec<String> = schema_names.into_iter().collect();
        names.sort();
        names
    }

    /// Get internal dependencies (within the same schema) for a given schema
    pub fn get_internal_dependencies(&self, schema: &str) -> HashSet<(String, String)> {
        let mut internal_deps = HashSet::new();

        for node in self.name_map.values() {
            if node.schema.as_deref() == Some(schema) {
                for dep in &node.deps {
                    if let Some(dep_node) = self.name_map.get(dep) {
                        if dep_node.schema.as_deref() == Some(schema) {
                            internal_deps.insert((node.name.clone(), dep.clone()));
                        }
                    }
                }
            }
        }

        internal_deps
    }

    /// Get external dependencies (to other schemas) for a given schema
    /// Returns pairs of (source_node, target_node, target_schema)
    pub fn get_external_dependencies(&self, schema: &str) -> Vec<(String, String, String)> {
        let mut external_deps = Vec::new();

        for node in self.name_map.values() {
            if node.schema.as_deref() == Some(schema) {
                for dep in &node.deps {
                    if let Some(dep_node) = self.name_map.get(dep) {
                        if let Some(dep_schema) = &dep_node.schema {
                            if dep_schema != schema {
                                external_deps.push((
                                    node.name.clone(),
                                    dep.clone(),
                                    dep_schema.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        external_deps
    }

    /// Get schemas that depend on the given schema
    pub fn get_dependent_schemas(&self, schema: &str) -> HashSet<String> {
        let mut dependent_schemas = HashSet::new();

        for node in self.name_map.values() {
            // Check if this node depends on any node in the target schema
            for dep in &node.deps {
                if let Some(dep_node) = self.name_map.get(dep) {
                    if dep_node.schema.as_deref() == Some(schema) {
                        if let Some(node_schema) = &node.schema {
                            if node_schema != schema {
                                dependent_schemas.insert(node_schema.clone());
                            }
                        }
                    }
                }
            }
        }

        dependent_schemas
    }

    /// Get cross-schema dependency pairs (schema_a -> schema_b)
    pub fn get_cross_schema_dependencies(&self) -> Vec<(String, String)> {
        let mut cross_deps = HashSet::new();

        for node in self.name_map.values() {
            if let Some(source_schema) = &node.schema {
                for dep in &node.deps {
                    if let Some(dep_node) = self.name_map.get(dep) {
                        if let Some(target_schema) = &dep_node.schema {
                            if source_schema != target_schema {
                                cross_deps.insert((source_schema.clone(), target_schema.clone()));
                            }
                        }
                    }
                }
            }
        }

        let mut deps: Vec<_> = cross_deps.into_iter().collect();
        deps.sort();
        deps
    }

    /// Apply schema-based node prefix filtering using SchemaFilter.
    ///
    /// This modifies the include_node_prefixes to filter by the specified schemas.
    /// If schemas is empty, no filtering is applied.
    ///
    /// # Arguments
    ///
    /// * `schemas` - List of schema names to filter by
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use topcat::file_dag::TCGraph;
    /// # let mut graph = TCGraph::new(&Default::default());
    /// graph.apply_schema_filter(&["auth".to_string(), "billing".to_string()]);
    /// ```
    pub fn apply_schema_filter(&mut self, schemas: &[String]) {
        let filter = SchemaFilter::from(schemas);
        if filter.is_empty() {
            return;
        }

        // Convert schemas to node prefixes using SchemaFilter
        let schema_prefixes: HashSet<String> = filter.to_node_prefixes().into_iter().collect();

        // Add to include_node_prefixes
        match &mut self.include_node_prefixes {
            Some(existing) => {
                // Intersect with existing prefixes if they exist
                *existing = existing.intersection(&schema_prefixes).cloned().collect();
            }
            None => {
                // Set new prefixes
                self.include_node_prefixes = Some(schema_prefixes);
            }
        }
    }

    /// Get all transitive dependencies of a node (nodes that this node depends on)
    pub fn get_transitive_dependencies(&self, node_name: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut to_visit = vec![node_name.to_string()];

        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }

            if let Some(node) = self.name_map.get(&current) {
                visited.insert(current.clone());

                for dep in &node.deps {
                    if !visited.contains(dep) {
                        to_visit.push(dep.clone());
                    }
                }
            }
        }

        // Remove the starting node itself
        visited.remove(node_name);
        visited
    }

    /// Get all transitive dependents of a node (nodes that depend on this node)
    pub fn get_transitive_dependents(&self, node_name: &str) -> HashSet<String> {
        let dependents_map = self.build_dependents_map();
        let mut visited = HashSet::new();
        let mut to_visit = vec![node_name.to_string()];

        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }

            visited.insert(current.clone());

            if let Some(dependents) = dependents_map.get(&current) {
                for dependent in dependents {
                    if !visited.contains(dependent) {
                        to_visit.push(dependent.clone());
                    }
                }
            }
        }

        // Remove the starting node itself
        visited.remove(node_name);
        visited
    }

    /// Get direct neighbors of a node (both dependencies and dependents)
    /// Returns (dependencies, dependents)
    pub fn get_direct_neighbors(&self, node_name: &str) -> (HashSet<String>, HashSet<String>) {
        let dependencies = if let Some(node) = self.name_map.get(node_name) {
            node.deps.iter().cloned().collect()
        } else {
            HashSet::new()
        };

        let dependents_map = self.build_dependents_map();
        let dependents = dependents_map
            .get(node_name)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        (dependencies, dependents)
    }

    /// Create a filtered subgraph containing only the specified nodes
    pub fn filter_nodes(&self, nodes_to_keep: &HashSet<String>) -> Result<TCGraph, TopCatError> {
        // Create a new graph with the same configuration
        let mut new_graph = TCGraph {
            comment_str: self.comment_str.clone(),
            file_dirs: self.file_dirs.clone(),
            exclude_globs: self.exclude_globs.clone(),
            include_globs: self.include_globs.clone(),
            include_extensions: self.include_extensions.clone(),
            exclude_extensions: self.exclude_extensions.clone(),
            include_node_prefixes: self.include_node_prefixes.clone(),
            exclude_node_prefixes: self.exclude_node_prefixes.clone(),
            layer_graphs: HashMap::new(),
            layer_index_maps: HashMap::new(),
            layers: self.layers.clone(),
            fallback_layer: self.fallback_layer.clone(),
            path_map: HashMap::new(),
            name_map: HashMap::new(),
            include_hidden: self.include_hidden,
            graph_is_built: false,
            subdir_filter: self.subdir_filter.clone(),
            sql_discovery: self.sql_discovery.clone(),
        };

        // Add filtered nodes to name_map and path_map
        for node_name in nodes_to_keep {
            if let Some(node) = self.name_map.get(node_name) {
                new_graph.name_map.insert(node_name.clone(), node.clone());
                new_graph.path_map.insert(node.path.clone(), node.clone());
            }
        }

        // Initialize layer graphs for filtered nodes
        for layer in &new_graph.layers {
            new_graph.layer_graphs.insert(layer.clone(), DiGraph::new());
            new_graph
                .layer_index_maps
                .insert(layer.clone(), HashMap::new());
        }

        // Add nodes to appropriate layer graphs
        for (node_name, node) in &new_graph.name_map {
            let layer_graph = new_graph.layer_graphs.get_mut(&node.layer).unwrap();
            let layer_map = new_graph.layer_index_maps.get_mut(&node.layer).unwrap();
            let idx = layer_graph.add_node(node.clone());
            layer_map.insert(node_name.clone(), idx);
        }

        // Add edges (only if both endpoints are in the filtered set)
        for (node_name, node) in &new_graph.name_map {
            let source_idx = new_graph
                .layer_index_maps
                .get(&node.layer)
                .and_then(|m| m.get(node_name))
                .copied();

            if let Some(source_idx) = source_idx {
                let layer_graph = new_graph.layer_graphs.get_mut(&node.layer).unwrap();
                for dep in &node.deps {
                    // Only add edge if dependency is in the same layer and in the filtered set
                    if nodes_to_keep.contains(dep) {
                        if let Some(dep_node) = new_graph.name_map.get(dep) {
                            if dep_node.layer == node.layer {
                                if let Some(&target_idx) = new_graph
                                    .layer_index_maps
                                    .get(&node.layer)
                                    .and_then(|m| m.get(dep))
                                {
                                    layer_graph.add_edge(source_idx, target_idx, ());
                                }
                            }
                        }
                    }
                }
            }
        }

        new_graph.graph_is_built = true;
        Ok(new_graph)
    }
}
