use std::path::PathBuf;
use topcat::config::Config;
use topcat::file_dag::TCGraph;

fn build_test_graph() -> TCGraph {
    let exts = vec!["sql".to_string()];
    let config = Config {
        input_dirs: vec![PathBuf::from("tests/input/sql")],
        include_extensions: Some(&exts),
        exclude_extensions: None,
        include_globs: None,
        exclude_globs: None,
        output: PathBuf::from("/dev/null"),
        comment_str: "--".to_string(),
        file_separator_str: String::new(),
        file_end_str: String::new(),
        include_hidden: false,
        verbose: false,
        include_node_prefixes: None,
        exclude_node_prefixes: None,
        dry_run: false,
        subdir_filter: None,
        layers: vec![
            "prepend".to_string(),
            "normal".to_string(),
            "append".to_string(),
        ],
        fallback_layer: "normal".to_string(),
        auto_mapping: &indexmap::IndexMap::new(),
        sql_discovery: Default::default(),
        header_update_mode: topcat::sql_config::HeaderUpdateMode::Never,
        header_output_dir: None,
    };

    let mut graph = TCGraph::new(&config);
    graph.build_graph().expect("Failed to build graph");
    graph
}

#[test]
fn test_get_schema_names() {
    let graph = build_test_graph();
    let schema_names = graph.get_schema_names();

    // Should have at least my_schema and my_other_schema
    assert!(schema_names.contains(&"my_schema".to_string()));
    assert!(schema_names.contains(&"my_other_schema".to_string()));
}

#[test]
fn test_get_schemas() {
    let graph = build_test_graph();
    let schemas = graph.get_schemas();

    // Should have my_schema with nodes
    let my_schema_nodes = schemas.get("my_schema").expect("my_schema not found");
    assert!(!my_schema_nodes.is_empty());

    // Should have my_other_schema with nodes
    let my_other_schema_nodes = schemas
        .get("my_other_schema")
        .expect("my_other_schema not found");
    assert!(!my_other_schema_nodes.is_empty());

    // Should have no_schema for nodes without schema prefix
    assert!(schemas.contains_key("no_schema"));
}

#[test]
fn test_get_internal_dependencies() {
    let graph = build_test_graph();
    let internal_deps = graph.get_internal_dependencies("my_schema");

    // my_schema should have some internal dependencies
    // Based on test data: my_schema.c -> my_schema.b, my_schema.b -> my_schema.a
    assert!(!internal_deps.is_empty());

    // Verify specific dependencies exist
    let has_c_to_b =
        internal_deps.contains(&("my_schema.c".to_string(), "my_schema.b".to_string()));
    let has_b_to_a =
        internal_deps.contains(&("my_schema.b".to_string(), "my_schema.a".to_string()));

    assert!(
        has_c_to_b || has_b_to_a,
        "Expected internal dependencies not found"
    );
}

#[test]
fn test_get_external_dependencies() {
    let graph = build_test_graph();
    let external_deps = graph.get_external_dependencies("my_schema");

    // my_schema should have external dependencies to my_other_schema
    // Based on test data: my_schema.c -> my_other_schema.a
    assert!(!external_deps.is_empty());

    // Check that we have a dependency to my_other_schema
    let has_dep_to_other = external_deps
        .iter()
        .any(|(_, _, target_schema)| target_schema == "my_other_schema");
    assert!(has_dep_to_other, "Expected external dependency not found");
}

#[test]
fn test_get_dependent_schemas() {
    let graph = build_test_graph();

    // Get schemas that depend on my_schema
    let dependents = graph.get_dependent_schemas("my_schema");

    // my_other_schema depends on my_schema (my_other_schema.a -> my_schema.b)
    assert!(
        dependents.contains("my_other_schema"),
        "Expected my_other_schema to depend on my_schema"
    );
}

#[test]
fn test_get_cross_schema_dependencies() {
    let graph = build_test_graph();
    let cross_deps = graph.get_cross_schema_dependencies();

    // Should have cross-schema dependencies
    assert!(!cross_deps.is_empty());

    // Should have bidirectional dependencies between my_schema and my_other_schema
    let has_my_to_other =
        cross_deps.contains(&("my_schema".to_string(), "my_other_schema".to_string()));
    let has_other_to_my =
        cross_deps.contains(&("my_other_schema".to_string(), "my_schema".to_string()));

    assert!(
        has_my_to_other || has_other_to_my,
        "Expected cross-schema dependencies not found"
    );
}

#[test]
fn test_schema_filtering() {
    let exts = vec!["sql".to_string()];
    let include_node_prefixes = vec!["my_schema".to_string(), "my_schema.".to_string()];

    let config = Config {
        input_dirs: vec![PathBuf::from("tests/input/sql")],
        include_extensions: Some(&exts),
        exclude_extensions: None,
        include_globs: None,
        exclude_globs: None,
        output: PathBuf::from("/dev/null"),
        comment_str: "--".to_string(),
        file_separator_str: String::new(),
        file_end_str: String::new(),
        include_hidden: false,
        verbose: false,
        include_node_prefixes: Some(&include_node_prefixes),
        exclude_node_prefixes: None,
        dry_run: false,
        subdir_filter: None,
        layers: vec![
            "prepend".to_string(),
            "normal".to_string(),
            "append".to_string(),
        ],
        fallback_layer: "normal".to_string(),
        auto_mapping: &indexmap::IndexMap::new(),
        sql_discovery: Default::default(),
        header_update_mode: topcat::sql_config::HeaderUpdateMode::Never,
        header_output_dir: None,
    };

    let mut graph = TCGraph::new(&config);
    graph.build_graph().expect("Failed to build graph");

    // Verify the graph was built successfully with filtering
    // Just check that it doesn't panic and has some schemas
    let schemas = graph.get_schemas();
    assert!(!schemas.is_empty());
}
