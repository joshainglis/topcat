//! Tests for header generation and updating.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use super::*;

#[test]
fn test_generate_header() {
    let file_node = FileNode::new(
        "test_schema.test_table".to_string(),
        PathBuf::from("/tmp/test.sql"),
        HashSet::from([
            "test_schema".to_string(), // Schema dependency (added by sql_parser.rs in real usage)
            "test_schema.dep1".to_string(),
            "test_schema.dep2".to_string(),
        ]),
        "normal".to_string(),
        true,
        HashSet::new(),
    );

    let header = generate_header(&file_node, "--");

    assert!(header.contains("-- name: test_schema.test_table"));
    assert!(header.contains("-- requires: test_schema"));
    assert!(
        header.contains("-- requires: test_schema.dep1")
            || header.contains("-- requires: test_schema.dep2")
    );
}

#[test]
fn test_generate_header_with_overrides() {
    let mut file_node = FileNode::new(
        "test_schema.test_table".to_string(),
        PathBuf::from("/tmp/test.sql"),
        HashSet::from(["test_schema.dep1".to_string()]),
        "normal".to_string(),
        true,
        HashSet::new(),
    );
    file_node.override_deps = HashSet::from(["test_schema.override_dep".to_string()]);

    let header = generate_header(&file_node, "--");

    assert!(header.contains("-- name: test_schema.test_table"));
    assert!(header.contains("-- requires: test_schema.dep1"));
    assert!(header.contains("-- requires: !test_schema.override_dep"));
}

#[test]
fn test_no_duplicate_schema_dependency() {
    // Test that schema dependency is only listed once (not duplicated)
    let file_node = FileNode::new(
        "c_kv.key_value".to_string(),
        PathBuf::from("/tmp/test.sql"),
        HashSet::from([
            "c_kv".to_string(), // Schema dependency (added by sql_parser.rs)
        ]),
        "normal".to_string(),
        true,
        HashSet::new(),
    );

    let header = generate_header(&file_node, "--");

    // Count occurrences of "-- requires: c_kv"
    let count = header.matches("-- requires: c_kv\n").count();
    assert_eq!(
        count, 1,
        "Schema dependency should appear exactly once, found {count} occurrences"
    );

    assert!(header.contains("-- name: c_kv.key_value"));
    assert!(header.contains("-- requires: c_kv\n"));
}

#[test]
fn test_schema_dependency_comes_first() {
    // Test that schema dependency appears before other dependencies (matching Python behavior)
    let file_node = FileNode::new(
        "md_tmf.filter_key_to_accessor_mapping".to_string(),
        PathBuf::from("/tmp/test.sql"),
        HashSet::from([
            "md_tmf".to_string(),
            "do_tmf.entity_index".to_string(),
            "e_extensions.ltree".to_string(),
            "md_tmf.jsonpath_path".to_string(),
        ]),
        "normal".to_string(),
        true,
        HashSet::new(),
    );

    let header = generate_header(&file_node, "--");

    // Find positions of dependencies
    let md_tmf_pos = header
        .find("-- requires: md_tmf\n")
        .expect("md_tmf should be present");
    let do_tmf_pos = header
        .find("-- requires: do_tmf.entity_index\n")
        .expect("do_tmf.entity_index should be present");
    let e_ext_pos = header
        .find("-- requires: e_extensions.ltree\n")
        .expect("e_extensions.ltree should be present");
    let md_tmf_json_pos = header
        .find("-- requires: md_tmf.jsonpath_path\n")
        .expect("md_tmf.jsonpath_path should be present");

    // Schema dependency should come first
    assert!(
        md_tmf_pos < do_tmf_pos,
        "md_tmf (schema) should come before do_tmf.entity_index"
    );
    assert!(
        md_tmf_pos < e_ext_pos,
        "md_tmf (schema) should come before e_extensions.ltree"
    );
    assert!(
        md_tmf_pos < md_tmf_json_pos,
        "md_tmf (schema) should come before md_tmf.jsonpath_path"
    );

    // Other dependencies should be sorted alphabetically after the schema
    assert!(
        do_tmf_pos < e_ext_pos,
        "do_tmf.entity_index should come before e_extensions.ltree (alphabetical)"
    );
    assert!(
        e_ext_pos < md_tmf_json_pos,
        "e_extensions.ltree should come before md_tmf.jsonpath_path (alphabetical)"
    );
}

