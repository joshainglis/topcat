use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs;
use tempfile::TempDir;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::analysis::GraphAnalyzer;
use topcat::config::Config;
use topcat::file_dag::TCGraph;
use topcat::sql_config::SqlDiscoveryConfig;

// Helper to create a test file with metadata
fn create_test_file(dir: &TempDir, name: &str, requires: &[&str], layer: &str) {
    let filename = format!("{}.sql", name);
    let file_path = dir.path().join(&filename);

    let mut content = format!("-- name: {}\n", name);
    if !requires.is_empty() {
        content.push_str(&format!("-- requires: {}\n", requires.join(", ")));
    }
    if !layer.is_empty() && layer != "normal" {
        content.push_str(&format!("-- layer: {}\n", layer));
    }
    content.push_str(&format!("CREATE TABLE {} ();\n", name));

    fs::write(&file_path, content).unwrap();
}

// Create a test graph with a specific number of nodes and dependencies
fn setup_test_graph(node_count: usize, avg_deps_per_node: usize) -> (TempDir, TCGraph) {
    let dir = TempDir::new().unwrap();

    // Create nodes with dependencies on earlier nodes
    for i in 0..node_count {
        let name = format!("node_{}", i);

        // Create dependencies to earlier nodes
        let num_deps = if i == 0 {
            0
        } else {
            avg_deps_per_node.min(i)
        };

        let requires: Vec<String> = (0..num_deps)
            .map(|j| format!("node_{}", i.saturating_sub(j + 1)))
            .collect();
        let requires_refs: Vec<&str> = requires.iter().map(|s| s.as_str()).collect();

        create_test_file(&dir, &name, &requires_refs, "normal");
    }

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
        layers: vec!["normal".to_string()],
        fallback_layer: "normal".to_string(),
        sql_discovery: SqlDiscoveryConfig::default(),
        header_update_mode: topcat::sql_config::HeaderUpdateMode::Never,
        header_output_dir: None,
    };

    let mut graph = TCGraph::new(&config);
    graph.build_graph().unwrap();

    (dir, graph)
}

fn bench_graph_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_building");

    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let dir = TempDir::new().unwrap();
            for i in 0..size {
                let name = format!("node_{}", i);
                let deps = if i > 0 {
                    vec![format!("node_{}", i - 1)]
                } else {
                    vec![]
                };
                let deps_refs: Vec<&str> = deps.iter().map(|s| s.as_str()).collect();
                create_test_file(&dir, &name, &deps_refs, "normal");
            }

            b.iter(|| {
                // Benchmark: build graph
                let extensions = vec!["sql".to_string()];
                let config = Config {
                    input_dirs: vec![dir.path().to_path_buf()],
                    include_extensions: Some(&extensions),
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
                    layers: vec!["normal".to_string()],
                    fallback_layer: "normal".to_string(),
                    sql_discovery: SqlDiscoveryConfig::default(),
                    header_update_mode: topcat::sql_config::HeaderUpdateMode::Never,
                    header_output_dir: None,
                };
                let mut graph = TCGraph::new(&config);
                black_box(graph.build_graph().unwrap());
            });
        });
    }

    group.finish();
}

fn bench_topological_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("topological_sort");

    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let (_dir, graph) = setup_test_graph(size, 2);

            b.iter(|| {
                let sorted = graph.get_sorted_files().unwrap();
                black_box(sorted);
            });
        });
    }

    group.finish();
}

fn bench_transitive_dependencies(c: &mut Criterion) {
    let mut group = c.benchmark_group("transitive_dependencies");

    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let (_dir, graph) = setup_test_graph(size, 2);
            let last_node = format!("node_{}", size - 1);

            b.iter(|| {
                let deps = graph.get_transitive_dependencies(&last_node);
                black_box(deps);
            });
        });
    }

    group.finish();
}

fn bench_orphan_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("orphan_detection");

    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let dir = TempDir::new().unwrap();

            // Create a graph with half connected nodes and half orphans
            for i in 0..size / 2 {
                let name = format!("connected_{}", i);
                if i > 0 {
                    create_test_file(&dir, &name, &[&format!("connected_0")], "normal");
                } else {
                    create_test_file(&dir, &name, &[], "normal");
                }
            }

            for i in 0..size / 2 {
                let name = format!("orphan_{}", i);
                create_test_file(&dir, &name, &[], "normal");
            }

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
                layers: vec!["normal".to_string()],
                fallback_layer: "normal".to_string(),
                sql_discovery: SqlDiscoveryConfig::default(),
                header_update_mode: topcat::sql_config::HeaderUpdateMode::Never,
                header_output_dir: None,
            };

            let mut graph = TCGraph::new(&config);
            graph.build_graph().unwrap();

            b.iter(|| {
                let orphans = graph.find_orphans();
                black_box(orphans);
            });
        });
    }

    group.finish();
}

fn bench_dead_branches(c: &mut Criterion) {
    let mut group = c.benchmark_group("dead_branch_detection");

    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let dir = TempDir::new().unwrap();

            // Create a main tree and a dead branch
            create_test_file(&dir, "main_root", &[], "normal");
            for i in 1..size / 2 {
                create_test_file(&dir, &format!("main_{}", i), &["main_root"], "normal");
            }

            // Create dead branch
            create_test_file(&dir, "dead_root", &[], "normal");
            for i in 1..size / 2 {
                create_test_file(&dir, &format!("dead_{}", i), &["dead_root"], "normal");
            }

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
                layers: vec!["normal".to_string()],
                fallback_layer: "normal".to_string(),
                sql_discovery: SqlDiscoveryConfig::default(),
                header_update_mode: topcat::sql_config::HeaderUpdateMode::Never,
                header_output_dir: None,
            };

            let mut graph = TCGraph::new(&config);
            graph.build_graph().unwrap();

            b.iter(|| {
                // Protect main_root as root node
                let matcher = RootNodeMatcher::new(
                    vec!["main_root".to_string()],
                    vec![],
                    vec![],
                    vec![],
                ).unwrap();
                let dead_branches = graph.find_dead_branches(Some(&matcher));
                black_box(dead_branches);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_graph_building,
    bench_topological_sort,
    bench_transitive_dependencies,
    bench_orphan_detection,
    bench_dead_branches
);
criterion_main!(benches);
