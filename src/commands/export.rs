use chrono::Utc;
use clap::{Args, Subcommand, ValueEnum};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use topcat::config::Config;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::file_node::FileNode;

// Constants for export formatting
const SCHEMA_COLORS: &[&str] = &[
    "lightblue",
    "lightgreen",
    "lightyellow",
    "lightpink",
    "lightcyan",
    "lavender",
];

const NODE_TYPE_ROOT: &str = "root";
const NODE_TYPE_LEAF: &str = "leaf";
const NODE_TYPE_INTERMEDIATE: &str = "intermediate";
const EDGE_TYPE_REQUIRES: &str = "requires";

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

// Helper functions for export operations

/// Writes content to a file, handling I/O errors appropriately.
fn write_output_file(path: &PathBuf, content: &str) -> Result<(), TopCatError> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

/// Groups nodes by their schema, returning a map of schema -> nodes.
fn group_nodes_by_schema<'a>(nodes: &'a [&FileNode]) -> HashMap<Option<String>, Vec<&'a FileNode>> {
    let mut schema_nodes: HashMap<Option<String>, Vec<&'a FileNode>> = HashMap::new();
    for node in nodes {
        schema_nodes
            .entry(node.schema.clone())
            .or_default()
            .push(node);
    }
    schema_nodes
}

/// Sanitizes schema names for use in GraphViz/Mermaid identifiers.
fn sanitize_schema_name(name: &str) -> String {
    name.replace('.', "_").replace("::", "_")
}