#[test]
fn test_find_content_start() {
    let content = "-- name: test\n-- requires: dep1\n\nSELECT 1;";
    let start = find_content_start(content, "--");

    assert!(content[start..].starts_with("SELECT"));
}

#[test]
fn test_update_headers_in_place() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.sql");

    // Create a test file
    fs::write(
        &test_file,
        "-- name: old_name\n-- requires: old_dep\n\nSELECT 1;\n",
    )?;

    let mut file_node = FileNode::new(
        "new_name".to_string(),
        test_file.clone(),
        HashSet::from(["new_dep".to_string()]),
        "normal".to_string(),
        true,
        HashSet::new(),
    );
    file_node.discovered_deps = Some(HashSet::from(["new_dep".to_string()]));

    update_headers_in_place(&[file_node], "--", false, "sql")?;

    let updated_content = fs::read_to_string(&test_file)?;
    assert!(updated_content.contains("-- name: new_name"));
    assert!(updated_content.contains("-- requires: new_dep"));
    assert!(updated_content.contains("SELECT 1;"));

    Ok(())
}

#[test]
fn test_file_renaming() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("old_name.sql");

    // Create a test file
    fs::write(
        &test_file,
        "-- name: old_name\n-- requires: old_dep\n\nCREATE TABLE test_schema.new_table (id INT);\n",
    )?;

    let mut file_node = FileNode::new(
        "test_schema.new_table".to_string(),
        test_file.clone(),
        HashSet::from(["test_schema".to_string()]),
        "normal".to_string(),
        true,
        HashSet::new(),
    );
    file_node.discovered_deps = Some(HashSet::from(["test_schema".to_string()]));

    // Update with renaming enabled
    update_headers_in_place(&[file_node], "--", true, "sql")?;

    // Old file should not exist
    assert!(!test_file.exists());

    // New file should exist with correct name
    let new_file = temp_dir.path().join("new_table.sql");
    assert!(new_file.exists());

    // Content should be updated
    let updated_content = fs::read_to_string(&new_file)?;
    assert!(updated_content.contains("-- name: test_schema.new_table"));
    assert!(updated_content.contains("-- requires: test_schema"));
    assert!(updated_content.contains("CREATE TABLE test_schema.new_table"));

    Ok(())
}

#[test]
fn test_rename_conflict_detection() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_file1 = temp_dir.path().join("file1.sql");
    let test_file2 = temp_dir.path().join("file2.sql");

    // Create two test files that would want the same name
    fs::write(&test_file1, "CREATE TABLE test_schema.my_table (id INT);\n")?;
    fs::write(
        &test_file2,
        "CREATE TABLE test_schema.my_table (name TEXT);\n",
    )?;

    let mut file_node1 = FileNode::new(
        "test_schema.my_table".to_string(),
        test_file1.clone(),
        HashSet::new(),
        "normal".to_string(),
        true,
        HashSet::new(),
    );
    file_node1.discovered_deps = Some(HashSet::new());

    let mut file_node2 = FileNode::new(
        "test_schema.my_table".to_string(),
        test_file2.clone(),
        HashSet::new(),
        "normal".to_string(),
        true,
        HashSet::new(),
    );
    file_node2.discovered_deps = Some(HashSet::new());

    // This should fail due to conflict
    let result = update_headers_in_place(&[file_node1, file_node2], "--", true, "sql");
    assert!(result.is_err());

    Ok(())
}
