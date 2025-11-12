use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::file_node::FileNode;

use super::common::EDGE_TYPE_REQUIRES;

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
pub fn export_graphml(graph: &TCGraph, output_path: &PathBuf) -> Result<(), TopCatError> {
    let file = File::create(output_path)?;
    let buf_writer = BufWriter::new(file);
    let mut writer = Writer::new_with_indent(buf_writer, b' ', 2);

    // Write header and metadata
    write_graphml_header(&mut writer)?;
    write_graphml_keys(&mut writer)?;

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
        write_graphml_node(&mut writer, node, i)?;
    }

    // Write all edges
    write_graphml_edges(&mut writer, &all_nodes, &node_id_map)?;

    // Close graph and graphml elements
    writer
        .write_event(Event::End(BytesEnd::new("graph")))
        .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;
    writer
        .write_event(Event::End(BytesEnd::new("graphml")))
        .map_err(|e| TopCatError::SerializationError(format!("XML write error: {e}")))?;

    println!("Exported GraphML to: {}", output_path.display());
    Ok(())
}
