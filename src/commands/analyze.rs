use std::path::PathBuf;

use clap::{Args, Subcommand};
use comfy_table::{Cell, Color, Table};
use env_logger::Builder;
use log::LevelFilter;

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::config;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::sql_config;

/// Analyze dependency structure and find cleanup candidates
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    #[arg(
        short = 'i',
        long = "input-dirs",
        help = "Paths to directories containing files to analyze",
        value_name = "DIRS"
    )]
    input_dirs: Vec<PathBuf>,

    #[arg(
        short = 'e',
        long = "include-exts",
        help = "Only include files with the given file extensions",
        value_name = "EXTENSIONS"
    )]
    include_file_extensions: Option<Vec<String>>,

    #[arg(
        short = 'E',
        long = "exclude-exts",
        help = "Exclude files with the given file extensions",
        value_name = "EXTENSIONS"
    )]
    exclude_file_extensions: Option<Vec<String>>,

    #[arg(
        short = 'g',
        long = "include-glob",
        help = "Only include files matching glob pattern",
        value_name = "PATTERN"
    )]
    include_globs: Option<Vec<String>>,

    #[arg(
        short = 'G',
        long = "exclude-glob",
        help = "Exclude files matching given glob pattern",
        value_name = "PATTERN"
    )]
    exclude_globs: Option<Vec<String>>,

    #[arg(
        short = 'c',
        long = "comment-prefix",
        help = "The string used to denote a comment",
        default_value = "--"
    )]
    comment_str: String,

    #[arg(long = "include-hidden", help = "Include hidden files and directories")]
    include_hidden_files_and_directories: bool,

    #[arg(short = 'v', long = "verbose", help = "Print debug information")]
    verbose: bool,

    #[arg(
        short = 'q',
        long = "quiet",
        help = "Suppress output, only return exit code"
    )]
    quiet: bool,

    #[arg(
        long = "layers",
        help = "Comma-separated list of layer names in order",
        value_name = "LAYERS"
    )]
    layers: Option<String>,

    #[arg(
        long = "fallback-layer",
        help = "Default layer for nodes without explicit layer declaration",
        value_name = "LAYER"
    )]
    fallback_layer: Option<String>,

    // SQL Discovery Options (for building the graph)
    #[arg(
        long = "enable-sql-discovery",
        help = "Enable automatic dependency discovery from SQL content"
    )]
    enable_sql_discovery: bool,

    #[arg(
        long = "sql-config",
        help = "Path to TOML configuration file for SQL discovery patterns",
        value_name = "FILE"
    )]
    sql_config_file: Option<PathBuf>,

    #[arg(
        long = "schema-pattern",
        help = "Regex pattern for matching schema names",
        value_name = "PATTERN"
    )]
    schema_pattern: Option<String>,

    #[arg(
        long = "merge-strategy",
        help = "How to merge discovered and manual dependencies",
        value_name = "STRATEGY",
        default_value = "discovery-only"
    )]
    merge_strategy: String,

    // External usage checking
    #[arg(
        long = "external-check-dir",
        help = "Directory to check for external usage of SQL functions (can specify multiple times)",
        value_name = "DIR"
    )]
    external_check_dirs: Vec<PathBuf>,

    #[arg(
        long = "external-check-pattern",
        help = "File pattern to check for external usage (e.g., '*.py', can specify multiple times)",
        value_name = "PATTERN"
    )]
    external_check_patterns: Vec<String>,

    // Root Node Configuration
    #[arg(
        long = "root-nodes",
        help = "Specific node names to always treat as roots (can specify multiple times)",
        value_name = "NODE"
    )]
    root_nodes: Vec<String>,

    #[arg(
        long = "root-pattern",
        help = "Glob patterns for root files (e.g., '**/api/*.sql', can specify multiple times)",
        value_name = "PATTERN"
    )]
    root_patterns: Vec<String>,

    #[arg(
        long = "root-regex",
        help = "Regex patterns for root node names (e.g., '^api_.*', can specify multiple times)",
        value_name = "REGEX"
    )]
    root_regex: Vec<String>,

    #[arg(
        long = "root-dir",
        help = "Directories whose files are all roots (can specify multiple times)",
        value_name = "DIR"
    )]
    root_dirs: Vec<PathBuf>,

    #[command(subcommand)]
    command: AnalyzeCommand,
}

