//! Export commands for generating graph representations.
//!
//! This module provides commands for exporting the dependency graph
//! in various formats for visualization and analysis.
//!
//! # Available Formats
//!
//! - **json**: Export as JSON with full metadata
//! - **dot**: Export as DOT format for GraphViz
//! - **graphml**: Export as GraphML for Gephi/yEd
//! - **mermaid**: Export as Mermaid diagram
//!
//! # Export Modes
//!
//! - **full**: Export the entire graph (default)
//! - **deps**: Export a node and all its transitive dependencies
//! - **dependents**: Export a node and all its transitive dependents
//! - **direct**: Export a node and its direct neighbors only
//!
//! # Examples
//!
//! ```bash
//! # Export full graph as JSON
//! topcat export -i sql/ -e sql graph.json json
//!
//! # Export dependencies of a specific node
//! topcat export -i sql/ -e sql --mode deps --node my_schema.my_table deps.dot dot
//!
//! # Export with schema filtering
//! topcat export -i sql/ -e sql --schema auth auth-graph.json json
//! ```

use clap::{Args, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::path::PathBuf;

use topcat::cli::{FilterArgs, GlobalArgs, GraphInputArgs};
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::settings::Settings;

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

/// Command-line arguments for the export subcommand.
///
/// The export command uses:
/// - GlobalArgs: config, verbose, quiet
/// - GraphInputArgs: input directories, file filtering, layers
/// - FilterArgs: schema filtering (node prefixes not typically used for export)
#[derive(Debug, Args)]
pub struct ExportArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(flatten)]
    pub input: GraphInputArgs,

    #[command(flatten)]
    pub filter: FilterArgs,

    /// Output file path (positional argument)
    #[arg(value_name = "OUTPUT")]
    output: PathBuf,

    /// Export mode: full, deps, dependents, or direct
    #[arg(long = "mode", default_value = "full")]
    mode: ExportMode,

    /// Node name for filtered exports (required for deps, dependents, direct modes)
    #[arg(long = "node")]
    node: Option<String>,

    #[command(subcommand)]
    command: ExportCommand,
}

impl ExportArgs {
    /// Executes the export command with the configured parameters.
    ///
    /// This method orchestrates the entire export process:
    /// 1. Loads configuration from all sources
    /// 2. Builds the dependency graph from input files
    /// 3. Applies schema filtering if schemas are specified
    /// 4. Applies export mode filtering (full, deps, dependents, direct)
    /// 5. Exports to the requested format (JSON, DOT, GraphML, Mermaid)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration loading/validation fails
    /// - Graph construction fails (invalid files, missing dependencies, etc.)
    /// - Schema filtering finds no matching nodes
    /// - Export mode requires a node but none is provided
    /// - File writing fails (I/O errors, permission issues, etc.)
    /// - Serialization fails (JSON or XML generation errors)
    pub fn execute(&self) -> Result<(), TopCatError> {
        // 1-2. Load settings and apply CLI overrides
        let settings =
            cmd_common::load_settings_with_overrides(self.global.config_path(), |settings| {
                self.global.apply_to_settings(settings);
                self.input.apply_to_settings(settings);
                self.filter.apply_to_settings(settings);
            })?;

        // 3-4. Validate settings
        cmd_common::validate_settings(&settings, true)?;

        // 5. Initialize logging
        let logger = cmd_common::init_logger_from_settings(&settings);

        // 6. Extract schema filter
        let schema_filter = self.filter.get_schemas(&settings);

        // 7. Build the graph
        let mut graph = self.build_graph(&settings)?;

        // 8. Apply schema filtering if requested
        if !schema_filter.is_empty() {
            graph = self.filter_by_schemas(&graph, &schema_filter)?;
        }

        // 9. Apply export mode filtering
        let graph = self.apply_export_mode(&graph)?;

        // 10. Execute the export command
        match &self.command {
            ExportCommand::Json => json::export_json(&graph, &self.output, &logger),
            ExportCommand::Dot => dot::export_dot(&graph, &self.output, &logger),
            ExportCommand::Graphml => graphml::export_graphml(&graph, &self.output, &logger),
            ExportCommand::Mermaid => mermaid::export_mermaid(&graph, &self.output, &logger),
        }
    }

    /// Builds the dependency graph from Settings.
    ///
    /// Delegates to the common implementation for graph building from Settings.
    /// Export command does not use schema filtering at the node level (filtering
    /// is done after the graph is built via filter_by_schemas method).
    fn build_graph(&self, settings: &Settings) -> Result<TCGraph, TopCatError> {
        cmd_common::build_graph_from_settings(&None, settings)
    }

    /// Filters the graph to include only nodes from the specified schemas.
    ///
    /// # Errors
    ///
    /// Returns an error if no nodes are found for the specified schemas.
    fn filter_by_schemas(
        &self,
        graph: &TCGraph,
        schema_filter: &[String],
    ) -> Result<TCGraph, TopCatError> {
        let mut filtered_nodes = HashSet::new();
        let schemas = graph.get_schemas();

        for schema_name in schema_filter {
            if let Some(nodes) = schemas.get(schema_name) {
                filtered_nodes.extend(nodes.iter().map(|n| n.name.clone()));
            }
        }

        if filtered_nodes.is_empty() {
            return Err(TopCatError::ConfigError(format!(
                "No nodes found for schemas: {schema_filter:?}"
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
