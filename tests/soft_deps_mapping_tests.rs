use indexmap::IndexMap;
/// Integration tests for soft dependency mapping feature
///
/// This feature allows configuring regex patterns to automatically convert
/// certain dependencies from `requires` to `exists` based on node name and
/// dependency name matching.
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use topcat::file_node::FileNode;
use topcat::header_generator::generate_header;
use topcat::soft_deps_matcher::SoftDepsMapper;

#[test]
fn test_soft_deps_mapping_basic() {
    // Create a FileNode similar to the user's example:
    // codegen_tmf.util_generate_tmf_list_function with dependencies on
    // codegen_tmf (schema) and c_tmf.ta_headers
    let mut file_node = FileNode::new(
        "codegen_tmf.util_generate_tmf_list_function".to_string(),
        PathBuf::from("/tmp/test.sql"),
        HashSet::from(["codegen_tmf".to_string(), "c_tmf.ta_headers".to_string()]),
        "normal".to_string(),
        true,
        HashSet::new(),
    );

    // Configure soft_deps_mappings: codegen_tmf nodes -> c_tmf deps become exists
    let mut soft_deps_mappings = IndexMap::new();
    soft_deps_mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());

    // Create and assign the mapper
    let mapper = SoftDepsMapper::new(&soft_deps_mappings).unwrap();
    file_node.soft_deps_mapper = Some(Rc::new(mapper));

    // Generate header
    let header = generate_header(&file_node, "--");

    println!("Generated header:\n{header}");

    // Verify that c_tmf.ta_headers is listed as "exists" instead of "requires"
    assert!(
        header.contains("-- exists: c_tmf.ta_headers"),
        "c_tmf.ta_headers should be converted to 'exists' dependency. Got:\n{header}"
    );

    // Verify that codegen_tmf is still "requires" (schema dependency is never converted)
    assert!(
        header.contains("-- requires: codegen_tmf"),
        "codegen_tmf (schema) should remain as 'requires'. Got:\n{header}"
    );

    // Verify that c_tmf.ta_headers is NOT listed as "requires"
    assert!(
        !header.contains("-- requires: c_tmf.ta_headers"),
        "c_tmf.ta_headers should NOT be a 'requires' dependency. Got:\n{header}"
    );
}

#[test]
fn test_soft_deps_mapping_multiple_patterns() {
    // Configure multiple soft_deps_mappings
    let mut soft_deps_mappings = IndexMap::new();
    soft_deps_mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());
    soft_deps_mappings.insert(r"^md_tmf\b".to_string(), r"^e_extensions\b".to_string());

    let mapper = SoftDepsMapper::new(&soft_deps_mappings).unwrap();
    let mapper_rc = Rc::new(mapper);

    // Test first pattern: codegen_tmf -> c_tmf
    let mut file_node1 = FileNode::new(
        "codegen_tmf.func1".to_string(),
        PathBuf::from("/tmp/test1.sql"),
        HashSet::from(["codegen_tmf".to_string(), "c_tmf.schema".to_string()]),
        "normal".to_string(),
        true,
        HashSet::new(),
    );
    file_node1.soft_deps_mapper = Some(Rc::clone(&mapper_rc));

    let header1 = generate_header(&file_node1, "--");
    assert!(
        header1.contains("-- exists: c_tmf.schema"),
        "c_tmf.schema should be converted to 'exists' for codegen_tmf node"
    );

    // Test second pattern: md_tmf -> e_extensions
    let mut file_node2 = FileNode::new(
        "md_tmf.table1".to_string(),
        PathBuf::from("/tmp/test2.sql"),
        HashSet::from(["md_tmf".to_string(), "e_extensions.ltree".to_string()]),
        "normal".to_string(),
        true,
        HashSet::new(),
    );
    file_node2.soft_deps_mapper = Some(Rc::clone(&mapper_rc));

    let header2 = generate_header(&file_node2, "--");
    assert!(
        header2.contains("-- exists: e_extensions.ltree"),
        "e_extensions.ltree should be converted to 'exists' for md_tmf node"
    );
}

#[test]
fn test_soft_deps_mapping_no_match() {
    // Test that dependencies NOT matching the pattern remain as "requires"
    let mut file_node = FileNode::new(
        "codegen_tmf.util_func".to_string(),
        PathBuf::from("/tmp/test.sql"),
        HashSet::from([
            "codegen_tmf".to_string(),
            "md_tmf.table".to_string(),
            "c_tmf.schema".to_string(),
        ]),
        "normal".to_string(),
        true,
        HashSet::new(),
    );

    // Configure pattern that only matches c_tmf (not md_tmf)
    let mut soft_deps_mappings = IndexMap::new();
    soft_deps_mappings.insert(r"^codegen_tmf\b".to_string(), r"^c_tmf\b".to_string());

    let mapper = SoftDepsMapper::new(&soft_deps_mappings).unwrap();
    file_node.soft_deps_mapper = Some(Rc::new(mapper));

    let header = generate_header(&file_node, "--");

    println!("Generated header:\n{header}");

    // c_tmf.schema should be converted to exists (matches pattern)
    assert!(
        header.contains("-- exists: c_tmf.schema"),
        "c_tmf.schema should be 'exists'"
    );

    // md_tmf.table should remain as requires (doesn't match the dep pattern)
    assert!(
        header.contains("-- requires: md_tmf.table"),
        "md_tmf.table should remain as 'requires'"
    );
}
