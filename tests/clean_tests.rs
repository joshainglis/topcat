// Integration tests for clean command and file deletion features

use std::fs;
use tempfile::TempDir;
use topcat::analysis::GraphAnalyzer;
use topcat::analysis::root_matcher::RootNodeMatcher;
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

fn file_exists(dir: &TempDir, path: &str) -> bool {
    dir.path().join(path).exists()
}

fn delete_files_by_names(_dir: &TempDir, graph: &TCGraph, node_names: &[&str]) {
    let node_to_path = graph.build_node_to_path_map();
    for node_name in node_names {
        if let Some(path) = node_to_path.get(*node_name) {
            fs::remove_file(path).unwrap();
        }
    }
}

#[test]
fn test_clean_dead_branches_simple() {
    let dir = TempDir::new().unwrap();

    // Create a simple dead branch: orphan with no connections
    create_test_file(&dir, "orphan.sql", "-- name: orphan\nSELECT 1;");

    // Create another branch: root -> leaf
    // In a closed system without external references, these will also be dead
    create_test_file(&dir, "root.sql", "-- name: root\nSELECT 1;");
    create_test_file(
        &dir,
        "leaf.sql",
        "-- name: leaf\n-- requires: root\nSELECT 1;",
    );

    let graph = build_test_graph(&dir);
    let dead_branches = graph.find_dead_branches(None);

    // In a closed system, root and leaf are also dead (no external references)
    // dead_branches contains root and leaf, but not orphan (orphans are separate)
    assert_eq!(dead_branches.len(), 2);
    assert!(dead_branches.contains("root"));
    assert!(dead_branches.contains("leaf"));

    // Orphans are a separate category
    let orphans = graph.find_orphans();
    assert_eq!(orphans.len(), 1);
    assert!(orphans.contains("orphan"));

    // Verify file exists before deletion
    assert!(file_exists(&dir, "orphan.sql"));

    // Delete the orphan
    delete_files_by_names(&dir, &graph, &["orphan"]);

    // Verify file was deleted
    assert!(!file_exists(&dir, "orphan.sql"));

    // Verify other files still exist
    assert!(file_exists(&dir, "root.sql"));
    assert!(file_exists(&dir, "leaf.sql"));
}

#[test]
fn test_clean_dead_branches_with_subtree() {
    let dir = TempDir::new().unwrap();

    // Create trees: root -> branch1 -> leaf1 and dead_root -> dead_leaf
    // In a closed system, ALL will be dead (no external references)
    create_test_file(&dir, "root.sql", "-- name: root\nSELECT 1;");
    create_test_file(
        &dir,
        "branch1.sql",
        "-- name: branch1\n-- requires: root\nSELECT 1;",
    );
    create_test_file(
        &dir,
        "leaf1.sql",
        "-- name: leaf1\n-- requires: branch1\nSELECT 1;",
    );

    create_test_file(&dir, "dead_root.sql", "-- name: dead_root\nSELECT 1;");
    create_test_file(
        &dir,
        "dead_leaf.sql",
        "-- name: dead_leaf\n-- requires: dead_root\nSELECT 1;",
    );

    let graph = build_test_graph(&dir);
    let dead_branches = graph.find_dead_branches(None);

    // In a closed system without external references, all 5 nodes are dead
    assert_eq!(dead_branches.len(), 5);
    assert!(dead_branches.contains("root"));
    assert!(dead_branches.contains("branch1"));
    assert!(dead_branches.contains("leaf1"));
    assert!(dead_branches.contains("dead_root"));
    assert!(dead_branches.contains("dead_leaf"));

    // Delete just the "dead_" prefixed files for this test
    delete_files_by_names(&dir, &graph, &["dead_root", "dead_leaf"]);

    // Verify specified files were deleted
    assert!(!file_exists(&dir, "dead_root.sql"));
    assert!(!file_exists(&dir, "dead_leaf.sql"));

    // Verify other files still exist
    assert!(file_exists(&dir, "root.sql"));
    assert!(file_exists(&dir, "branch1.sql"));
    assert!(file_exists(&dir, "leaf1.sql"));
}