#[derive(Debug, Subcommand)]
enum AnalyzeCommand {
    /// Find complete dead branches (subtrees that can be trimmed together)
    DeadBranches,
    /// Find files with no dependencies or dependents
    Orphans,
    /// Find files not required by any other files
    Unrequired,
    /// Find files with dependencies but no dependents
    LeafNodes,
    /// Find files with dependents but no dependencies
    RootNodes,
    /// Detect cycles in the dependency graph
    Cycles,
    /// Find missing dependencies (referenced but non-existent files)
    Missing,
    /// Analyze a specific file in detail
    File {
        #[arg(help = "Path to the file to analyze")]
        path: PathBuf,
    },
}

impl AnalyzeArgs {
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Initialize logging (unless quiet mode)
        if !self.quiet {
            if self.verbose {
                Builder::new()
                    .filter(None, LevelFilter::Debug)
                    .try_init()
                    .ok();
            } else {
                Builder::new()
                    .filter(None, LevelFilter::Info)
                    .try_init()
                    .ok();
            }
        }

        // For cycles and missing commands, we handle graph building specially
        match &self.command {
            AnalyzeCommand::Cycles => return self.analyze_cycles(),
            AnalyzeCommand::Missing => return self.analyze_missing(),
            _ => {}
        }

        // Build the dependency graph (for all other commands)
        let graph = self.build_graph()?;

        // Check for external usage if requested
        let external_checker = if !self.external_check_dirs.is_empty()
            && !self.external_check_patterns.is_empty()
        {
            println!(
                "🔍 Setting up external usage checker for {} directories...",
                self.external_check_dirs.len()
            );
            Some(
                ExternalUsageChecker::new(
                    &self.external_check_dirs,
                    &self.external_check_patterns,
                    !self.verbose,
                )
                .map_err(|e| TopCatError::ConfigError(format!("External checker error: {e}")))?,
            )
        } else {
            None
        };

        // Build root matcher from CLI args and config
        let root_matcher = self.build_root_matcher()?;

