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
use crate::stable_topo::StableTopo;
use crate::{config, io_utils};

fn string_slice_to_array<T: Hash + Eq + Clone>(option: Option<&[T]>) -> Option<HashSet<T>> {
    match option {
        Some(arr) => Some(arr.iter().cloned().collect()),
        None => None,
    }
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
    debug!("files: {:?}", files);
    debug!("include files: {:?}", include_file_set);
    debug!("exclude files: {:?}", exclude_file_set);
    debug!("include extensions: {:?}", include_extensions);
    debug!("exclude extensions: {:?}", exclude_extensions);
    files.iter().filter(move |path| {
        trace!("checking filters for path: {:?}", path);
        if let Some(ref include) = include_extensions {
            if !include.is_empty() {
                let ext = match path.extension() {
                    Some(e) => e.to_string_lossy().to_lowercase(),
                    None => return false,
                };
                if !include.contains(&ext) {
                    debug!(
                        "Excluding file {:?} as its extension {:?} isn't in the include set: {:?}",
                        path, ext, include
                    );
                    return false;
                }
            }
        }
        if let Some(ref exclude) = exclude_extensions {
            if !exclude.is_empty() {
                let ext = match path.extension() {
                    Some(e) => e.to_string_lossy().to_lowercase(),
                    None => return false,
                };
                if exclude.contains(&ext) {
                    debug!(
                        "Excluding file {:?} as its extension '{:?}' is in the exclude set: {:?}",
                        path, ext, exclude
                    );
                    return false;
                }
            }
        }
        if let Some(ref include) = include_file_set {
            if !include.is_empty() && !include.contains::<PathBuf>(&*path) {
                debug!("Excluding file as it isn't in the include set: {:?}", path);
                return false;
            }
        }
        if let Some(ref exclude) = exclude_file_set {
            if !exclude.is_empty() && exclude.contains::<PathBuf>(&*path) {
                debug!("Excluding file as it is in the exclude set: {:?}", path);
                return false;
            }
        }
        true
    })
}

fn handle_file_node_error(e: FileNodeError) -> Result<(), TopCatError> {
    return match e {
        FileNodeError::NoNameDefined(p) => {
            info!("Ignoring {:?}: No name defined in file header", p);
            Ok(())
        }
        FileNodeError::TooManyNames(p, s) => Err(TopCatError::InvalidFileHeader(
            p,
            format!("Too many names declared: {}", s.join(", ")),
        )),
        FileNodeError::InvalidLayer(p, layer) => Err(TopCatError::InvalidFileHeader(
            p,
            format!("Invalid layer '{}' declared", layer),
        )),
    };
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
                        file_node.layer, file_layer_idx, dep.clone(), dep_node.layer, dep_layer_idx
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

        for file in filtered_files {
            let file_node = match FileNode::from_file(
                &self.comment_str,
                &file,
                &self.layers,
                &self.fallback_layer,
            ) {
                Ok(f) => f,
                Err(e) => {
                    handle_file_node_error(e)?;
                    continue;
                }
            };

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
                    "Node '{}' not found in name_map during dependency traversal.",
                    node_name
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
        let graph = self.layer_graphs.get(layer_name).ok_or_else(|| {
            TopCatError::UnknownError(format!("Layer '{}' not found", layer_name))
        })?;
        let dot = Dot::with_attr_getters(
            graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, _| String::new(),
            &|_, (_, f)| format!("label=\"{}\"", f.name),
        );
        Ok(dot)
    }

    pub fn get_sorted_files(&self) -> Result<Vec<PathBuf>, TopCatError> {
        if !self.graph_is_built {
            return Err(TopCatError::GraphMissing);
        }
        info!("Getting sorted files");

        let required_node_names: Option<HashSet<String>> =
            if let Some(subdir_path) = &self.subdir_filter {
                info!("Applying subdirectory filter: {:?}", subdir_path);
                let canonical_subdir_path =
                    subdir_path.canonicalize().map_err(|e| TopCatError::Io(e))?;

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
                        "No files are found within the specified subdirectory filter: {:?}",
                        subdir_path
                    );
                    return Ok(Vec::new());
                }

                debug!("Initial nodes from subdir: {:?}", initial_nodes);
                Some(self.find_required_nodes(&initial_nodes)?)
            } else {
                None
            };

        if let Some(required) = &required_node_names {
            debug!(
                "Total required nodes (including dependencies): {:?}",
                required
            );
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

            let mut topo = StableTopo::new(graph);
            while let Some(node_idx) = topo.next() {
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
}