#[test]
fn test_clean_with_root_node_protection() {
    let dir = TempDir::new().unwrap();

    // Create a chain that we'll protect and an unprotected orphan
    create_test_file(
        &dir,
        "protected_root.sql",
        "-- name: protected_root\nSELECT 1;",
    );
    create_test_file(
        &dir,
        "protected_leaf.sql",
        "-- name: protected_leaf\n-- requires: protected_root\nSELECT 1;",
    );
    create_test_file(&dir, "unprotected.sql", "-- name: unprotected\nSELECT 1;");

    let graph = build_test_graph(&dir);

    // Find dead branches without protection - unprotected is an orphan, not in dead_branches
    // protected_root and protected_leaf are in dead_branches (closed system)
    let dead_without_protection = graph.find_dead_branches(None);
    assert_eq!(dead_without_protection.len(), 2);
    assert!(dead_without_protection.contains("protected_root"));
    assert!(dead_without_protection.contains("protected_leaf"));

    // Create root matcher to protect "protected_leaf"
    // Protecting the leaf protects the entire chain!
    let root_matcher =
        RootNodeMatcher::new(vec!["protected_leaf".to_string()], vec![], vec![], vec![]).unwrap();

    // Find dead branches with protection
    let dead_with_protection = graph.find_dead_branches(Some(&root_matcher));

    // Nothing should be in dead_branches now (protected_leaf protects the whole tree)
    assert_eq!(dead_with_protection.len(), 0);

    // The orphan is in a separate category
    let orphans = graph.find_orphans();
    assert_eq!(orphans.len(), 1);
    assert!(orphans.contains("unprotected"));
}

#[test]
fn test_clean_orphans() {
    let dir = TempDir::new().unwrap();

    // Create orphans (no dependencies, no dependents)
    create_test_file(&dir, "orphan1.sql", "-- name: orphan1\nSELECT 1;");
    create_test_file(&dir, "orphan2.sql", "-- name: orphan2\nSELECT 1;");

    // Create a connected tree
    create_test_file(&dir, "root.sql", "-- name: root\nSELECT 1;");
    create_test_file(
        &dir,
        "leaf.sql",
        "-- name: leaf\n-- requires: root\nSELECT 1;",
    );

    let graph = build_test_graph(&dir);
    let orphans = graph.find_orphans();

    assert_eq!(orphans.len(), 2);
    assert!(orphans.contains("orphan1"));
    assert!(orphans.contains("orphan2"));

    // Delete orphans
    delete_files_by_names(&dir, &graph, &["orphan1", "orphan2"]);

    // Verify orphans were deleted
    assert!(!file_exists(&dir, "orphan1.sql"));
    assert!(!file_exists(&dir, "orphan2.sql"));

    // Verify connected files still exist
    assert!(file_exists(&dir, "root.sql"));
    assert!(file_exists(&dir, "leaf.sql"));
}

#[test]
fn test_clean_unrequired() {
    let dir = TempDir::new().unwrap();

    // Create unrequired files (no dependents, but may have dependencies)
    create_test_file(&dir, "unrequired1.sql", "-- name: unrequired1\nSELECT 1;");
    create_test_file(
        &dir,
        "unrequired2.sql",
        "-- name: unrequired2\n-- requires: root\nSELECT 1;",
    );

    // Create a required chain
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
    let unrequired = graph.find_unrequired();

    // unrequired1, unrequired2, and leaf have no dependents
    assert_eq!(unrequired.len(), 3);
    assert!(unrequired.contains("unrequired1"));
    assert!(unrequired.contains("unrequired2"));
    assert!(unrequired.contains("leaf"));

    // Delete only the truly unrequired files
    delete_files_by_names(&dir, &graph, &["unrequired1", "unrequired2"]);

    // Verify unrequired files were deleted
    assert!(!file_exists(&dir, "unrequired1.sql"));
    assert!(!file_exists(&dir, "unrequired2.sql"));

    // Verify required files still exist
    assert!(file_exists(&dir, "root.sql"));
    assert!(file_exists(&dir, "middle.sql"));
    assert!(file_exists(&dir, "leaf.sql"));
}

#[test]
fn test_clean_respects_dependencies() {
    let dir = TempDir::new().unwrap();

    // Create a dependency chain: root -> middle -> leaf
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
    let dependents_map = graph.build_dependents_map();

    // root has dependents (middle)
    assert!(dependents_map.contains_key("root"));
    assert_eq!(dependents_map.get("root").unwrap().len(), 1);
    assert!(dependents_map.get("root").unwrap().contains("middle"));

    // middle has dependents (leaf)
    assert!(dependents_map.contains_key("middle"));
    assert_eq!(dependents_map.get("middle").unwrap().len(), 1);
    assert!(dependents_map.get("middle").unwrap().contains("leaf"));

    // leaf has no dependents
    assert!(!dependents_map.contains_key("leaf"));

    // Attempting to delete "middle" should fail because it has dependents
    // (This would be checked by the clean targets command)
    let has_dependents = dependents_map
        .get("middle")
        .map(|d| !d.is_empty())
        .unwrap_or(false);
    assert!(
        has_dependents,
        "middle should have dependents and cannot be safely deleted"
    );
}

