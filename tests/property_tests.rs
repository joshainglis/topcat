use proptest::prelude::*;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;
use topcat::analysis::GraphAnalyzer;
use topcat::config::Config;
use topcat::file_dag::TCGraph;
use topcat::sql_config::SqlDiscoveryConfig;

// Helper to create a test file with metadata
fn create_test_file(dir: &TempDir, name: &str, requires: &[&str], layer: &str) {
    let filename = format!("{name}.sql");
    let file_path = dir.path().join(&filename);

    let mut content = format!("-- name: {name}\n");
    if !requires.is_empty() {
        content.push_str(&format!("-- requires: {}\n", requires.join(", ")));
    }
    if !layer.is_empty() && layer != "normal" {
        content.push_str(&format!("-- layer: {layer}\n"));
    }
    content.push_str(&format!("CREATE TABLE {name} ();\n"));

    fs::write(&file_path, content).unwrap();
}

// Helper to build a graph from test directory
fn build_graph_from_dir(dir: &TempDir, layers: &[&str]) -> TCGraph {
    let extensions: Option<Vec<String>> = Some(vec!["sql".to_string()]);

    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        include_extensions: extensions.as_deref(),
        exclude_extensions: None,
        include_globs: None,
        exclude_globs: None,
        output: dir.path().join("output.sql"),
        comment_str: "--".to_string(),
        file_separator_str: String::new(),
        file_end_str: String::new(),
        include_hidden: true,
        verbose: false,
        include_node_prefixes: None,
        exclude_node_prefixes: None,
        dry_run: false,
        subdir_filter: None,
        layers: layers.iter().map(|s| s.to_string()).collect(),
        fallback_layer: "normal".to_string(),
        auto_mapping: &indexmap::IndexMap::new(),
        sql_discovery: SqlDiscoveryConfig::default(),
        header_update_mode: topcat::sql_config::HeaderUpdateMode::Never,
        header_output_dir: None,
    };

    let mut graph = TCGraph::new(&config);
    graph.build_graph().unwrap();
    graph
}