        // Execute the requested analysis
        match &self.command {
            AnalyzeCommand::DeadBranches => {
                self.analyze_dead_branches(&graph, external_checker.as_ref(), root_matcher.as_ref())
            }
            AnalyzeCommand::Orphans => self.analyze_orphans(&graph, external_checker.as_ref()),
            AnalyzeCommand::Unrequired => {
                self.analyze_unrequired(&graph, external_checker.as_ref())
            }
            AnalyzeCommand::LeafNodes => self.analyze_leaf_nodes(&graph, external_checker.as_ref()),
            AnalyzeCommand::RootNodes => self.analyze_root_nodes(&graph),
            AnalyzeCommand::File { path } => {
                self.analyze_file(&graph, path, external_checker.as_ref())
            }
            // Cycles and Missing are handled earlier
            AnalyzeCommand::Cycles | AnalyzeCommand::Missing => unreachable!(),
        }
    }

    fn build_graph(&self) -> Result<TCGraph, TopCatError> {
        let sql_discovery = self.load_sql_discovery_config()?;

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

        let fallback_layer = self
            .fallback_layer
            .clone()
            .unwrap_or_else(|| "normal".to_string());

        if !layers.contains(&fallback_layer) {
            return Err(TopCatError::ConfigError(format!(
                "Fallback layer '{fallback_layer}' is not in the layers list: {layers:?}"
            )));
        }

        let config = config::Config {
            input_dirs: self.input_dirs.clone(),
            include_extensions: self.include_file_extensions.as_deref(),
            exclude_extensions: self.exclude_file_extensions.as_deref(),
            include_globs: self.include_globs.as_deref(),
            exclude_globs: self.exclude_globs.as_deref(),
            output: PathBuf::from("/dev/null"), // Not used for analysis
            comment_str: self.comment_str.clone(),
            file_separator_str: String::new(),
            file_end_str: String::new(),
            include_hidden: self.include_hidden_files_and_directories,
            verbose: self.verbose,
            include_node_prefixes: None,
            exclude_node_prefixes: None,
            dry_run: false,
            subdir_filter: None,
            layers,
            fallback_layer,
            sql_discovery,
            header_update_mode: sql_config::HeaderUpdateMode::Never,
            header_output_dir: None,
        };

        let mut graph = TCGraph::new(&config);
        graph.build_graph()?;
        Ok(graph)
    }

    fn load_sql_discovery_config(&self) -> Result<sql_config::SqlDiscoveryConfig, TopCatError> {
        let mut config = if let Some(ref config_path) = self.sql_config_file {
            match sql_config::TopcatConfig::from_file(config_path) {
                Ok(cfg) => cfg.sql_discovery,
                Err(e) => {
                    eprintln!("Warning: Failed to load SQL config file: {e}");
                    sql_config::SqlDiscoveryConfig::default()
                }
            }
        } else {
            sql_config::SqlDiscoveryConfig::default()
        };

        if self.enable_sql_discovery {
            config.enabled = true;
        }

        if let Some(ref pattern) = self.schema_pattern {
            config.schema_pattern = Some(pattern.clone());
        }

        config.merge_strategy = self
            .merge_strategy
            .parse()
            .map_err(|e: String| TopCatError::ConfigError(e))?;

        Ok(config)
    }

    fn build_root_matcher(&self) -> Result<Option<RootNodeMatcher>, TopCatError> {
        // Load from config file if specified
        let mut config_roots = if let Some(ref config_path) = self.sql_config_file {
            let config = sql_config::TopcatConfig::from_file(config_path)
                .map_err(|e| TopCatError::ConfigError(format!("Failed to load config: {e}")))?;
            config.analysis
        } else {
            sql_config::AnalysisConfig::default()
        };

        // Merge CLI args (CLI extends config)
        if !self.root_nodes.is_empty() {
            config_roots.root_nodes.extend(self.root_nodes.clone());
        }
        if !self.root_patterns.is_empty() {
            config_roots
                .root_patterns
                .extend(self.root_patterns.clone());
        }
        if !self.root_regex.is_empty() {
            config_roots.root_regex.extend(self.root_regex.clone());
        }
        if !self.root_dirs.is_empty() {
            config_roots.root_dirs.extend(
                self.root_dirs
                    .iter()
                    .map(|p| p.to_string_lossy().to_string()),
            );
        }

        // Only create matcher if any patterns were specified
        if config_roots.root_nodes.is_empty()
            && config_roots.root_patterns.is_empty()
            && config_roots.root_regex.is_empty()
            && config_roots.root_dirs.is_empty()
        {
            return Ok(None);
        }

        let dirs: Vec<PathBuf> = config_roots.root_dirs.iter().map(PathBuf::from).collect();

        RootNodeMatcher::new(
            config_roots.root_nodes,
            config_roots.root_patterns,
            config_roots.root_regex,
            dirs,
        )
        .map(Some)
        .map_err(TopCatError::ConfigError)
    }

    fn analyze_dead_branches(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
        root_matcher: Option<&RootNodeMatcher>,
    ) -> Result<(), TopCatError> {
        println!("\n🌳 Dead Branches Analysis");
        println!("═══════════════════════════════════════════════════════════\n");

        let mut dead_branches = graph.find_dead_branches(root_matcher);
        let leaf_nodes = graph.find_leaf_nodes();

        // Filter by external usage if checker is provided
        if let Some(checker) = external_checker {
            dead_branches = checker.filter_unused(&dead_branches);
        }

        if dead_branches.is_empty() {
            println!("✅ No dead branches found (all unrequired files are needed)");
            return Ok(());
        }

        let additional_nodes: Vec<_> = dead_branches.difference(&leaf_nodes).collect();

        println!(
            "📊 Found {} node(s) in dead branches:\n",
            dead_branches.len()
        );
        println!("   • Leaf nodes (initial): {}", leaf_nodes.len());
        println!(
            "   • Additional nodes (pulled in): {}",
            additional_nodes.len()
        );
        println!("   • Total nodes in dead branches: {}", dead_branches.len());

        if !additional_nodes.is_empty() {
            println!(
                "\n💡 Benefit: Trimming avoids {} additional deletion iteration(s)",
                additional_nodes.len()
            );
        }

        // Create a table for better formatting
        let mut table = Table::new();
        table.set_header(vec!["Node Name", "Type", "File Path"]);

        // Sort for consistent output
        let mut sorted_branches: Vec<_> = dead_branches.iter().collect();
        sorted_branches.sort();

        for node_name in sorted_branches {
            let all_nodes = graph.get_all_nodes();
            if let Some(node) = all_nodes.iter().find(|n| &n.name == node_name) {
                let node_type = if leaf_nodes.contains(node_name) {
                    "leaf 🍃"
                } else {
                    "branch 🌿"
                };

                table.add_row(vec![
                    Cell::new(&node.name),
                    Cell::new(node_type).fg(if leaf_nodes.contains(node_name) {
                        Color::Green
                    } else {
                        Color::Yellow
                    }),
                    Cell::new(node.path.display()),
                ]);
            }
        }

        println!("\n{table}");

        println!(
            "\n💡 These {} files can all be deleted together in one operation",
            dead_branches.len()
        );
        println!("   Use 'topcat clean dead-branches' to remove them (coming in Phase 3)");

        Ok(())
    }

    fn analyze_orphans(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
    ) -> Result<(), TopCatError> {
        println!("\n🔍 Orphan Files Analysis");
        println!("═══════════════════════════════════════════════════════════\n");

        let mut orphans = graph.find_orphans();

        if let Some(checker) = external_checker {
            orphans = checker.filter_unused(&orphans);
        }

        if orphans.is_empty() {
            println!("✅ No orphaned files found");
            return Ok(());
        }

        println!(
            "📊 Found {} orphaned file(s) with no connections:\n",
            orphans.len()
        );

        let mut table = Table::new();
        table.set_header(vec!["Node Name", "File Path"]);

        let all_nodes = graph.get_all_nodes();
        for orphan in &orphans {
            if let Some(node) = all_nodes.iter().find(|n| &n.name == orphan) {
                table.add_row(vec![Cell::new(&node.name), Cell::new(node.path.display())]);
            }
        }

        println!("{table}");
        println!("\n💡 These files might be safe to delete or could be entry points");

        Ok(())
    }

    fn analyze_unrequired(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
    ) -> Result<(), TopCatError> {
        println!("\n🧹 Unrequired Files Analysis");
        println!("═══════════════════════════════════════════════════════════\n");

        let mut unrequired = graph.find_unrequired();

        if let Some(checker) = external_checker {
            unrequired = checker.filter_unused(&unrequired);
        }

        if unrequired.is_empty() {
            println!("✅ All files are required by at least one other file");
            return Ok(());
        }

        println!(
            "📊 Found {} unrequired file(s) (not needed by any other files):\n",
            unrequired.len()
        );

        let mut table = Table::new();
        table.set_header(vec!["Node Name", "Has Dependencies", "File Path"]);

        let all_nodes = graph.get_all_nodes();
        for name in &unrequired {
            if let Some(node) = all_nodes.iter().find(|n| &n.name == name) {
                table.add_row(vec![
                    Cell::new(&node.name),
                    Cell::new(if node.deps.is_empty() { "No" } else { "Yes" }),
                    Cell::new(node.path.display()),
                ]);
            }
        }

        println!("{table}");

        Ok(())
    }

    fn analyze_leaf_nodes(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
    ) -> Result<(), TopCatError> {
        println!("\n🍃 Leaf Nodes Analysis");
        println!("═══════════════════════════════════════════════════════════\n");

        let mut leaf_nodes = graph.find_leaf_nodes();

        if let Some(checker) = external_checker {
            leaf_nodes = checker.filter_unused(&leaf_nodes);
        }

        if leaf_nodes.is_empty() {
            println!("✅ No leaf nodes found");
            return Ok(());
        }

        println!(
            "📊 Found {} leaf node(s) (have dependencies but no dependents):\n",
            leaf_nodes.len()
        );

        let mut table = Table::new();
        table.set_header(vec!["Node Name", "Dependencies Count", "File Path"]);

        let all_nodes = graph.get_all_nodes();
        for name in &leaf_nodes {
            if let Some(node) = all_nodes.iter().find(|n| &n.name == name) {
                table.add_row(vec![
                    Cell::new(&node.name),
                    Cell::new(node.deps.len()),
                    Cell::new(node.path.display()),
                ]);
            }
        }

        println!("{table}");

        Ok(())
    }

    fn analyze_root_nodes(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        println!("\n🌱 Root Nodes Analysis");
        println!("═══════════════════════════════════════════════════════════\n");

        let root_nodes = graph.find_root_nodes();

        if root_nodes.is_empty() {
            println!("✅ No root nodes found");
            return Ok(());
        }

        println!(
            "📊 Found {} root node(s) (have dependents but no dependencies):\n",
            root_nodes.len()
        );

        let mut table = Table::new();
        table.set_header(vec!["Node Name", "File Path"]);

        let all_nodes = graph.get_all_nodes();
        for name in &root_nodes {
            if let Some(node) = all_nodes.iter().find(|n| &n.name == name) {
                table.add_row(vec![Cell::new(&node.name), Cell::new(node.path.display())]);
            }
        }

        println!("{table}");
        println!("\n💡 These files are entry points in your dependency graph");

        Ok(())
    }

    fn analyze_cycles(&self) -> Result<(), TopCatError> {
        if !self.quiet {
            println!("\n🔄 Cycle Detection Analysis");
            println!("═══════════════════════════════════════════════════════════\n");
        }

        // Try to build the graph - if it has cycles, it will return a CyclicDependency error
        match self.build_graph() {
            Ok(_) => {
                if !self.quiet {
                    println!("✅ No cycles detected in the dependency graph");
                    println!("   The graph is a valid DAG (Directed Acyclic Graph)");
                }
                Ok(())
            }
            Err(TopCatError::CyclicDependency(cycles)) => {
                if !self.quiet {
                    println!(
                        "⚠️  Found {} cycle(s) in the dependency graph:\n",
                        cycles.len()
                    );

                    for (i, cycle) in cycles.iter().enumerate() {
                        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        println!("Cycle #{}", i + 1);
                        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

                        // Show participants
                        println!("Participants:");
                        let mut table = Table::new();
                        table.set_header(vec!["Node Name", "File Path"]);

                        // Deduplicate participants
                        let mut seen = std::collections::HashSet::new();
                        for node in cycle {
                            if seen.insert(&node.name) {
                                table.add_row(vec![
                                    Cell::new(&node.name),
                                    Cell::new(node.path.display()),
                                ]);
                            }
                        }
                        println!("{table}\n");

                        // Show cycle edges
                        println!("Cycle Path:");
                        for (j, node) in cycle.iter().enumerate() {
                            let next_node = &cycle[(j + 1) % cycle.len()];
                            println!("  {} → {}", node.name, next_node.name);
                        }
                        println!();
                    }

                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
                    println!("💡 How to fix cycles:");
                    println!("   1. Remove one of the dependencies in the cycle");
                    println!("   2. Use 'exists' instead of 'requires' for soft dependencies");
                    println!("   3. Restructure code to break circular dependencies");
                    println!("   4. Use layers to enforce ordering between groups");
                }

                // Return error with exit code 1 for scripting
                Err(TopCatError::CyclicDependency(cycles))
            }
            Err(e) => {
                // Other errors
                Err(e)
            }
        }
    }

    fn analyze_missing(&self) -> Result<(), TopCatError> {
        if !self.quiet {
            println!("\n🔍 Missing Dependencies Analysis");
            println!("═══════════════════════════════════════════════════════════\n");
        }

        // Try to build the graph - collect all missing dependency errors
        let mut missing_deps: Vec<(String, String)> = Vec::new();

        // We need to attempt to build and catch the missing dependency errors
        match self.build_graph() {
            Ok(_) => {
                if !self.quiet {
                    println!("✅ No missing dependencies found");
                    println!("   All referenced dependencies exist in the graph");
                }
                Ok(())
            }
            Err(TopCatError::MissingDependency(file, dep)) => {
                // Caught a missing dependency
                missing_deps.push((file, dep));

                if !self.quiet {
                    println!("⚠️  Found missing dependencies:\n");

                    let mut table = Table::new();
                    table.set_header(vec!["File", "Missing Dependency"]);

                    for (file, dep) in &missing_deps {
                        table.add_row(vec![
                            Cell::new(file).fg(Color::Yellow),
                            Cell::new(dep).fg(Color::Red),
                        ]);
                    }

                    println!("{table}\n");
                    println!("💡 These files reference dependencies that don't exist:");
                    println!("   1. Check if the dependency file name is spelled correctly");
                    println!("   2. Verify the dependency file is in the input directory");
                    println!("   3. Consider using 'exists' instead of 'requires' if optional");
                }

                // Return error for scripting (exit code 1)
                Err(TopCatError::MissingDependency(
                    missing_deps[0].0.clone(),
                    missing_deps[0].1.clone(),
                ))
            }
            Err(e) => Err(e),
        }
    }

    fn analyze_file(
        &self,
        graph: &TCGraph,
        path: &PathBuf,
        external_checker: Option<&ExternalUsageChecker>,
    ) -> Result<(), TopCatError> {
        if !self.quiet {
            println!("\n📄 File Analysis: {}", path.display());
            println!("═══════════════════════════════════════════════════════════\n");
        }

        // Find the node in the graph
        let all_nodes = graph.get_all_nodes();
        let target_node = all_nodes.iter().find(|n| n.path == *path).ok_or_else(|| {
            TopCatError::ConfigError(format!("File not found in graph: {}", path.display()))
        })?;

        if !self.quiet {
            println!("Node Name: {}", target_node.name);
            println!("File Path: {}", target_node.path.display());
            println!("Layer: {}", target_node.layer);
            println!();
        }

        // Get dependencies
        if !self.quiet {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Direct Dependencies ({}):", target_node.deps.len());
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        }

        if target_node.deps.is_empty() {
            if !self.quiet {
                println!("  (none)\n");
            }
        } else if !self.quiet {
            let mut table = Table::new();
            table.set_header(vec!["Dependency Name"]);
            for dep in &target_node.deps {
                table.add_row(vec![Cell::new(dep)]);
            }
            println!("{table}\n");
        }

        // Get dependents
        let dependents_map = graph.build_dependents_map();
        let direct_dependents = dependents_map
            .get(&target_node.name)
            .cloned()
            .unwrap_or_default();

        if !self.quiet {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Direct Dependents ({}):", direct_dependents.len());
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        }

        if direct_dependents.is_empty() {
            if !self.quiet {
                println!("  (none)\n");
            }
        } else if !self.quiet {
            let mut table = Table::new();
            table.set_header(vec!["Dependent Name"]);
            for dep in &direct_dependents {
                table.add_row(vec![Cell::new(dep)]);
            }
            println!("{table}\n");
        }

        // Check external usage if available
        if let Some(checker) = external_checker {
            let mut single_node_set = std::collections::HashSet::new();
            single_node_set.insert(target_node.name.clone());
            let externally_used = checker.filter_unused(&single_node_set);

            if !self.quiet {
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("External Usage:");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

                if externally_used.is_empty() {
                    println!("  ❌ Not used by external files\n");
                } else {
                    println!("  ✅ Used by external files\n");
                }
            }
        }

        // Analysis summary
        if !self.quiet {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Summary:");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

            // Determine node type
            let node_type = if target_node.deps.is_empty() && direct_dependents.is_empty() {
                "Orphan (no connections)"
            } else if target_node.deps.is_empty() {
                "Root Node (entry point)"
            } else if direct_dependents.is_empty() {
                "Leaf Node (terminal)"
            } else {
                "Intermediate Node"
            };

            println!("  Node Type: {node_type}");

            // Check if in dead branches
            let unrequired = graph.find_unrequired();
            if unrequired.contains(&target_node.name) {
                println!("  ⚠️  Part of unrequired files (not used by others)");
            } else {
                println!("  ✅ Required by other files");
            }
        }

        Ok(())
    }
}
