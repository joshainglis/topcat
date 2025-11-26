//! Tests for FileNode parsing and functionality.

use super::*;

#[test]
fn test_new_layer_header_format() {
    let layers = vec![
        "first".to_string(),
        "second".to_string(),
        "third".to_string(),
    ];
    let fallback_layer = "second";

    // Create a temporary file with new layer format
    let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(&temp_file, "-- name: test_node\n-- layer: first\nSELECT 1;").unwrap();

    let file_node = FileNode::from_file(
        "--",
        &temp_file.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node.name, "test_node");
    assert_eq!(file_node.layer, "first");
}

#[test]
fn test_backward_compatibility_is_initial() {
    let layers = vec![
        "prepend".to_string(),
        "normal".to_string(),
        "append".to_string(),
    ];
    let fallback_layer = "normal";

    let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(&temp_file, "-- name: test_node\n-- is_initial\nSELECT 1;").unwrap();

    let file_node = FileNode::from_file(
        "--",
        &temp_file.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node.name, "test_node");
    assert_eq!(file_node.layer, "prepend");
}

#[test]
fn test_backward_compatibility_is_final() {
    let layers = vec![
        "prepend".to_string(),
        "normal".to_string(),
        "append".to_string(),
    ];
    let fallback_layer = "normal";

    let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(&temp_file, "-- name: test_node\n-- is_final\nSELECT 1;").unwrap();

    let file_node = FileNode::from_file(
        "--",
        &temp_file.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node.name, "test_node");
    assert_eq!(file_node.layer, "append");
}

#[test]
fn test_fallback_layer() {
    let layers = vec![
        "first".to_string(),
        "second".to_string(),
        "third".to_string(),
    ];
    let fallback_layer = "second";

    let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(&temp_file, "-- name: test_node\nSELECT 1;").unwrap();

    let file_node = FileNode::from_file(
        "--",
        &temp_file.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node.name, "test_node");
    assert_eq!(file_node.layer, "second");
}

#[test]
fn test_invalid_layer_error() {
    use crate::exceptions::FileNodeError;

    let layers = vec!["first".to_string(), "second".to_string()];
    let fallback_layer = "first";

    let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(
        &temp_file,
        "-- name: test_node\n-- layer: invalid\nSELECT 1;",
    )
    .unwrap();

    let result = FileNode::from_file(
        "--",
        &temp_file.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        FileNodeError::InvalidLayer(_, layer) => assert_eq!(layer, "invalid"),
        _ => panic!("Expected InvalidLayer error"),
    }
}

#[test]
fn test_dependencies_parsing() {
    let layers = vec!["first".to_string(), "second".to_string()];
    let fallback_layer = "first";

    let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(
        &temp_file,
        "-- name: test_node\n-- layer: first\n-- requires: dep1, dep2\n-- dropped_by: dep3\nSELECT 1;",
    )
    .unwrap();

    let file_node = FileNode::from_file(
        "--",
        &temp_file.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node.name, "test_node");
    assert_eq!(file_node.layer, "first");
    assert!(file_node.deps.contains("dep1"));
    assert!(file_node.deps.contains("dep2"));
    assert!(file_node.deps.contains("dep3"));
    assert_eq!(file_node.deps.len(), 3);
}

#[test]
fn test_schema_extraction_dot_separator() {
    assert_eq!(
        FileNode::extract_schema("my_schema.table_name"),
        Some("my_schema".to_string())
    );
    assert_eq!(
        FileNode::extract_schema("public.users"),
        Some("public".to_string())
    );
}

#[test]
fn test_schema_extraction_no_schema() {
    assert_eq!(FileNode::extract_schema("table_name"), None);
    assert_eq!(FileNode::extract_schema("simple_table"), None);
}

#[test]
fn test_schema_in_file_node() {
    let layers = vec!["first".to_string()];
    let fallback_layer = "first";

    let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(&temp_file, "-- name: my_schema.my_table\nSELECT 1;").unwrap();

    let file_node = FileNode::from_file(
        "--",
        &temp_file.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node.name, "my_schema.my_table");
    assert_eq!(file_node.schema, Some("my_schema".to_string()));
}

#[test]
fn test_manual_header_parsing() {
    let layers = vec!["normal".to_string()];
    let fallback_layer = "normal";

    let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(
        &temp_file,
        "-- My custom header\n-- manual\n-- name: test_node\nSELECT 1;",
    )
    .unwrap();

    let file_node = FileNode::from_file(
        "--",
        &temp_file.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node.name, "test_node");
    assert!(file_node.manual);
    assert!(file_node.original_headers.is_some());
}

#[test]
fn test_soft_deps_parsing() {
    let layers = vec!["normal".to_string()];
    let fallback_layer = "normal";

    let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(
        &temp_file,
        "-- name: test_node\n-- soft-deps\n-- exists: dep1\nSELECT 1;",
    )
    .unwrap();

    let file_node = FileNode::from_file(
        "--",
        &temp_file.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node.name, "test_node");
    assert!(file_node.soft_deps);
}

#[test]
fn test_final_initial_parsing() {
    let layers = vec!["prepend".to_string(), "normal".to_string()];
    let fallback_layer = "normal";

    // Test "final" marker
    let temp_file1 = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(&temp_file1, "-- name: test_final\n-- final\nSELECT 1;").unwrap();

    let file_node1 = FileNode::from_file(
        "--",
        &temp_file1.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node1.final_initial, Some("final".to_string()));

    // Test "initial" marker
    let temp_file2 = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(&temp_file2, "-- name: test_initial\n-- initial\nSELECT 1;").unwrap();

    let file_node2 = FileNode::from_file(
        "--",
        &temp_file2.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node2.final_initial, Some("initial".to_string()));
}

#[test]
fn test_node_name_header_parsing() {
    let layers = vec!["normal".to_string()];
    let fallback_layer = "normal";

    let temp_file = tempfile::NamedTempFile::with_suffix(".sql").unwrap();
    std::fs::write(
        &temp_file,
        "-- node_name: my_schema.my_table\nCREATE TABLE test (id INT);",
    )
    .unwrap();

    let file_node = FileNode::from_file(
        "--",
        &temp_file.path().to_path_buf(),
        &layers,
        fallback_layer,
        None,
    )
    .unwrap();

    assert_eq!(file_node.name, "my_schema.my_table");
    assert!(file_node.original_node_name_header.is_some());
    assert_eq!(
        file_node.original_node_name_header.unwrap(),
        "-- node_name: my_schema.my_table"
    );
}