/// Checks if an edge crosses schema boundaries.
fn is_cross_schema_edge(
    source_node: &FileNode,
    target_name: &str,
    node_map: &HashMap<String, &FileNode>,
) -> bool {
    match (source_node.schema.as_ref(), node_map.get(target_name)) {
        (Some(src_schema), Some(target_node)) => Some(src_schema) != target_node.schema.as_ref(),
        _ => false, // If target is missing, don't mark as cross-schema
    }
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
            ExportCommand::Json => self.export_json(&graph),
            ExportCommand::Dot => self.export_dot(&graph),
            ExportCommand::Graphml => self.export_graphml(&graph),
            ExportCommand::Mermaid => self.export_mermaid(&graph),
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
        let layers = if let Some(ref layers_str) = self.layers {
            layers_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        } else {
            vec![
                "prepend".to_string(),
                "normal".to_string(),
                "append".to_string(),
            ]
        };

        let fallback_layer = self.fallback_layer.clone();

        if !layers.contains(&fallback_layer) {
            return Err(TopCatError::ConfigError(format!(
                "Fallback layer '{fallback_layer}' is not in the layers list: {layers:?}"
            )));
        }

        let config = Config {
            input_dirs: self.input_dirs.clone(),
            include_extensions: if self.include_extensions.is_empty() {
                None
            } else {
                Some(&self.include_extensions)
            },
            include_globs: None,
            exclude_globs: None,
            exclude_extensions: None,
            output: PathBuf::new(), // Not used for export - dummy value
            comment_str: self.comment_str.clone(),
            file_separator_str: String::new(),
            file_end_str: String::new(),
            verbose: false,
            dry_run: false,
            include_node_prefixes: None,
            exclude_node_prefixes: None,
            include_hidden: false,
            subdir_filter: None,
            layers,
            fallback_layer,
            sql_discovery: Default::default(),
            header_update_mode: Default::default(),
            header_output_dir: None,
        };

        let mut graph = TCGraph::new(&config);
        graph.build_graph()?;
        Ok(graph)
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

    /// Exports the graph to JSON format with full metadata.
    ///
    /// The JSON export includes:
    /// - Metadata (version, timestamp, counts)
    /// - Schema information (node counts, dependency counts)
    /// - Node details (name, path, layer, schema, dependencies, dependents, type)
    /// - Edge list (source, target, type)
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization or file writing fails.
    fn export_json(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let export_data = JsonExport::from_graph(graph);
        let json = serde_json::to_string_pretty(&export_data).map_err(|e| {
            TopCatError::SerializationError(format!("JSON serialization failed: {e}"))
        })?;

        write_output_file(&self.output, &json)?;

        println!("Exported JSON to: {}", self.output.display());
        Ok(())
    }

    /// Renders a single DOT schema subgraph with colored nodes.
    fn render_dot_schema_subgraph(
        output: &mut String,
        schema_name: &str,
        nodes: &[&FileNode],
        color: &str,
    ) {
        output.push_str(&format!(
            "    subgraph cluster_{} {{\n",
            sanitize_schema_name(schema_name)
        ));
        output.push_str(&format!("        label=\"{schema_name}\";\n"));
        output.push_str("        style=filled;\n");
        output.push_str(&format!("        color={color};\n"));
        output.push_str(&format!("        fillcolor={color};\n\n"));

        for node in nodes {
            output.push_str(&format!("        \"{}\";\n", node.name));
        }

        output.push_str("    }\n\n");
    }

    /// Renders DOT edges, marking cross-schema dependencies with dashed red lines.
    fn render_dot_edges(
        output: &mut String,
        all_nodes: &[FileNode],
        node_map: &HashMap<String, &FileNode>,
    ) {
        output.push_str("    // Dependencies\n");
        for node in all_nodes {
            for dep in &node.deps {
                if is_cross_schema_edge(node, dep, node_map) {
                    output.push_str(&format!(
                        "    \"{}\" -> \"{}\" [color=red, style=dashed];\n",
                        node.name, dep
                    ));
                } else {
                    output.push_str(&format!("    \"{}\" -> \"{}\";\n", node.name, dep));
                }
            }
        }
    }

    /// Exports the graph to DOT format for GraphViz visualization.
    ///
    /// Generates a directed graph in DOT format with:
    /// - Nodes grouped by schema (with colored subgraphs)
    /// - Cross-schema edges shown as red dashed lines
    /// - Left-to-right layout (rankdir=LR)
    ///
    /// The output can be visualized using GraphViz tools like `dot`:
    /// ```bash
    /// dot -Tpng graph.dot -o graph.png
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if file writing fails.
    fn export_dot(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let mut output = String::new();
        output.push_str("digraph dependencies {\n");
        output.push_str("    rankdir=LR;\n");
        output.push_str("    node [shape=box, style=filled];\n\n");

        // Get all nodes and build lookup map
        let all_nodes = graph.get_all_nodes();
        let node_refs: Vec<&FileNode> = all_nodes.iter().collect();
        let node_map: HashMap<String, &FileNode> =
            all_nodes.iter().map(|n| (n.name.clone(), n)).collect();

        // Group nodes by schema
        let schema_nodes = group_nodes_by_schema(&node_refs);

        // Render schema subgraphs
        let mut color_idx = 0;
        for (schema, nodes) in schema_nodes.iter() {
            if let Some(schema_name) = schema {
                let color = SCHEMA_COLORS[color_idx % SCHEMA_COLORS.len()];
                color_idx += 1;
                Self::render_dot_schema_subgraph(&mut output, schema_name, nodes, color);
            } else {
                // Nodes without schema
                for node in nodes {
                    output.push_str(&format!("    \"{}\" [fillcolor=white];\n", node.name));
                }
                output.push('\n');
            }
        }

        // Render edges
        Self::render_dot_edges(&mut output, &all_nodes, &node_map);

        output.push_str("}\n");
        write_output_file(&self.output, &output)?;

        println!("Exported DOT to: {}", self.output.display());
        Ok(())
    }

    /// Writes the GraphML header (XML declaration and root element).
    fn write_graphml_header(writer: &mut Writer<BufWriter<File>>) -> Result<(), TopCatError> {
        // Write XML declaration
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;

        // Write graphml root element
        let mut graphml = BytesStart::new("graphml");
        graphml.push_attribute(("xmlns", "http://graphml.graphdrawing.org/xmlns"));
        graphml.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
        graphml.push_attribute((
            "xsi:schemaLocation",
            "http://graphml.graphdrawing.org/xmlns http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd",
        ));
        writer
            .write_event(Event::Start(graphml))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;

        Ok(())
    }

    /// Writes GraphML key definitions for node and edge attributes.
    fn write_graphml_keys(writer: &mut Writer<BufWriter<File>>) -> Result<(), TopCatError> {
        for (id, for_type, name, attr_type) in [
            ("d0", "node", "schema", "string"),
            ("d1", "node", "layer", "string"),
            ("d2", "node", "path", "string"),
            ("d3", "edge", "type", "string"),
        ] {
            let mut key = BytesStart::new("key");
            key.push_attribute(("id", id));
            key.push_attribute(("for", for_type));
            key.push_attribute(("attr.name", name));
            key.push_attribute(("attr.type", attr_type));
            writer
                .write_event(Event::Empty(key))
                .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;
        }
        Ok(())
    }

    /// Writes a single GraphML node with all its data attributes.
    fn write_graphml_node(
        writer: &mut Writer<BufWriter<File>>,
        node: &FileNode,
        node_id: usize,
    ) -> Result<(), TopCatError> {
        let mut node_elem = BytesStart::new("node");
        node_elem.push_attribute(("id", format!("n{node_id}").as_str()));
        writer
            .write_event(Event::Start(node_elem.clone()))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;

        // Write schema data if present
        if let Some(ref schema) = node.schema {
            let mut data = BytesStart::new("data");
            data.push_attribute(("key", "d0"));
            writer
                .write_event(Event::Start(data.clone()))
                .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::Text(BytesText::new(schema)))
                .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;
            writer
                .write_event(Event::End(BytesEnd::new("data")))
                .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;
        }

        // Write layer data
        let mut data = BytesStart::new("data");
        data.push_attribute(("key", "d1"));
        writer
            .write_event(Event::Start(data.clone()))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;
        writer
            .write_event(Event::Text(BytesText::new(&node.layer)))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;
        writer
            .write_event(Event::End(BytesEnd::new("data")))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;

        // Write path data
        let mut data = BytesStart::new("data");
        data.push_attribute(("key", "d2"));
        writer
            .write_event(Event::Start(data.clone()))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;
        writer
            .write_event(Event::Text(BytesText::new(
                &node.path.display().to_string(),
            )))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;
        writer
            .write_event(Event::End(BytesEnd::new("data")))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;

        writer
            .write_event(Event::End(BytesEnd::new("node")))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;

        Ok(())
    }

    /// Writes all GraphML edges for the graph.
    fn write_graphml_edges(
        writer: &mut Writer<BufWriter<File>>,
        all_nodes: &[FileNode],
        node_id_map: &HashMap<String, usize>,
    ) -> Result<(), TopCatError> {
        let mut edge_id = 0;
        for node in all_nodes {
            let source_id = node_id_map
                .get(&node.name)
                .expect("node should exist in node_id_map");

            for dep in &node.deps {
                if let Some(target_id) = node_id_map.get(dep) {
                    let mut edge = BytesStart::new("edge");
                    edge.push_attribute(("id", format!("e{edge_id}").as_str()));
                    edge.push_attribute(("source", format!("n{source_id}").as_str()));
                    edge.push_attribute(("target", format!("n{target_id}").as_str()));
                    writer
                        .write_event(Event::Start(edge.clone()))
                        .map_err(|e| {
                            TopCatError::SerializationError(format!("XML write error: {e}"))
                        })?;

                    // Write edge type data
                    let mut data = BytesStart::new("data");
                    data.push_attribute(("key", "d3"));
                    writer
                        .write_event(Event::Start(data.clone()))
                        .map_err(|e| {
                            TopCatError::SerializationError(format!("XML write error: {e}"))
                        })?;
                    writer
                        .write_event(Event::Text(BytesText::new(EDGE_TYPE_REQUIRES)))
                        .map_err(|e| {
                            TopCatError::SerializationError(format!("XML write error: {e}"))
                        })?;
                    writer
                        .write_event(Event::End(BytesEnd::new("data")))
                        .map_err(|e| {
                            TopCatError::SerializationError(format!("XML write error: {e}"))
                        })?;

                    writer
                        .write_event(Event::End(BytesEnd::new("edge")))
                        .map_err(|e| {
                            TopCatError::SerializationError(format!("XML write error: {e}"))
                        })?;

                    edge_id += 1;
                }
            }
        }
        Ok(())
    }

    /// Exports the graph to GraphML format for tools like Gephi or yEd.
    ///
    /// GraphML is an XML-based format that includes:
    /// - Node attributes: schema, layer, path
    /// - Edge attributes: type (e.g., "requires")
    /// - Full graph structure with metadata
    ///
    /// The output can be imported into graph analysis tools for visualization
    /// and further analysis.
    ///
    /// # Errors
    ///
    /// Returns an error if XML writing or file creation fails.
    fn export_graphml(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let file = File::create(&self.output)?;
        let buf_writer = BufWriter::new(file);
        let mut writer = Writer::new_with_indent(buf_writer, b' ', 2);

        // Write header and metadata
        Self::write_graphml_header(&mut writer)?;
        Self::write_graphml_keys(&mut writer)?;

        // Write graph element
        let mut graph_elem = BytesStart::new("graph");
        graph_elem.push_attribute(("id", "G"));
        graph_elem.push_attribute(("edgedefault", "directed"));
        writer
            .write_event(Event::Start(graph_elem))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;

        // Get all nodes and build lookup map
        let all_nodes = graph.get_all_nodes();
        let node_id_map: HashMap<String, usize> = all_nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.name.clone(), i))
            .collect();

        // Write all nodes
        for (i, node) in all_nodes.iter().enumerate() {
            Self::write_graphml_node(&mut writer, node, i)?;
        }

        // Write all edges
        Self::write_graphml_edges(&mut writer, &all_nodes, &node_id_map)?;

        // Close graph and graphml elements
        writer
            .write_event(Event::End(BytesEnd::new("graph")))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;
        writer
            .write_event(Event::End(BytesEnd::new("graphml")))
            .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;

        println!("Exported GraphML to: {}", self.output.display());
        Ok(())
    }

    /// Renders a single Mermaid schema subgraph.
    fn render_mermaid_schema_subgraph(
        output: &mut String,
        schema_name: &str,
        nodes: &[&FileNode],
        node_id_map: &HashMap<String, String>,
    ) {
        output.push_str(&format!(
            "    subgraph {}\n",
            sanitize_schema_name(schema_name)
        ));

        for node in nodes {
            let node_id = node_id_map
                .get(&node.name)
                .expect("node should exist in node_id_map");
            output.push_str(&format!("        {}[\"{}\"]\n", node_id, node.name));
        }

        output.push_str("    end\n");
    }

    /// Renders Mermaid edges, marking cross-schema dependencies with dotted lines.
    fn render_mermaid_edges(
        output: &mut String,
        all_nodes: &[FileNode],
        node_id_map: &HashMap<String, String>,
        node_map: &HashMap<String, &FileNode>,
    ) {
        for node in all_nodes {
            let source_id = node_id_map
                .get(&node.name)
                .expect("node should exist in node_id_map");

            for dep in &node.deps {
                if let Some(target_id) = node_id_map.get(dep) {
                    if is_cross_schema_edge(node, dep, node_map) {
                        output.push_str(&format!("    {source_id} -.-> {target_id}\n"));
                    } else {
                        output.push_str(&format!("    {source_id} --> {target_id}\n"));
                    }
                }
            }
        }
    }

    /// Exports the graph to Mermaid diagram format.
    ///
    /// Generates a Mermaid flowchart with:
    /// - Nodes grouped by schema (using subgraphs)
    /// - Cross-schema edges shown as dotted lines (-.->)
    /// - Top-down layout
    ///
    /// The output can be rendered in Markdown viewers that support Mermaid,
    /// or using the Mermaid CLI:
    /// ```bash
    /// mmdc -i graph.md -o graph.png
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if file writing fails.
    fn export_mermaid(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let mut output = String::new();
        output.push_str("```mermaid\n");
        output.push_str("graph TD\n");

        // Get all nodes and build lookup maps
        let all_nodes = graph.get_all_nodes();
        let node_refs: Vec<&FileNode> = all_nodes.iter().collect();
        let node_map: HashMap<String, &FileNode> =
            all_nodes.iter().map(|n| (n.name.clone(), n)).collect();

        // Group nodes by schema
        let schema_nodes = group_nodes_by_schema(&node_refs);

        // Create node ID mapping (mermaid needs simple IDs)
        let node_id_map: HashMap<String, String> = all_nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.name.clone(), format!("N{i}")))
            .collect();

        // Render schema subgraphs
        for (schema, nodes) in schema_nodes.iter() {
            if let Some(schema_name) = schema {
                Self::render_mermaid_schema_subgraph(&mut output, schema_name, nodes, &node_id_map);
            } else {
                // Nodes without schema
                for node in nodes {
                    let node_id = node_id_map
                        .get(&node.name)
                        .expect("node should exist in node_id_map");
                    output.push_str(&format!("    {}[\"{}\"]\n", node_id, node.name));
                }
            }
        }

        output.push('\n');

        // Render edges
        Self::render_mermaid_edges(&mut output, &all_nodes, &node_id_map, &node_map);

        output.push_str("```\n");
        write_output_file(&self.output, &output)?;

        println!("Exported Mermaid diagram to: {}", self.output.display());
        Ok(())
    }
}

