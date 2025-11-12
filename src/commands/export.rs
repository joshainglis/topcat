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
            output: PathBuf::from("/dev/null"), // Not used for export
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

    fn require_node(&self) -> Result<String, TopCatError> {
        self.node.clone().ok_or_else(|| {
            TopCatError::ConfigError(
                "The --node argument is required for deps, dependents, and direct export modes"
                    .to_string(),
            )
        })
    }

    fn export_json(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let export_data = JsonExport::from_graph(graph);
        let json = serde_json::to_string_pretty(&export_data)
            .map_err(|e| TopCatError::UnknownError(format!("Failed to serialize JSON: {e}")))?;

        let mut file = File::create(&self.output)
            .map_err(|e| TopCatError::UnknownError(format!("Failed to create output file: {e}")))?;

        file.write_all(json.as_bytes())
            .map_err(|e| TopCatError::UnknownError(format!("Failed to write JSON output: {e}")))?;

        println!("Exported JSON to: {}", self.output.display());
        Ok(())
    }

    fn export_dot(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let mut output = String::new();
        output.push_str("digraph dependencies {\n");
        output.push_str("    rankdir=LR;\n");
        output.push_str("    node [shape=box, style=filled];\n\n");

        // Get all nodes and build lookup map
        let all_nodes = graph.get_all_nodes();
        let node_map: HashMap<String, &FileNode> =
            all_nodes.iter().map(|n| (n.name.clone(), n)).collect();

        // Group nodes by schema
        let mut schema_nodes: HashMap<Option<String>, Vec<&FileNode>> = HashMap::new();

        for node in &all_nodes {
            schema_nodes
                .entry(node.schema.clone())
                .or_default()
                .push(node);
        }

        // Define schema colors
        let schema_colors = [
            "lightblue",
            "lightgreen",
            "lightyellow",
            "lightpink",
            "lightcyan",
            "lavender",
        ];

        // Output nodes grouped by schema
        let mut color_idx = 0;
        for (schema, nodes) in schema_nodes.iter() {
            if let Some(schema_name) = schema {
                let color = schema_colors[color_idx % schema_colors.len()];
                color_idx += 1;

                output.push_str(&format!(
                    "    subgraph cluster_{} {{\n",
                    schema_name.replace('.', "_").replace("::", "_")
                ));
                output.push_str(&format!("        label=\"{schema_name}\";\n"));
                output.push_str("        style=filled;\n");
                output.push_str(&format!("        color={color};\n"));
                output.push_str(&format!("        fillcolor={color};\n\n"));

                for node in nodes {
                    output.push_str(&format!("        \"{}\";\n", node.name));
                }

                output.push_str("    }\n\n");
            } else {
                // Nodes without schema
                for node in nodes {
                    output.push_str(&format!("    \"{}\" [fillcolor=white];\n", node.name));
                }
                output.push('\n');
            }
        }

        // Output edges
        output.push_str("    // Dependencies\n");
        for node in &all_nodes {
            for dep in &node.deps {
                let is_cross_schema =
                    node.schema.as_ref() != node_map.get(dep).and_then(|n| n.schema.as_ref());

                if is_cross_schema {
                    output.push_str(&format!(
                        "    \"{}\" -> \"{}\" [color=red, style=dashed];\n",
                        node.name, dep
                    ));
                } else {
                    output.push_str(&format!("    \"{}\" -> \"{}\";\n", node.name, dep));
                }
            }
        }

        output.push_str("}\n");

        let mut file = File::create(&self.output)
            .map_err(|e| TopCatError::UnknownError(format!("Failed to create output file: {e}")))?;

        file.write_all(output.as_bytes())
            .map_err(|e| TopCatError::UnknownError(format!("Failed to write DOT output: {e}")))?;

        println!("Exported DOT to: {}", self.output.display());
        Ok(())
    }

    fn export_graphml(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let file = File::create(&self.output)
            .map_err(|e| TopCatError::UnknownError(format!("Failed to create output file: {e}")))?;

        let buf_writer = BufWriter::new(file);
        let mut writer = Writer::new_with_indent(buf_writer, b' ', 2);

        // Write XML declaration
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;

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
            .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;

        // Write key definitions for node/edge attributes
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
                .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;
        }

        // Write graph element
        let mut graph_elem = BytesStart::new("graph");
        graph_elem.push_attribute(("id", "G"));
        graph_elem.push_attribute(("edgedefault", "directed"));
        writer
            .write_event(Event::Start(graph_elem))
            .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;

        // Get all nodes and build lookup map
        let all_nodes = graph.get_all_nodes();
        let node_id_map: HashMap<String, usize> = all_nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.name.clone(), i))
            .collect();

        // Write nodes
        for (i, node) in all_nodes.iter().enumerate() {
            let mut node_elem = BytesStart::new("node");
            node_elem.push_attribute(("id", format!("n{i}").as_str()));
            writer
                .write_event(Event::Start(node_elem.clone()))
                .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;

            // Write schema data if present
            if let Some(ref schema) = node.schema {
                let mut data = BytesStart::new("data");
                data.push_attribute(("key", "d0"));
                writer
                    .write_event(Event::Start(data.clone()))
                    .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;
                writer
                    .write_event(Event::Text(BytesText::new(schema)))
                    .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;
                writer
                    .write_event(Event::End(BytesEnd::new("data")))
                    .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;
            }

            // Write layer data
            let mut data = BytesStart::new("data");
            data.push_attribute(("key", "d1"));
            writer
                .write_event(Event::Start(data.clone()))
                .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;
            writer
                .write_event(Event::Text(BytesText::new(&node.layer)))
                .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;
            writer
                .write_event(Event::End(BytesEnd::new("data")))
                .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;

            // Write path data
            let mut data = BytesStart::new("data");
            data.push_attribute(("key", "d2"));
            writer
                .write_event(Event::Start(data.clone()))
                .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;
            writer
                .write_event(Event::Text(BytesText::new(
                    &node.path.display().to_string(),
                )))
                .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;
            writer
                .write_event(Event::End(BytesEnd::new("data")))
                .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;

            writer
                .write_event(Event::End(BytesEnd::new("node")))
                .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;
        }

        // Write edges
        let mut edge_id = 0;
        for node in &all_nodes {
            let source_id = node_id_map.get(&node.name).unwrap();

            for dep in &node.deps {
                if let Some(target_id) = node_id_map.get(dep) {
                    let mut edge = BytesStart::new("edge");
                    edge.push_attribute(("id", format!("e{edge_id}").as_str()));
                    edge.push_attribute(("source", format!("n{source_id}").as_str()));
                    edge.push_attribute(("target", format!("n{target_id}").as_str()));
                    writer
                        .write_event(Event::Start(edge.clone()))
                        .map_err(|e| {
                            TopCatError::UnknownError(format!("Failed to write XML: {e}"))
                        })?;

                    // Write edge type data
                    let mut data = BytesStart::new("data");
                    data.push_attribute(("key", "d3"));
                    writer
                        .write_event(Event::Start(data.clone()))
                        .map_err(|e| {
                            TopCatError::UnknownError(format!("Failed to write XML: {e}"))
                        })?;
                    writer
                        .write_event(Event::Text(BytesText::new("requires")))
                        .map_err(|e| {
                            TopCatError::UnknownError(format!("Failed to write XML: {e}"))
                        })?;
                    writer
                        .write_event(Event::End(BytesEnd::new("data")))
                        .map_err(|e| {
                            TopCatError::UnknownError(format!("Failed to write XML: {e}"))
                        })?;

                    writer
                        .write_event(Event::End(BytesEnd::new("edge")))
                        .map_err(|e| {
                            TopCatError::UnknownError(format!("Failed to write XML: {e}"))
                        })?;

                    edge_id += 1;
                }
            }
        }

        // Close graph element
        writer
            .write_event(Event::End(BytesEnd::new("graph")))
            .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;

        // Close graphml element
        writer
            .write_event(Event::End(BytesEnd::new("graphml")))
            .map_err(|e| TopCatError::UnknownError(format!("Failed to write XML: {e}")))?;

        println!("Exported GraphML to: {}", self.output.display());
        Ok(())
    }

    fn export_mermaid(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let mut output = String::new();
        output.push_str("```mermaid\n");
        output.push_str("graph TD\n");

        // Get all nodes and build lookup map
        let all_nodes = graph.get_all_nodes();
        let node_map: HashMap<String, &FileNode> =
            all_nodes.iter().map(|n| (n.name.clone(), n)).collect();

        // Group nodes by schema
        let mut schema_nodes: HashMap<Option<String>, Vec<&FileNode>> = HashMap::new();
        for node in &all_nodes {
            schema_nodes
                .entry(node.schema.clone())
                .or_default()
                .push(node);
        }

        // Create node ID mapping (mermaid needs simple IDs)
        let node_id_map: HashMap<String, String> = all_nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.name.clone(), format!("N{i}")))
            .collect();

        // Output subgraphs by schema
        for (schema, nodes) in schema_nodes.iter() {
            if let Some(schema_name) = schema {
                output.push_str(&format!(
                    "    subgraph {}\n",
                    schema_name.replace('.', "_").replace("::", "_")
                ));

                for node in nodes {
                    let node_id = node_id_map.get(&node.name).unwrap();
                    output.push_str(&format!("        {}[\"{}\"]\n", node_id, node.name));
                }

                output.push_str("    end\n");
            } else {
                // Nodes without schema
                for node in nodes {
                    let node_id = node_id_map.get(&node.name).unwrap();
                    output.push_str(&format!("    {}[\"{}\"]\n", node_id, node.name));
                }
            }
        }

        output.push('\n');

        // Output edges
        for node in &all_nodes {
            let source_id = node_id_map.get(&node.name).unwrap();

            for dep in &node.deps {
                if let Some(target_id) = node_id_map.get(dep) {
                    let is_cross_schema =
                        node.schema.as_ref() != node_map.get(dep).and_then(|n| n.schema.as_ref());

                    if is_cross_schema {
                        output.push_str(&format!("    {source_id} -.-> {target_id}\n"));
                    } else {
                        output.push_str(&format!("    {source_id} --> {target_id}\n"));
                    }
                }
            }
        }

        output.push_str("```\n");

        let mut file = File::create(&self.output)
            .map_err(|e| TopCatError::UnknownError(format!("Failed to create output file: {e}")))?;

        file.write_all(output.as_bytes()).map_err(|e| {
            TopCatError::UnknownError(format!("Failed to write Mermaid output: {e}"))
        })?;

        println!("Exported Mermaid diagram to: {}", self.output.display());
        Ok(())
    }
}

// JSON export data structures
#[derive(Debug, Serialize, Deserialize)]
struct JsonExport {
    metadata: MetadataExport,
    schemas: Vec<SchemaExport>,
    nodes: Vec<NodeExport>,
    edges: Vec<EdgeExport>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetadataExport {
    version: String,
    generated_at: String,
    node_count: usize,
    edge_count: usize,
    schema_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct SchemaExport {
    name: String,
    node_count: usize,
    internal_deps: usize,
    external_deps: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeExport {
    name: String,
    path: String,
    layer: String,
    schema: Option<String>,
    dependencies: Vec<String>,
    dependents: Vec<String>,
    node_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EdgeExport {
    source: String,
    target: String,
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
                    "leaf"
                } else if dependencies.is_empty() {
                    "root"
                } else {
                    "intermediate"
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
                        edge_type: "requires".to_string(),
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
