use clap::{Args, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::path::PathBuf;

use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;

use super::common as cmd_common;

mod common;
mod dot;
mod graphml;
mod json;
mod mermaid;

#[derive(Debug, Clone, ValueEnum)]
pub enum ExportMode {
    /// Export the entire graph
    Full,
    /// Export a node and all its transitive dependencies
    Deps,
    /// Export a node and all its transitive dependents
    Dependents,
    /// Export a node and its direct neighbors only
    Direct,
}

#[derive(Debug, Subcommand)]
pub enum ExportCommand {
    /// Export as JSON with full metadata
    Json,
    /// Export as DOT format for GraphViz
    Dot,
    /// Export as GraphML for Gephi/yEd
    Graphml,
    /// Export as Mermaid diagram
    Mermaid,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Input directories containing files to analyze
    #[arg(short = 'i', long = "input-dirs", required = true)]
    input_dirs: Vec<PathBuf>,

    /// Output file path
    #[arg(short = 'o', long = "output", required = true)]
    output: PathBuf,

    /// File extensions to include (e.g., "sql")
    #[arg(short = 'e', long = "include-exts")]
    include_extensions: Vec<String>,

    /// Comment string used in file headers
    #[arg(short = 'c', long = "comment-str", default_value = "--")]
    comment_str: String,

    /// Custom layer ordering (comma-separated)
    #[arg(short = 'l', long = "layers")]
    layers: Option<String>,

    /// Fallback layer for files without explicit layer declaration
    #[arg(short = 'f', long = "fallback-layer", default_value = "normal")]
    fallback_layer: String,

    /// Export mode: full, deps, dependents, or direct
    #[arg(long = "mode", default_value = "full")]
    mode: ExportMode,

    /// Node name for filtered exports (required for deps, dependents, direct modes)
    #[arg(long = "node")]
    node: Option<String>,

    /// Filter by schema names (can be specified multiple times)
    #[arg(long = "schema")]
    schema: Vec<String>,

    #[command(subcommand)]
    command: ExportCommand,
}

impl ExportArgs {
    /// Executes the export command with the configured parameters.
    ///
    /// This method orchestrates the entire export process:
    /// 1. Builds the dependency graph from input files
    /// 2. Applies schema filtering if schemas are specified
    /// 3. Applies export mode filtering (full, deps, dependents, direct)
    /// 4. Exports to the requested format (JSON, DOT, GraphML, Mermaid)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Graph construction fails (invalid files, missing dependencies, etc.)
    /// - Schema filtering finds no matching nodes
    /// - Export mode requires a node but none is provided
    /// - File writing fails (I/O errors, permission issues, etc.)
    /// - Serialization fails (JSON or XML generation errors)
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Build the graph
        let mut graph = self.build_graph()?;

        // Apply schema filtering if requested
        if !self.schema.is_empty() {
            graph = self.filter_by_schemas(&graph)?;
        }

        // Apply export mode filtering
        let graph = self.apply_export_mode(&graph)?;

        // Execute the export command
        match &self.command {
            ExportCommand::Json => json::export_json(&graph, &self.output),
            ExportCommand::Dot => dot::export_dot(&graph, &self.output),
            ExportCommand::Graphml => graphml::export_graphml(&graph, &self.output),
            ExportCommand::Mermaid => mermaid::export_mermaid(&graph, &self.output),
        }
    }

    /// Builds the dependency graph from the configured input directories.
    ///
    /// Creates a `TCGraph` by scanning the input directories for files matching
    /// the specified extensions, parsing their dependency metadata, and constructing
    /// the DAG with layer-based ordering.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The fallback layer is not in the layers list
    /// - Graph construction fails (file reading, parsing, cycle detection)
    fn build_graph(&self) -> Result<TCGraph, TopCatError> {
        let (layers, fallback_layer) = cmd_common::parse_and_validate_layers(
            &None, // export doesn't have sql_config support yet
            &self.layers,
            &Some(self.fallback_layer.clone()),
        )?;

        cmd_common::build_graph(
            self.input_dirs.clone(),
            if self.include_extensions.is_empty() {
                None
            } else {
                Some(&self.include_extensions)
            },
            None,  // exclude_extensions
            None,  // include_globs
            None,  // exclude_globs
            false, // include_hidden
            false, // verbose
            self.comment_str.clone(),
            layers,
            fallback_layer,
            Default::default(), // sql_discovery
            None,               // schema_filter_prefixes
        )
    }

    /// Filters the graph to include only nodes from the specified schemas.
    ///
    /// # Errors
    ///
    /// Returns an error if no nodes are found for the specified schemas.
    fn filter_by_schemas(&self, graph: &TCGraph) -> Result<TCGraph, TopCatError> {
        let mut filtered_nodes = HashSet::new();
        let schemas = graph.get_schemas();

        for schema_name in &self.schema {
            if let Some(nodes) = schemas.get(schema_name) {
                filtered_nodes.extend(nodes.iter().map(|n| n.name.clone()));
            }
        }

        if filtered_nodes.is_empty() {
            return Err(TopCatError::ConfigError(format!(
                "No nodes found for schemas: {:?}",
                self.schema
            )));
        }

        graph.filter_nodes(&filtered_nodes)
    }

    /// Applies the export mode filter to the graph.
    ///
    /// Different modes include:
    /// - `Full`: All nodes in the graph
    /// - `Deps`: A node and all its transitive dependencies
    /// - `Dependents`: A node and all its transitive dependents
    /// - `Direct`: A node and its immediate neighbors (direct deps + dependents)
    ///
    /// # Errors
    ///
    /// Returns an error if the mode requires a node but none is provided.
    fn apply_export_mode(&self, graph: &TCGraph) -> Result<TCGraph, TopCatError> {
        match self.mode {
            ExportMode::Full => {
                // Return a filtered graph with all nodes
                let all_node_names: HashSet<String> = graph
                    .get_all_nodes()
                    .iter()
                    .map(|n| n.name.clone())
                    .collect();
                graph.filter_nodes(&all_node_names)
            }
            ExportMode::Deps => {
                let node = self.require_node()?;
                let deps = graph.get_transitive_dependencies(&node);
                let mut nodes = deps;
                nodes.insert(node);
                graph.filter_nodes(&nodes)
            }
            ExportMode::Dependents => {
                let node = self.require_node()?;
                let dependents = graph.get_transitive_dependents(&node);
                let mut nodes = dependents;
                nodes.insert(node);
                graph.filter_nodes(&nodes)
            }
            ExportMode::Direct => {
                let node = self.require_node()?;
                let (deps, dependents) = graph.get_direct_neighbors(&node);
                let mut nodes = deps;
                nodes.extend(dependents);
                nodes.insert(node);
                graph.filter_nodes(&nodes)
            }
        }
    }

    /// Ensures a node name is provided when required by the export mode.
    ///
    /// # Errors
    ///
    /// Returns an error if no node is specified for modes that require one.
    fn require_node(&self) -> Result<String, TopCatError> {
        self.node.clone().ok_or_else(|| {
            TopCatError::ConfigError(
                "The --node argument is required for deps, dependents, and direct export modes"
                    .to_string(),
            )
        })
    }
}