// JSON export data structures

/// Top-level JSON export structure containing all graph data.
#[derive(Debug, Serialize, Deserialize)]
struct JsonExport {
    /// Export metadata (version, timestamp, counts)
    metadata: MetadataExport,
    /// Schema information and statistics
    schemas: Vec<SchemaExport>,
    /// Node details with dependencies and attributes
    nodes: Vec<NodeExport>,
    /// Edge list representing dependencies
    edges: Vec<EdgeExport>,
}

/// Metadata about the export and graph statistics.
#[derive(Debug, Serialize, Deserialize)]
struct MetadataExport {
    /// Topcat version that generated this export
    version: String,
    /// ISO 8601 timestamp of export generation
    generated_at: String,
    /// Total number of nodes in the graph
    node_count: usize,
    /// Total number of edges in the graph
    edge_count: usize,
    /// Number of distinct schemas
    schema_count: usize,
}

/// Schema-level statistics and metadata.
#[derive(Debug, Serialize, Deserialize)]
struct SchemaExport {
    /// Schema name
    name: String,
    /// Number of nodes in this schema
    node_count: usize,
    /// Number of dependencies within this schema
    internal_deps: usize,
    /// Number of dependencies to other schemas
    external_deps: usize,
}

/// Individual node with all its attributes and relationships.
#[derive(Debug, Serialize, Deserialize)]
struct NodeExport {
    /// Unique node name
    name: String,
    /// File path
    path: String,
    /// Layer assignment (e.g., "prepend", "normal", "append")
    layer: String,
    /// Schema this node belongs to (if any)
    schema: Option<String>,
    /// List of node names this node depends on
    dependencies: Vec<String>,
    /// List of node names that depend on this node
    dependents: Vec<String>,
    /// Node classification: "root", "leaf", or "intermediate"
    node_type: String,
}

