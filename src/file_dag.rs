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

/// The `TCGraphType` enum represents the different types of graph modifications.
///
/// These modifications can be applied to a graph to manipulate its content.
///
/// # Variants
///
/// - `Normal`: Indicates no modifications will be applied to the graph.
/// - `Prepend`: Indicates that new elements will be prepended to the graph.
/// - `Append`: Indicates that new elements will be appended to the graph.
pub enum TCGraphType {
    Normal,
    Prepend,
    Append,
}

impl TCGraphType {
    pub fn as_str(&self) -> &str {
        match self {
            TCGraphType::Normal => "normal",
            TCGraphType::Prepend => "prepend",
            TCGraphType::Append => "append",
        }
    }
}

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
    };
}

fn add_nodes_to_graphs(
    prepend_graph: &mut Graph<FileNode, (), Directed>,
    append_graph: &mut Graph<FileNode, (), Directed>,
    normal_graph: &mut Graph<FileNode, (), Directed>,
    prepend_index_map: &mut HashMap<String, NodeIndex>,
    append_index_map: &mut HashMap<String, NodeIndex>,
    normal_index_map: &mut HashMap<String, NodeIndex>,
    name_map: &HashMap<String, FileNode>,
) {
    for file_node in name_map.values() {
        let idx: NodeIndex;
        if file_node.prepend {
            idx = prepend_graph.add_node(file_node.clone());
            prepend_index_map.insert(file_node.name.clone(), idx);
        } else if file_node.append {
            idx = append_graph.add_node(file_node.clone());
            append_index_map.insert(file_node.name.clone(), idx);
        } else {
            idx = normal_graph.add_node(file_node.clone());
            normal_index_map.insert(file_node.name.clone(), idx);
        }
    }
}

fn validate_dependencies(
    name_map: &HashMap<String, FileNode>,
    prepend_graph: &mut Graph<FileNode, (), Directed>,
    append_graph: &mut Graph<FileNode, (), Directed>,
    normal_graph: &mut Graph<FileNode, (), Directed>,
    prepend_index_map: &HashMap<String, NodeIndex>,
    append_index_map: &HashMap<String, NodeIndex>,
    normal_index_map: &HashMap<String, NodeIndex>,
) -> Result<(), TopCatError> {
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

            if file_node.prepend {
                if !dep_node.prepend {
                    return Err(TopCatError::InvalidDependency(
                        file_node.name.clone(),
                        dep.clone(),
                    ));
                }
                prepend_graph.add_edge(
                    *prepend_index_map.get(dep).unwrap(),
                    *prepend_index_map.get(&file_node.name).unwrap(),
                    (),
                );
            } else if file_node.append {
                if dep_node.append {
                    append_graph.add_edge(
                        *append_index_map.get(dep).unwrap(),
                        *append_index_map.get(&file_node.name).unwrap(),
                        (),
                    );
                }
            } else {
                if dep_node.append {
                    return Err(TopCatError::InvalidDependency(
                        file_node.name.clone(),
                        dep.clone(),
                    ));
                } else if !dep_node.prepend {
                    normal_graph.add_edge(
                        *normal_index_map.get(dep).unwrap(),
                        *normal_index_map.get(&file_node.name).unwrap(),
                        (),
                    );
                }
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
    normal_graph: &Graph<FileNode, (), Directed>,
    prepend_graph: &Graph<FileNode, (), Directed>,
    append_graph: &Graph<FileNode, (), Directed>,
) -> Result<(), TopCatError> {
    let mut cycles: Vec<Vec<FileNode>> = Vec::new();
    if is_cyclic_directed(prepend_graph) {
        cycles.extend(convert_cycle_indexes_to_cycle_nodes(
            prepend_graph.cycles(),
            prepend_graph,
        ));
    }
    if is_cyclic_directed(normal_graph) {
        cycles.extend(convert_cycle_indexes_to_cycle_nodes(
            normal_graph.cycles(),
            normal_graph,
        ));
    }
    if is_cyclic_directed(append_graph) {
        cycles.extend(convert_cycle_indexes_to_cycle_nodes(
            append_graph.cycles(),
            append_graph,
        ));
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
    normal_graph: DiGraph<FileNode, ()>,
    prepend_graph: DiGraph<FileNode, ()>,
    append_graph: DiGraph<FileNode, ()>,
    path_map: HashMap<PathBuf, FileNode>,
    name_map: HashMap<String, FileNode>,
    normal_index_map: HashMap<String, NodeIndex>,
    prepend_index_map: HashMap<String, NodeIndex>,
    append_index_map: HashMap<String, NodeIndex>,
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

        TCGraph {
            comment_str: config.comment_str.clone(),
            file_dirs: config.input_dirs.clone(),
            exclude_globs,
            include_globs,
            include_extensions,
            exclude_extensions,
            include_node_prefixes,
            exclude_node_prefixes,
            normal_graph: DiGraph::new(),
            prepend_graph: DiGraph::new(),
            append_graph: DiGraph::new(),
            path_map: HashMap::new(),
            name_map: HashMap::new(),
            normal_index_map: HashMap::new(),
            prepend_index_map: HashMap::new(),
            append_index_map: HashMap::new(),
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
            let file_node = match FileNode::from_file(&self.comment_str, &file) {
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
            &mut self.prepend_graph,
            &mut self.append_graph,
            &mut self.normal_graph,
            &mut self.prepend_index_map,
            &mut self.append_index_map,
            &mut self.normal_index_map,
            &self.name_map,
        );

        validate_dependencies(
            &self.name_map,
            &mut self.prepend_graph,
            &mut self.append_graph,
            &mut self.normal_graph,
            &self.prepend_index_map,
            &self.append_index_map,
            &self.normal_index_map,
        )?;

        check_cyclic_dependencies(&self.normal_graph, &self.prepend_graph, &self.append_graph)?;

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
        graph_type: TCGraphType,
    ) -> Result<Dot<&DiGraph<FileNode, ()>>, TopCatError> {
        if !self.graph_is_built {
            return Err(TopCatError::GraphMissing);
        }
        let graph = match graph_type {
            TCGraphType::Normal => &self.normal_graph,
            TCGraphType::Prepend => &self.prepend_graph,
            TCGraphType::Append => &self.append_graph,
        };
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

        for graph_type in [
            TCGraphType::Prepend,
            TCGraphType::Normal,
            TCGraphType::Append,
        ]
        .iter()
        {
            let graph = match graph_type {
                TCGraphType::Prepend => &self.prepend_graph,
                TCGraphType::Normal => &self.normal_graph,
                TCGraphType::Append => &self.append_graph,
            };

            debug!(
                "{} graph: {:?} nodes and {:?} edges",
                graph_type.as_str(),
                graph.node_count(),
                graph.edge_count()
            );

            let mut topo = StableTopo::new(graph);
            while let Some(node_idx) = topo.next() {
                let file_node = match graph.node_weight(node_idx) {
                    Some(x) => x,
                    None => return Err(TopCatError::UnknownError("Node not found".to_string())),
                };
                trace!("{} node: {:?}", graph_type.as_str(), file_node.name);

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
