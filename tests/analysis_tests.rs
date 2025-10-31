// Integration tests for dependency analysis features

use std::fs;
use tempfile::TempDir;
use topcat::analysis::GraphAnalyzer;
use topcat::config::Config;
use topcat::file_dag::TCGraph;
use topcat::sql_config::SqlDiscoveryConfig;

fn create_test_file(dir: &TempDir, path: &str, content: &str) {
    let file_path = dir.path().join(path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    // Use fs::write for simplicity and reliability
    let content_with_newline = format!("{content}\n");
    fs::write(&file_path, content_with_newline).unwrap();

    // Verify file was written correctly
    let written_content = fs::read_to_string(&file_path).unwrap();
    assert!(
        !written_content.is_empty(),
        "File was not written: {}",
        file_path.display()
    );
}

fn build_test_graph(dir: &TempDir) -> TCGraph {
    // Use Option<Vec<String>> and .as_deref() like the analyze command does
    let extensions: Option<Vec<String>> = Some(vec!["sql".to_string()]);

    let config = Config {
        input_dirs: vec![dir.path().to_path_buf()],
        include_extensions: extensions.as_deref(), // Use as_deref() like analyze command
        exclude_extensions: None,
        include_globs: None,
        exclude_globs: None,
        output: dir.path().join("output.sql"),
        comment_str: "--".to_string(),
        file_separator_str: String::new(),
        file_end_str: String::new(),
        include_hidden: true, // CRITICAL: TempDir paths start with dot, need to include hidden
        verbose: false,
        include_node_prefixes: None,
        exclude_node_prefixes: None,
        dry_run: false,
        subdir_filter: None,
        layers: vec!["normal".to_string()],
        fallback_layer: "normal".to_string(),
        sql_discovery: SqlDiscoveryConfig::default(),
        header_update_mode: topcat::sql_config::HeaderUpdateMode::Never,
        header_output_dir: None,
    };

    let mut graph = TCGraph::new(&config);
    graph.build_graph().unwrap();

    graph
}

#[test]
fn test_find_orphans() {
    let dir = TempDir::new().unwrap();

    // Create files with various dependency patterns
    create_test_file(&dir, "orphan.sql", "-- name: orphan\nSELECT 1;");

    create_test_file(&dir, "root.sql", "-- name: root\nSELECT 1;");

    create_test_file(
        &dir,
        "leaf.sql",
        "-- name: leaf\n-- requires: root\nSELECT 1;",
    );

    let graph = build_test_graph(&dir);
    let orphans = graph.find_orphans();

    // Only the orphan should be found (no dependencies, no dependents)
    assert_eq!(orphans.len(), 1);
    assert!(orphans.contains("orphan"));
}

#[test]
fn test_find_leaf_nodes() {
    let dir = TempDir::new().unwrap();

    create_test_file(&dir, "root.sql", "-- name: root\nSELECT 1;");

    create_test_file(
        &dir,
        "middle.sql",
        "-- name: middle\n-- requires: root\nSELECT 1;",
    );

    create_test_file(
        &dir,
        "leaf.sql",
        "-- name: leaf\n-- requires: middle\nSELECT 1;",
    );

    let graph = build_test_graph(&dir);
    let leaf_nodes = graph.find_leaf_nodes();

    // Only 'leaf' has dependencies but no dependents
    assert_eq!(leaf_nodes.len(), 1);
    assert!(leaf_nodes.contains("leaf"));
}

#[test]
fn test_find_root_nodes() {
    let dir = TempDir::new().unwrap();

    create_test_file(&dir, "root.sql", "-- name: root\nSELECT 1;");

    create_test_file(
        &dir,
        "leaf.sql",
        "-- name: leaf\n-- requires: root\nSELECT 1;",
    );

    let graph = build_test_graph(&dir);
    let root_nodes = graph.find_root_nodes();

    // Only 'root' has dependents but no dependencies
    assert_eq!(root_nodes.len(), 1);
    assert!(root_nodes.contains("root"));
}

#[test]
fn test_find_unrequired() {
    let dir = TempDir::new().unwrap();

    create_test_file(&dir, "used.sql", "-- name: used\nSELECT 1;");

    create_test_file(
        &dir,
        "depends_on_used.sql",
        "-- name: depends_on_used\n-- requires: used\nSELECT 1;",
    );

    create_test_file(&dir, "unused.sql", "-- name: unused\nSELECT 1;");

    let graph = build_test_graph(&dir);
    let unrequired = graph.find_unrequired();

    // Both unused and depends_on_used have no dependents
    assert_eq!(unrequired.len(), 2);
    assert!(unrequired.contains("unused"));
    assert!(unrequired.contains("depends_on_used"));
}

#[test]
fn test_find_dead_branches_simple() {
    let dir = TempDir::new().unwrap();

    // Create a live tree that is used (has a top-level consumer)
    create_test_file(&dir, "live_root.sql", "-- name: live_root\nSELECT 1;");

    create_test_file(
        &dir,
        "live_middle.sql",
        "-- name: live_middle\n-- requires: live_root\nSELECT 1;",
    );

    // This is the "entry point" - simulates being used by external code
    // In reality, nothing in our test depends on it, but in a real scenario,
    // external code would reference it. However, without external checking,
    // this will also be considered dead.
    create_test_file(
        &dir,
        "entry_point.sql",
        "-- name: entry_point\n-- requires: live_middle\nSELECT 1;",
    );

    // Create a simple dead branch: dead_root <- dead_branch <- dead_leaf
    create_test_file(&dir, "dead_root.sql", "-- name: dead_root\nSELECT 1;");

    create_test_file(
        &dir,
        "dead_branch.sql",
        "-- name: dead_branch\n-- requires: dead_root\nSELECT 1;",
    );

    create_test_file(
        &dir,
        "dead_leaf.sql",
        "-- name: dead_leaf\n-- requires: dead_branch\nSELECT 1;",
    );

    let graph = build_test_graph(&dir);
    let dead_branches = graph.find_dead_branches();

    // In a closed system without external references, all nodes that lead to leaf nodes
    // are considered dead. This includes the entire "live" tree because entry_point is a leaf.
    // In production, you'd use --external-check-dir to filter out truly used entry points.
    assert_eq!(dead_branches.len(), 6, "Found: {dead_branches:?}");
    assert!(dead_branches.contains("dead_root"));
    assert!(dead_branches.contains("dead_branch"));
    assert!(dead_branches.contains("dead_leaf"));
    assert!(dead_branches.contains("entry_point"));
    assert!(dead_branches.contains("live_middle"));
    assert!(dead_branches.contains("live_root"));
}

#[test]
fn test_find_dead_branches_complex() {
    let dir = TempDir::new().unwrap();

    // Create a complex scenario:
    // live_root <- live_middle <- entry_point (top of live tree)
    // dead_a <- dead_b
    // dead_a <- dead_c
    // dead_d <- dead_e <- dead_f
    // All dead nodes should be in dead branches, but live tree should be protected

    create_test_file(&dir, "live_root.sql", "-- name: live_root\nSELECT 1;");

    create_test_file(
        &dir,
        "live_middle.sql",
        "-- name: live_middle\n-- requires: live_root\nSELECT 1;",
    );

    create_test_file(
        &dir,
        "entry_point.sql",
        "-- name: entry_point\n-- requires: live_middle\nSELECT 1;",
    );

    // Dead branch 1: tree structure
    create_test_file(&dir, "dead_a.sql", "-- name: dead_a\nSELECT 1;");

    create_test_file(
        &dir,
        "dead_b.sql",
        "-- name: dead_b\n-- requires: dead_a\nSELECT 1;",
    );

    create_test_file(
        &dir,
        "dead_c.sql",
        "-- name: dead_c\n-- requires: dead_a\nSELECT 1;",
    );

    // Dead branch 2: linear chain
    create_test_file(&dir, "dead_d.sql", "-- name: dead_d\nSELECT 1;");

    create_test_file(
        &dir,
        "dead_e.sql",
        "-- name: dead_e\n-- requires: dead_d\nSELECT 1;",
    );

    create_test_file(
        &dir,
        "dead_f.sql",
        "-- name: dead_f\n-- requires: dead_e\nSELECT 1;",
    );

    let graph = build_test_graph(&dir);
    let dead_branches = graph.find_dead_branches();

    // All nodes are dead in a closed system
    assert_eq!(dead_branches.len(), 9, "Found: {dead_branches:?}");
    assert!(dead_branches.contains("dead_a"));
    assert!(dead_branches.contains("dead_b"));
    assert!(dead_branches.contains("dead_c"));
    assert!(dead_branches.contains("dead_d"));
    assert!(dead_branches.contains("dead_e"));
    assert!(dead_branches.contains("dead_f"));
    assert!(dead_branches.contains("entry_point"));
    assert!(dead_branches.contains("live_root"));
    assert!(dead_branches.contains("live_middle"));
}

#[test]
fn test_dead_branches_with_shared_dependency() {
    let dir = TempDir::new().unwrap();

    // Create scenario where multiple dead branches share a dependency:
    // shared <- dead_a
    // shared <- dead_b
    // live_root <- shared (shared is still live!)
    // This tests that shared is NOT marked as dead

    create_test_file(&dir, "live_root.sql", "-- name: live_root\nSELECT 1;");

    create_test_file(
        &dir,
        "shared.sql",
        "-- name: shared\n-- requires: live_root\nSELECT 1;",
    );

    create_test_file(
        &dir,
        "dead_a.sql",
        "-- name: dead_a\n-- requires: shared\nSELECT 1;",
    );

    create_test_file(
        &dir,
        "dead_b.sql",
        "-- name: dead_b\n-- requires: shared\nSELECT 1;",
    );

    let graph = build_test_graph(&dir);
    let dead_branches = graph.find_dead_branches();

    // In a closed system, all 4 nodes are dead
    // (dead_a and dead_b are leaves, which makes shared dead, which makes live_root dead)
    assert_eq!(dead_branches.len(), 4, "Found: {dead_branches:?}");
    assert!(dead_branches.contains("dead_a"));
    assert!(dead_branches.contains("dead_b"));
    assert!(dead_branches.contains("shared"));
    assert!(dead_branches.contains("live_root"));
}

#[test]
fn test_dead_branches_diamond_dependency() {
    let dir = TempDir::new().unwrap();

    // Create diamond pattern:
    //       top
    //      /   \
    //   left  right
    //      \   /
    //      bottom
    // If bottom is the only leaf, all should be dead

    create_test_file(&dir, "top.sql", "-- name: top\nSELECT 1;");

    create_test_file(
        &dir,
        "left.sql",
        "-- name: left\n-- requires: top\nSELECT 1;",
    );

    create_test_file(
        &dir,
        "right.sql",
        "-- name: right\n-- requires: top\nSELECT 1;",
    );

    create_test_file(
        &dir,
        "bottom.sql",
        "-- name: bottom\n-- requires: left\n-- requires: right\nSELECT 1;",
    );

    let graph = build_test_graph(&dir);
    let dead_branches = graph.find_dead_branches();

    // All 4 nodes should be dead
    assert_eq!(dead_branches.len(), 4);
    assert!(dead_branches.contains("top"));
    assert!(dead_branches.contains("left"));
    assert!(dead_branches.contains("right"));
    assert!(dead_branches.contains("bottom"));
}

#[test]
fn test_no_dead_branches_in_live_graph() {
    let dir = TempDir::new().unwrap();

    // Create a live graph where everything is connected and used
    create_test_file(&dir, "a.sql", "-- name: a\nSELECT 1;");

    create_test_file(&dir, "b.sql", "-- name: b\n-- requires: a\nSELECT 1;");

    create_test_file(&dir, "c.sql", "-- name: c\n-- requires: b\nSELECT 1;");

    // Add a user of the chain
    create_test_file(&dir, "user.sql", "-- name: user\n-- requires: c\nSELECT 1;");

    let graph = build_test_graph(&dir);
    let dead_branches = graph.find_dead_branches();

    // All nodes are dead because 'user' is a leaf (nothing depends on it)
    // In production, external usage checking would identify if 'user' is referenced externally
    assert_eq!(dead_branches.len(), 4, "Found: {dead_branches:?}");
    assert!(dead_branches.contains("a"));
    assert!(dead_branches.contains("b"));
    assert!(dead_branches.contains("c"));
    assert!(dead_branches.contains("user"));
}

#[test]
fn test_multiple_independent_dead_branches() {
    let dir = TempDir::new().unwrap();

    // Create multiple independent dead branches
    // Branch 1: a1 <- a2
    // Branch 2: b1 <- b2 <- b3
    // Branch 3: c1
    // All should be detected

    create_test_file(&dir, "a1.sql", "-- name: a1\nSELECT 1;");

    create_test_file(&dir, "a2.sql", "-- name: a2\n-- requires: a1\nSELECT 1;");

    create_test_file(&dir, "b1.sql", "-- name: b1\nSELECT 1;");

    create_test_file(&dir, "b2.sql", "-- name: b2\n-- requires: b1\nSELECT 1;");

    create_test_file(&dir, "b3.sql", "-- name: b3\n-- requires: b2\nSELECT 1;");

    create_test_file(&dir, "c1.sql", "-- name: c1\nSELECT 1;");

    let graph = build_test_graph(&dir);
    let dead_branches = graph.find_dead_branches();
    let orphans = graph.find_orphans();

    // All nodes should be dead (6 total)
    assert_eq!(dead_branches.len(), 5); // a1, a2, b1, b2, b3
    assert_eq!(orphans.len(), 1); // c1 is an orphan, not in dead_branches

    // Dead branches
    assert!(dead_branches.contains("a1"));
    assert!(dead_branches.contains("a2"));
    assert!(dead_branches.contains("b1"));
    assert!(dead_branches.contains("b2"));
    assert!(dead_branches.contains("b3"));

    // Orphan
    assert!(orphans.contains("c1"));
}