proptest! {
    /// Property: Topological sort produces valid ordering
    /// Every node appears after all its dependencies
    #[test]
    fn test_topo_sort_validity(
        node_count in 2usize..10,
        seed in any::<u64>()
    ) {
        use rand::{SeedableRng, Rng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        let dir = TempDir::new().unwrap();

        // Create nodes with random but acyclic dependencies
        let mut nodes = Vec::new();
        for i in 0..node_count {
            let name = format!("node_{i}");

            // Only depend on nodes created before this one (ensures no cycles)
            let max_deps = i.min(3);
            let num_deps = if max_deps == 0 { 0 } else { rng.random_range(0..=max_deps) };

            let mut requires = Vec::new();
            for _ in 0..num_deps {
                let dep_idx = rng.random_range(0..i);
                requires.push(format!("node_{dep_idx}"));
            }
            requires.dedup();

            let requires_refs: Vec<&str> = requires.iter().map(|s| s.as_str()).collect();
            create_test_file(&dir, &name, &requires_refs, "normal");
            nodes.push((name, requires));
        }

        let graph = build_graph_from_dir(&dir, &["normal"]);

        // Get nodes in topological order
        let sorted_paths = graph.get_sorted_files().unwrap();
        let all_nodes = graph.get_all_nodes();
        let path_to_node: HashMap<_, _> = all_nodes.iter()
            .map(|n| (n.path.clone(), n))
            .collect();

        let sorted_nodes: Vec<_> = sorted_paths.iter()
            .filter_map(|p| path_to_node.get(p).copied())
            .collect();

        // Build position map
        let mut positions = HashMap::new();
        for (pos, node) in sorted_nodes.iter().enumerate() {
            positions.insert(&node.name, pos);
        }

        // Verify: every node appears after its dependencies
        for (name, requires) in &nodes {
            if let Some(&node_pos) = positions.get(name) {
                for dep in requires {
                    if let Some(&dep_pos) = positions.get(dep) {
                        prop_assert!(
                            dep_pos < node_pos,
                            "Node {} at position {} should appear after dependency {} at position {}",
                            name, node_pos, dep, dep_pos
                        );
                    }
                }
            }
        }
    }

    /// Property: Layer ordering is respected
    /// All nodes in earlier layers appear before later layers
    #[test]
    fn test_layer_ordering(
        prepend_count in 1usize..5,
        normal_count in 1usize..5,
        append_count in 1usize..5
    ) {
        let dir = TempDir::new().unwrap();

        // Create nodes in different layers
        for i in 0..prepend_count {
            create_test_file(&dir, &format!("prep_{i}"), &[], "prepend");
        }
        for i in 0..normal_count {
            create_test_file(&dir, &format!("norm_{i}"), &[], "normal");
        }
        for i in 0..append_count {
            create_test_file(&dir, &format!("app_{i}"), &[], "append");
        }

        let graph = build_graph_from_dir(&dir, &["prepend", "normal", "append"]);

        // Get nodes in topological order
        let sorted_paths = graph.get_sorted_files().unwrap();
        let all_nodes = graph.get_all_nodes();
        let path_to_node: HashMap<_, _> = all_nodes.iter()
            .map(|n| (n.path.clone(), n))
            .collect();

        let sorted_nodes: Vec<_> = sorted_paths.iter()
            .filter_map(|p| path_to_node.get(p).copied())
            .collect();

        // Find the last prepend and first normal
        let last_prepend_pos = sorted_nodes.iter().rposition(|n| n.name.starts_with("prep_"));
        let first_normal_pos = sorted_nodes.iter().position(|n| n.name.starts_with("norm_"));
        let last_normal_pos = sorted_nodes.iter().rposition(|n| n.name.starts_with("norm_"));
        let first_append_pos = sorted_nodes.iter().position(|n| n.name.starts_with("app_"));

        // Verify layer ordering
        if let (Some(last_prep), Some(first_norm)) = (last_prepend_pos, first_normal_pos) {
            prop_assert!(
                last_prep < first_norm,
                "Last prepend node at {} should be before first normal at {}",
                last_prep, first_norm
            );
        }
        if let (Some(last_norm), Some(first_app)) = (last_normal_pos, first_append_pos) {
            prop_assert!(
                last_norm < first_app,
                "Last normal node at {} should be before first append at {}",
                last_norm, first_app
            );
        }
    }

    /// Property: Transitive dependency closure is correct
    /// If A requires B and B requires C, then A's transitive deps include C
    #[test]
    fn test_transitive_dependencies(chain_length in 2usize..8) {
        let dir = TempDir::new().unwrap();

        // Create a chain: node_0 <- node_1 <- node_2 <- ...
        create_test_file(&dir, "node_0", &[], "normal");
        for i in 1..chain_length {
            let dep = format!("node_{}", i - 1);
            create_test_file(&dir, &format!("node_{i}"), &[&dep], "normal");
        }

        let graph = build_graph_from_dir(&dir, &["normal"]);

        // The last node should transitively depend on all previous nodes
        let last_node_name = format!("node_{}", chain_length - 1);
        let transitive_deps = graph.get_transitive_dependencies(&last_node_name);

        // Should include all nodes in the chain except itself
        prop_assert_eq!(
            transitive_deps.len(),
            chain_length - 1,
            "Node {} should have {} transitive dependencies",
            last_node_name,
            chain_length - 1
        );

        // Verify all expected nodes are present
        for i in 0..chain_length - 1 {
            let expected = format!("node_{i}");
            prop_assert!(
                transitive_deps.contains(&expected),
                "Transitive dependencies should include {}",
                expected
            );
        }
    }

    /// Property: Dead branch detection is consistent
    /// If a node has no dependents and no external usage, removing it shouldn't affect other nodes
    #[test]
    fn test_orphan_consistency(
        main_count in 2usize..6,
        orphan_count in 1usize..4
    ) {
        let dir = TempDir::new().unwrap();

        // Create main nodes that depend on each other
        create_test_file(&dir, "main_0", &[], "normal");
        for i in 1..main_count {
            create_test_file(&dir, &format!("main_{i}"), &["main_0"], "normal");
        }

        // Create isolated orphan nodes
        for i in 0..orphan_count {
            create_test_file(&dir, &format!("orphan_{i}"), &[], "normal");
        }

        let graph = build_graph_from_dir(&dir, &["normal"]);

        // Find orphans (nodes with no deps and no dependents)
        let orphans = graph.find_orphans();

        // All orphan_ nodes should be identified as orphans
        prop_assert_eq!(
            orphans.len(),
            orphan_count,
            "Should find exactly {} orphan nodes",
            orphan_count
        );

        for orphan_name in &orphans {
            prop_assert!(
                orphan_name.starts_with("orphan_"),
                "Orphan {} should be an orphan node",
                orphan_name
            );
        }
    }

    /// Property: Schema extraction is consistent
    /// Nodes with schema prefixes are correctly grouped
    #[test]
    fn test_schema_extraction(
        schema_a_count in 1usize..5,
        schema_b_count in 1usize..5
    ) {
        let dir = TempDir::new().unwrap();

        // Create nodes in two schemas
        for i in 0..schema_a_count {
            create_test_file(&dir, &format!("schema_a.table_{i}"), &[], "normal");
        }
        for i in 0..schema_b_count {
            create_test_file(&dir, &format!("schema_b.table_{i}"), &[], "normal");
        }

        let graph = build_graph_from_dir(&dir, &["normal"]);
        let all_nodes = graph.get_all_nodes();

        // Count nodes by schema
        let mut schema_a_nodes = 0;
        let mut schema_b_nodes = 0;

        for node in &all_nodes {
            if node.name.starts_with("schema_a.") {
                schema_a_nodes += 1;
            } else if node.name.starts_with("schema_b.") {
                schema_b_nodes += 1;
            }
        }

        prop_assert_eq!(schema_a_nodes, schema_a_count, "Schema A node count");
        prop_assert_eq!(schema_b_nodes, schema_b_count, "Schema B node count");
    }
}

#[test]
fn test_deterministic_sort() {
    // Non-property test: Verify that topological sort is deterministic
    let dir = TempDir::new().unwrap();

    // Create nodes with no dependencies (order is ambiguous without determinism)
    for i in 0..5 {
        create_test_file(&dir, &format!("node_{i}"), &[], "normal");
    }

    let graph1 = build_graph_from_dir(&dir, &["normal"]);
    let sorted_paths1 = graph1.get_sorted_files().unwrap();
    let all_nodes1 = graph1.get_all_nodes();
    let path_to_node1: HashMap<_, _> = all_nodes1.iter().map(|n| (n.path.clone(), n)).collect();
    let sorted1: Vec<_> = sorted_paths1
        .iter()
        .filter_map(|p| path_to_node1.get(p).map(|n| &n.name))
        .collect();

    let graph2 = build_graph_from_dir(&dir, &["normal"]);
    let sorted_paths2 = graph2.get_sorted_files().unwrap();
    let all_nodes2 = graph2.get_all_nodes();
    let path_to_node2: HashMap<_, _> = all_nodes2.iter().map(|n| (n.path.clone(), n)).collect();
    let sorted2: Vec<_> = sorted_paths2
        .iter()
        .filter_map(|p| path_to_node2.get(p).map(|n| &n.name))
        .collect();

    // Sort should be deterministic
    assert_eq!(sorted1.len(), sorted2.len());
    for (n1, n2) in sorted1.iter().zip(sorted2.iter()) {
        assert_eq!(n1, n2, "Topological sort should be deterministic");
    }
}