/// Directed edge in the dependency graph.
#[derive(Debug, Serialize, Deserialize)]
struct EdgeExport {
    /// Source node name
    source: String,
    /// Target node name
    target: String,
    /// Edge type (e.g., "requires")
    edge_type: String,
}

impl JsonExport {
    fn from_graph(graph: &TCGraph) -> Self {
        let all_nodes = graph.get_all_nodes();
        let schemas = graph.get_schemas();

        // Count edges
        let edge_count: usize = all_nodes.iter().map(|n| n.deps.len()).sum();

        // Build dependents map
        let dependents_map = graph.build_dependents_map();

        // Export schemas
        let schema_exports: Vec<SchemaExport> = schemas
            .iter()
            .map(|(name, nodes)| {
                let internal_deps = graph.get_internal_dependencies(name).len();
                let external_deps = graph.get_external_dependencies(name).len();
                SchemaExport {
                    name: name.clone(),
                    node_count: nodes.len(),
                    internal_deps,
                    external_deps,
                }
            })
            .collect();

        // Export nodes
        let nodes: Vec<NodeExport> = all_nodes
            .iter()
            .map(|node| {
                let dependencies: Vec<String> = node.deps.iter().cloned().collect();
                let dependents: Vec<String> = dependents_map
                    .get(&node.name)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect();

                let node_type = if dependents.is_empty() {
                    NODE_TYPE_LEAF
                } else if dependencies.is_empty() {
                    NODE_TYPE_ROOT
                } else {
                    NODE_TYPE_INTERMEDIATE
                };

                NodeExport {
                    name: node.name.clone(),
                    path: node.path.display().to_string(),
                    layer: node.layer.clone(),
                    schema: node.schema.clone(),
                    dependencies,
                    dependents,
                    node_type: node_type.to_string(),
                }
            })
            .collect();

        // Export edges
        let edges: Vec<EdgeExport> = all_nodes
            .iter()
            .flat_map(|node| {
                node.deps
                    .iter()
                    .map(|dep| EdgeExport {
                        source: node.name.clone(),
                        target: dep.clone(),
                        edge_type: EDGE_TYPE_REQUIRES.to_string(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        JsonExport {
            metadata: MetadataExport {
                version: env!("CARGO_PKG_VERSION").to_string(),
                generated_at: Utc::now().to_rfc3339(),
                node_count: all_nodes.len(),
                edge_count,
                schema_count: schemas.len(),
            },
            schemas: schema_exports,
            nodes,
            edges,
        }
    }
}
