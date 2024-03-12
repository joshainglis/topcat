use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use log::{debug, error, info};
use petgraph::algo::is_cyclic_directed;
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;

use crate::exceptions::{FileNodeError, TopCatError};
use crate::file_node::FileNode;
use crate::io_utils;
use crate::stable_topo::StableTopo;

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

/// Represents a graph structure for a set of files and their dependencies.
pub struct TCGraph {
    pub comment_str: String,
    pub file_dirs: Vec<PathBuf>,
    pub exclude: Option<HashSet<PathBuf>>,
    pub include: Option<HashSet<PathBuf>>,
    normal_graph: DiGraph<FileNode, ()>,
    prepend_graph: DiGraph<FileNode, ()>,
    append_graph: DiGraph<FileNode, ()>,
    path_map: HashMap<PathBuf, FileNode>,
    name_map: HashMap<String, FileNode>,
    normal_index_map: HashMap<String, NodeIndex>,
    prepend_index_map: HashMap<String, NodeIndex>,
    append_index_map: HashMap<String, NodeIndex>,
    graph_is_built: bool,
}

impl TCGraph {
    pub fn new(
        comment_str: String,
        file_dirs: Vec<PathBuf>,
        exclude: Option<&[String]>,
        include: Option<&[String]>,
    ) -> TCGraph {
        let exclude = exclude.map(|patterns| io_utils::glob_files(patterns).unwrap_or_default());
        let include = include.map(|patterns| io_utils::glob_files(patterns).unwrap_or_default());
        TCGraph {
            comment_str,
            file_dirs,
            exclude,
            include,
            normal_graph: DiGraph::new(),
            prepend_graph: DiGraph::new(),
            append_graph: DiGraph::new(),
            path_map: HashMap::new(),
            name_map: HashMap::new(),
            normal_index_map: HashMap::new(),
            prepend_index_map: HashMap::new(),
            append_index_map: HashMap::new(),
            graph_is_built: false,
        }
    }
    pub fn build_graph(&mut self) -> Result<(), TopCatError> {
        let mut files: HashSet<PathBuf> = HashSet::new();

        for dir in self.file_dirs.iter() {
            for f in io_utils::walk_dir(&dir)? {
                files.insert(f);
            }
        }

        debug!("include: {:?}", self.include);
        debug!("exclude: {:?}", self.exclude);
        for file in files {
            let path = &file;
            if let Some(ref include) = self.include {
                if !include.is_empty() && include.contains(path) {
                    debug!("Excluding file as it isn't in the include set: {:?}", path);
                    continue;
                }
            }
            if let Some(ref exclude) = self.exclude {
                if !exclude.is_empty() && exclude.contains(path) {
                    debug!("Excluding file as it is in the exclude set: {:?}", path);
                    continue;
                }
            }
            let file_node = match FileNode::from_file(&self.comment_str, &path) {
                Ok(f) => f,
                Err(e) => {
                    match e {
                        FileNodeError::NoNameDefined(p) => {
                            info!("Ignoring {:?}: No name defined in file header", p);
                        }
                        FileNodeError::InvalidPath(p) => {
                            error!("Ignoring {:?}: Invalid path", p);
                        }
                        FileNodeError::TooManyNames(p, s) => {
                            return Err(TopCatError::InvalidFileHeader(
                                p,
                                format!("Too many names declared: {}", s.join(", ")),
                            ));
                        }
                    }
                    continue;
                }
            };
            if self.name_map.contains_key(&file_node.name) {
                let other_path = match self.name_map.get(&file_node.name) {
                    Some(f) => f.path.clone(),
                    None => {
                        return Err(TopCatError::UnknownError(format!(
                            "FileNode with name {} not found",
                            file_node.name
                        )))
                    }
                };
                return Err(TopCatError::NameClash(
                    file_node.name,
                    file_node.path,
                    other_path,
                ));
            }
            self.name_map
                .insert(file_node.name.clone(), file_node.clone());
            self.path_map
                .insert(file_node.path.clone(), file_node.clone());
        }

        for file_node in self.name_map.values() {
            let idx: NodeIndex;
            if file_node.prepend {
                idx = self.prepend_graph.add_node((file_node).clone());
                self.prepend_index_map.insert(file_node.name.clone(), idx);
            } else if file_node.append {
                idx = self.append_graph.add_node((file_node).clone());
                self.append_index_map.insert(file_node.name.clone(), idx);
            } else {
                idx = self.normal_graph.add_node((file_node).clone());
                self.normal_index_map.insert(file_node.name.clone(), idx);
            }
        }

        for file_node in self.name_map.values() {
            for ensure in &file_node.ensure_exists {
                if !self.name_map.contains_key(ensure) {
                    return Err(TopCatError::MissingExist(
                        file_node.name.clone(),
                        ensure.clone(),
                    ));
                }
            }

            for dep in &file_node.deps {
                if !self.name_map.contains_key(dep) {
                    return Err(TopCatError::MissingDependency(
                        file_node.name.clone(),
                        dep.clone(),
                    ));
                }
                let dep_node = match self.name_map.get(dep) {
                    Some(x) => x,
                    None => {
                        return Err(TopCatError::UnknownError(format!(
                            "FileNode with name {} not found",
                            dep
                        )))
                    }
                };
                if file_node.prepend {
                    if !dep_node.prepend {
                        return Err(TopCatError::InvalidDependency(
                            file_node.name.clone(),
                            dep.clone(),
                        ));
                    }
                    self.prepend_graph.add_edge(
                        match self.prepend_index_map.get(dep) {
                            Some(x) => x,
                            None => {
                                return Err(TopCatError::UnknownError(format!(
                                    "FileNode with name {} not found",
                                    dep
                                )))
                            }
                        }
                        .clone(),
                        match self.prepend_index_map.get(&file_node.name) {
                            Some(x) => x,
                            None => {
                                return Err(TopCatError::UnknownError(format!(
                                    "FileNode with name {} not found",
                                    file_node.name
                                )))
                            }
                        }
                        .clone(),
                        (),
                    );
                } else if file_node.append {
                    if dep_node.append {
                        self.append_graph.add_edge(
                            match self.append_index_map.get(dep) {
                                Some(x) => x,
                                None => {
                                    return Err(TopCatError::UnknownError(format!(
                                        "FileNode with name {} not found",
                                        dep
                                    )))
                                }
                            }
                            .clone(),
                            match self.append_index_map.get(&file_node.name) {
                                Some(x) => x,
                                None => {
                                    return Err(TopCatError::UnknownError(format!(
                                        "FileNode with name {} not found",
                                        file_node.name
                                    )))
                                }
                            }
                            .clone(),
                            (),
                        );
                    }
                } else {
                    if dep_node.append {
                        return Err(TopCatError::InvalidDependency(
                            file_node.name.clone(),
                            dep.clone(),
                        ));
                    } else if dep_node.prepend {
                        continue;
                    }
                    self.normal_graph.add_edge(
                        match self.normal_index_map.get(dep) {
                            Some(x) => x,
                            None => {
                                return Err(TopCatError::UnknownError(format!(
                                    "FileNode with name {} not found",
                                    dep
                                )))
                            }
                        }
                        .clone(),
                        match self.normal_index_map.get(&file_node.name) {
                            Some(x) => x,
                            None => {
                                return Err(TopCatError::UnknownError(format!(
                                    "FileNode with name {} not found",
                                    file_node.name
                                )))
                            }
                        }
                        .clone(),
                        (),
                    );
                }
            }
        }

        if is_cyclic_directed(&self.normal_graph) {
            return Err(TopCatError::CyclicDependency(
                "dependency graph".to_string(),
            ));
        }
        if is_cyclic_directed(&self.prepend_graph) {
            return Err(TopCatError::CyclicDependency("prepend graph".to_string()));
        }
        if is_cyclic_directed(&self.append_graph) {
            return Err(TopCatError::CyclicDependency("append graph".to_string()));
        }

        self.graph_is_built = true;
        Ok(())
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
        debug!(
            "Prepend graph: {:?} nodes and {:?} edges",
            self.prepend_graph.node_count(),
            self.prepend_graph.edge_count()
        );
        debug!(
            "Normal graph: {:?} nodes and {:?} edges",
            self.normal_graph.node_count(),
            self.normal_graph.edge_count()
        );
        debug!(
            "Append graph: {:?} nodes and {:?} edges",
            self.append_graph.node_count(),
            self.append_graph.edge_count()
        );
        let mut normal_topo = StableTopo::new(&self.normal_graph);
        let mut prepend_topo = StableTopo::new(&self.prepend_graph);
        let mut append_topo = StableTopo::new(&self.append_graph);
        let mut sorted_files = Vec::new();
        while let Some(node) = prepend_topo.next() {
            let file_node = match self.prepend_graph.node_weight(node) {
                Some(x) => x,
                None => return Err(TopCatError::UnknownError("Node not found".to_string())),
            };
            debug!("Prepend node: {:?}", file_node.name);
            sorted_files.push(file_node.path.clone());
        }
        while let Some(node) = normal_topo.next() {
            let file_node = match self.normal_graph.node_weight(node) {
                Some(x) => x,
                None => return Err(TopCatError::UnknownError("Node not found".to_string())),
            };
            debug!("Normal node: {:?}", file_node.name);
            sorted_files.push(file_node.path.clone());
        }
        while let Some(node) = append_topo.next() {
            let file_node = match self.append_graph.node_weight(node) {
                Some(x) => x,
                None => return Err(TopCatError::UnknownError("Node not found".to_string())),
            };
            debug!("Append node: {:?}", file_node.name);
            sorted_files.push(file_node.path.clone());
        }
        Ok(sorted_files)
    }
}