#[test]
fn test_clean_partial_deletion_error_handling() {
    let dir = TempDir::new().unwrap();

    // Create test files
    create_test_file(&dir, "file1.sql", "-- name: file1\nSELECT 1;");
    create_test_file(&dir, "file2.sql", "-- name: file2\nSELECT 1;");
    create_test_file(&dir, "file3.sql", "-- name: file3\nSELECT 1;");

    let graph = build_test_graph(&dir);

    // Delete file1 successfully
    delete_files_by_names(&dir, &graph, &["file1"]);
    assert!(!file_exists(&dir, "file1.sql"));

    // Try to delete file1 again (should fail silently or with error)
    let node_to_path = graph.build_node_to_path_map();
    if let Some(path) = node_to_path.get("file1") {
        let result = fs::remove_file(path);
        assert!(result.is_err(), "Deleting non-existent file should fail");
    }

    // Other files should still exist
    assert!(file_exists(&dir, "file2.sql"));
    assert!(file_exists(&dir, "file3.sql"));
}

#[test]
fn test_clean_empty_directory_handling() {
    let dir = TempDir::new().unwrap();

    // Create a subdirectory with a file
    create_test_file(&dir, "subdir/orphan.sql", "-- name: orphan\nSELECT 1;");

    let graph = build_test_graph(&dir);
    let orphans = graph.find_orphans();

    assert_eq!(orphans.len(), 1);
    assert!(orphans.contains("orphan"));

    // Delete the orphan
    delete_files_by_names(&dir, &graph, &["orphan"]);

    // File should be deleted
    assert!(!file_exists(&dir, "subdir/orphan.sql"));

    // Directory should still exist (we only delete files, not directories)
    assert!(dir.path().join("subdir").exists());
}

#[test]
fn test_node_to_path_mapping() {
    let dir = TempDir::new().unwrap();

    create_test_file(&dir, "test1.sql", "-- name: test1\nSELECT 1;");
    create_test_file(&dir, "subdir/test2.sql", "-- name: test2\nSELECT 1;");

    let graph = build_test_graph(&dir);
    let node_to_path = graph.build_node_to_path_map();

    // Verify mapping works correctly
    assert_eq!(node_to_path.len(), 2);
    assert!(node_to_path.contains_key("test1"));
    assert!(node_to_path.contains_key("test2"));

    // Verify paths end with expected filenames
    let test1_path = node_to_path.get("test1").unwrap();
    assert!(test1_path.to_string_lossy().ends_with("test1.sql"));

    let test2_path = node_to_path.get("test2").unwrap();
    assert!(test2_path.to_string_lossy().ends_with("test2.sql"));
}

#[test]
fn test_clean_with_glob_pattern_protection() {
    let dir = TempDir::new().unwrap();

    // Create files in different directories
    create_test_file(&dir, "api/endpoint1.sql", "-- name: endpoint1\nSELECT 1;");
    create_test_file(&dir, "api/endpoint2.sql", "-- name: endpoint2\nSELECT 1;");
    create_test_file(&dir, "internal/helper.sql", "-- name: helper\nSELECT 1;");

    let graph = build_test_graph(&dir);

    // Create root matcher with glob pattern to protect api/* files
    let api_glob = format!("{}/**", dir.path().join("api").display());
    let root_matcher = RootNodeMatcher::new(vec![], vec![api_glob], vec![], vec![]).unwrap();

    // Check protection for node_to_path mapping
    let node_to_path = graph.build_node_to_path_map();

    let endpoint1_path = node_to_path.get("endpoint1").unwrap();
    assert!(root_matcher.is_root("endpoint1", endpoint1_path));

    let endpoint2_path = node_to_path.get("endpoint2").unwrap();
    assert!(root_matcher.is_root("endpoint2", endpoint2_path));

    let helper_path = node_to_path.get("helper").unwrap();
    assert!(!root_matcher.is_root("helper", helper_path));
}

#[test]
fn test_build_dependents_map_public() {
    let dir = TempDir::new().unwrap();

    create_test_file(&dir, "a.sql", "-- name: a\nSELECT 1;");
    create_test_file(&dir, "b.sql", "-- name: b\n-- requires: a\nSELECT 1;");
    create_test_file(&dir, "c.sql", "-- name: c\n-- requires: a\nSELECT 1;");

    let graph = build_test_graph(&dir);

    // Verify build_dependents_map is public and works correctly
    let dependents_map = graph.build_dependents_map();

    // 'a' should have two dependents: 'b' and 'c'
    assert!(dependents_map.contains_key("a"));
    let a_dependents = dependents_map.get("a").unwrap();
    assert_eq!(a_dependents.len(), 2);
    assert!(a_dependents.contains("b"));
    assert!(a_dependents.contains("c"));

    // 'b' and 'c' should have no dependents
    assert!(!dependents_map.contains_key("b"));
    assert!(!dependents_map.contains_key("c"));
}
