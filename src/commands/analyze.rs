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

/// Abstraction for analysis output that respects quiet mode
struct AnalysisLogger {
    quiet: bool,
}

impl AnalysisLogger {
    fn new(quiet: bool) -> Self {
        Self { quiet }
    }

    /// Print a section header with title and separator line
    fn section(&self, title: &str) {
        if !self.quiet {
            println!("\n{title}");
            println!("═══════════════════════════════════════════════════════════\n");
        }
    }

    /// Print a regular info message
    fn info(&self, msg: &str) {
        if !self.quiet {
            println!("{msg}");
        }
    }

    /// Print a table
    fn table(&self, table: &Table) {
        if !self.quiet {
            println!("{table}");
        }
    }

    /// Print a separator line
    fn separator(&self) {
        if !self.quiet {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
    }

    /// Print an empty line
    fn newline(&self) {
        if !self.quiet {
            println!();
        }
    }
}

/// Configuration for displaying analysis results in a table format
struct AnalysisDisplayConfig {
    /// Section title (e.g., "🔍 Orphan Files Analysis")
    title: String,
    /// Message when no results found (e.g., "✅ No orphaned files found")
    empty_message: String,
    /// Summary format with placeholder for count (e.g., "📊 Found {} orphaned file(s)")
    result_summary: String,
    /// Table column headers
    table_headers: Vec<String>,
    /// Optional footer message
    footer_message: Option<String>,
    /// Whether to apply external checker filtering
    apply_external_filter: bool,
}

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

    // Schema Filtering
    #[arg(
        long = "schema",
        help = "Filter analysis to specific schema(s) (can specify multiple times)",
        value_name = "SCHEMA"
    )]
    schema_filter: Vec<String>,

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
    /// Build a HashMap for O(1) node lookups by name
    fn build_node_map(
        nodes: &[topcat::file_node::FileNode],
    ) -> std::collections::HashMap<&str, &topcat::file_node::FileNode> {
        nodes.iter().map(|n| (n.name.as_str(), n)).collect()
    }

    /// Get the platform-specific null device path
    #[cfg(unix)]
    fn null_device() -> &'static str {
        "/dev/null"
    }

    #[cfg(windows)]
    fn null_device() -> &'static str {
        "NUL"
    }

    #[cfg(not(any(unix, windows)))]
    fn null_device() -> &'static str {
        "/dev/null" // Fallback for other platforms
    }

    /// Parse layers from CLI args or use defaults, validate fallback layer
    fn parse_and_validate_layers(&self) -> Result<(Vec<String>, String), TopCatError> {
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

        Ok((layers, fallback_layer))
    }

    /// Convert schema filter CLI args to node prefixes for filtering
    fn build_schema_filter_prefixes(&self) -> Option<Vec<String>> {
        if self.schema_filter.is_empty() {
            return None;
        }

        let mut prefixes = Vec::new();
        for schema in &self.schema_filter {
            prefixes.push(schema.clone()); // For exact match (e.g., "my_schema")
            prefixes.push(format!("{schema}.")); // For prefixed match (e.g., "my_schema.")
        }
        Some(prefixes)
    }

    /// Generic analysis function that handles the common pattern of:
    /// 1. Finding nodes based on criteria
    /// 2. Optionally filtering by external usage
    /// 3. Displaying results in a formatted table
    fn analyze_and_display<F, R>(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
        config: AnalysisDisplayConfig,
        finder: F,
        row_builder: R,
    ) -> Result<(), TopCatError>
    where
        F: Fn(&TCGraph) -> std::collections::HashSet<String>,
        R: Fn(&topcat::file_node::FileNode) -> Vec<Cell>,
    {
        let logger = AnalysisLogger::new(self.quiet);
        logger.section(&config.title);

        let mut results = finder(graph);

        // Apply external filtering if requested
        if config.apply_external_filter {
            if let Some(checker) = external_checker {
                results = checker.filter_unused(&results);
            }
        }

        if results.is_empty() {
            logger.info(&config.empty_message);
            return Ok(());
        }

        logger.info(&format!(
            "{}\n",
            config
                .result_summary
                .replace("{}", &results.len().to_string())
        ));

        let mut table = Table::new();
        table.set_header(
            config
                .table_headers
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        );

        // Sort for deterministic output
        let mut sorted_results: Vec<_> = results.iter().collect();
        sorted_results.sort();

        // Build node map once for O(1) lookups
        let all_nodes = graph.get_all_nodes();
        let node_map = Self::build_node_map(&all_nodes);

        for name in sorted_results {
            if let Some(&node) = node_map.get(name.as_str()) {
                table.add_row(row_builder(node));
            }
        }

        logger.table(&table);

        if let Some(footer) = &config.footer_message {
            logger.info(footer);
        }

        Ok(())
    }

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
        let (layers, fallback_layer) = self.parse_and_validate_layers()?;
        let include_node_prefixes = self.build_schema_filter_prefixes();

        let config = config::Config {
            input_dirs: self.input_dirs.clone(),
            include_extensions: self.include_file_extensions.as_deref(),
            exclude_extensions: self.exclude_file_extensions.as_deref(),
            include_globs: self.include_globs.as_deref(),
            exclude_globs: self.exclude_globs.as_deref(),
            output: PathBuf::from(Self::null_device()), // Platform-specific null device
            comment_str: self.comment_str.clone(),
            file_separator_str: String::new(),
            file_end_str: String::new(),
            include_hidden: self.include_hidden_files_and_directories,
            verbose: self.verbose,
            include_node_prefixes: include_node_prefixes.as_deref(),
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
        let logger = AnalysisLogger::new(self.quiet);
        logger.section("🌳 Dead Branches Analysis");

        let mut dead_branches = graph.find_dead_branches(root_matcher);
        let leaf_nodes = graph.find_leaf_nodes();

        // Filter by external usage if checker is provided
        if let Some(checker) = external_checker {
            dead_branches = checker.filter_unused(&dead_branches);
        }

        if dead_branches.is_empty() {
            logger.info("✅ No dead branches found (all unrequired files are needed)");
            return Ok(());
        }

        let additional_nodes: Vec<_> = dead_branches.difference(&leaf_nodes).collect();

        logger.info(&format!(
            "📊 Found {} node(s) in dead branches:\n",
            dead_branches.len()
        ));
        logger.info(&format!("   • Leaf nodes (initial): {}", leaf_nodes.len()));
        logger.info(&format!(
            "   • Additional nodes (pulled in): {}",
            additional_nodes.len()
        ));
        logger.info(&format!(
            "   • Total nodes in dead branches: {}",
            dead_branches.len()
        ));

        if !additional_nodes.is_empty() {
            logger.info(&format!(
                "\n💡 Benefit: Trimming avoids {} additional deletion iteration(s)",
                additional_nodes.len()
            ));
        }

        // Create a table for better formatting
        let mut table = Table::new();
        table.set_header(vec!["Node Name", "Type", "File Path"]);

        // Sort for consistent output
        let mut sorted_branches: Vec<_> = dead_branches.iter().collect();
        sorted_branches.sort();

        // Build node map once for O(1) lookups
        let all_nodes = graph.get_all_nodes();
        let node_map = Self::build_node_map(&all_nodes);

        for node_name in sorted_branches {
            if let Some(&node) = node_map.get(node_name.as_str()) {
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

        logger.info("");
        logger.table(&table);

        logger.info(&format!(
            "\n💡 These {} files can all be deleted together in one operation",
            dead_branches.len()
        ));
        logger.info("   Use 'topcat clean dead-branches' to remove them (coming in Phase 3)");

        Ok(())
    }

    fn analyze_orphans(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
    ) -> Result<(), TopCatError> {
        let config = AnalysisDisplayConfig {
            title: "🔍 Orphan Files Analysis".to_string(),
            empty_message: "✅ No orphaned files found".to_string(),
            result_summary: "📊 Found {} orphaned file(s) with no connections:".to_string(),
            table_headers: vec!["Node Name".to_string(), "File Path".to_string()],
            footer_message: Some(
                "\n💡 These files might be safe to delete or could be entry points".to_string(),
            ),
            apply_external_filter: true,
        };

        self.analyze_and_display(
            graph,
            external_checker,
            config,
            |g| g.find_orphans(),
            |node| vec![Cell::new(&node.name), Cell::new(node.path.display())],
        )
    }

    fn analyze_unrequired(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
    ) -> Result<(), TopCatError> {
        let config = AnalysisDisplayConfig {
            title: "🧹 Unrequired Files Analysis".to_string(),
            empty_message: "✅ All files are required by at least one other file".to_string(),
            result_summary: "📊 Found {} unrequired file(s) (not needed by any other files):"
                .to_string(),
            table_headers: vec![
                "Node Name".to_string(),
                "Has Dependencies".to_string(),
                "File Path".to_string(),
            ],
            footer_message: None,
            apply_external_filter: true,
        };

        self.analyze_and_display(
            graph,
            external_checker,
            config,
            |g| g.find_unrequired(),
            |node| {
                vec![
                    Cell::new(&node.name),
                    Cell::new(if node.deps.is_empty() { "No" } else { "Yes" }),
                    Cell::new(node.path.display()),
                ]
            },
        )
    }

    fn analyze_leaf_nodes(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
    ) -> Result<(), TopCatError> {
        let config = AnalysisDisplayConfig {
            title: "🍃 Leaf Nodes Analysis".to_string(),
            empty_message: "✅ No leaf nodes found".to_string(),
            result_summary: "📊 Found {} leaf node(s) (have dependencies but no dependents):"
                .to_string(),
            table_headers: vec![
                "Node Name".to_string(),
                "Dependencies Count".to_string(),
                "File Path".to_string(),
            ],
            footer_message: None,
            apply_external_filter: true,
        };

        self.analyze_and_display(
            graph,
            external_checker,
            config,
            |g| g.find_leaf_nodes(),
            |node| {
                vec![
                    Cell::new(&node.name),
                    Cell::new(node.deps.len()),
                    Cell::new(node.path.display()),
                ]
            },
        )
    }

    fn analyze_root_nodes(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let config = AnalysisDisplayConfig {
            title: "🌱 Root Nodes Analysis".to_string(),
            empty_message: "✅ No root nodes found".to_string(),
            result_summary: "📊 Found {} root node(s) (have dependents but no dependencies):"
                .to_string(),
            table_headers: vec!["Node Name".to_string(), "File Path".to_string()],
            footer_message: Some(
                "\n💡 These files are entry points in your dependency graph".to_string(),
            ),
            apply_external_filter: false,
        };

        self.analyze_and_display(
            graph,
            None,
            config,
            |g| g.find_root_nodes(),
            |node| vec![Cell::new(&node.name), Cell::new(node.path.display())],
        )
    }

    fn analyze_cycles(&self) -> Result<(), TopCatError> {
        let logger = AnalysisLogger::new(self.quiet);
        logger.section("🔄 Cycle Detection Analysis");

        // Try to build the graph - if it has cycles, it will return a CyclicDependency error
        match self.build_graph() {
            Ok(_) => {
                logger.info("✅ No cycles detected in the dependency graph");
                logger.info("   The graph is a valid DAG (Directed Acyclic Graph)");
                Ok(())
            }
            Err(TopCatError::CyclicDependency(cycles)) => {
                logger.info(&format!(
                    "⚠️  Found {} cycle(s) in the dependency graph:\n",
                    cycles.len()
                ));

                for (i, cycle) in cycles.iter().enumerate() {
                    logger.separator();
                    logger.info(&format!("Cycle #{}", i + 1));
                    logger.separator();
                    logger.newline();

                    // Show participants
                    logger.info("Participants:");
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
                    logger.table(&table);
                    logger.newline();

                    // Show cycle edges
                    logger.info("Cycle Path:");
                    for (j, node) in cycle.iter().enumerate() {
                        let next_node = &cycle[(j + 1) % cycle.len()];
                        logger.info(&format!("  {} → {}", node.name, next_node.name));
                    }
                    logger.newline();
                }

                logger.separator();
                logger.newline();
                logger.info("💡 How to fix cycles:");
                logger.info("   1. Remove one of the dependencies in the cycle");
                logger.info("   2. Use 'exists' instead of 'requires' for soft dependencies");
                logger.info("   3. Restructure code to break circular dependencies");
                logger.info("   4. Use layers to enforce ordering between groups");

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
        let logger = AnalysisLogger::new(self.quiet);
        logger.section("🔍 Missing Dependencies Analysis");

        // Build a minimal config for validation (same as build_graph but for validation only)
        let sql_discovery = self.load_sql_discovery_config()?;
        let (layers, fallback_layer) = self.parse_and_validate_layers()?;
        let include_node_prefixes = self.build_schema_filter_prefixes();

        let config = config::Config {
            input_dirs: self.input_dirs.clone(),
            include_extensions: self.include_file_extensions.as_deref(),
            exclude_extensions: self.exclude_file_extensions.as_deref(),
            include_globs: self.include_globs.as_deref(),
            exclude_globs: self.exclude_globs.as_deref(),
            output: PathBuf::from(Self::null_device()),
            comment_str: self.comment_str.clone(),
            file_separator_str: String::new(),
            file_end_str: String::new(),
            include_hidden: self.include_hidden_files_and_directories,
            verbose: self.verbose,
            include_node_prefixes: include_node_prefixes.as_deref(),
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

        // Use the new validate_dependencies_only method to get ALL missing dependencies
        let missing_deps = graph.validate_dependencies_only()?;

        if missing_deps.is_empty() {
            logger.info("✅ No missing dependencies found");
            logger.info("   All referenced dependencies exist in the graph");
            return Ok(());
        }

        logger.info(&format!(
            "⚠️  Found {} missing dependencies:\n",
            missing_deps.len()
        ));

        let mut table = Table::new();
        table.set_header(vec!["File", "Missing Dependency"]);

        // Sort for deterministic output
        let mut sorted_deps = missing_deps.clone();
        sorted_deps.sort();

        for (file, dep) in &sorted_deps {
            table.add_row(vec![
                Cell::new(file).fg(Color::Yellow),
                Cell::new(dep).fg(Color::Red),
            ]);
        }

        logger.table(&table);
        logger.newline();
        logger.info("💡 These files reference dependencies that don't exist:");
        logger.info("   1. Check if the dependency file name is spelled correctly");
        logger.info("   2. Verify the dependency file is in the input directory");
        logger.info("   3. Consider using 'exists' instead of 'requires' if optional");

        // Return error for scripting (exit code 1)
        Err(TopCatError::MissingDependency(
            format!("{} files with missing dependencies", sorted_deps.len()),
            format!("{} total missing", sorted_deps.len()),
        ))
    }

    fn analyze_file(
        &self,
        graph: &TCGraph,
        path: &PathBuf,
        external_checker: Option<&ExternalUsageChecker>,
    ) -> Result<(), TopCatError> {
        let logger = AnalysisLogger::new(self.quiet);
        logger.section(&format!("📄 File Analysis: {}", path.display()));

        // Find the node in the graph
        let all_nodes = graph.get_all_nodes();
        let target_node = all_nodes.iter().find(|n| n.path == *path).ok_or_else(|| {
            TopCatError::ConfigError(format!("File not found in graph: {}", path.display()))
        })?;

        logger.info(&format!("Node Name: {}", target_node.name));
        logger.info(&format!("File Path: {}", target_node.path.display()));
        logger.info(&format!("Layer: {}", target_node.layer));
        logger.newline();

        // Get dependencies
        logger.separator();
        logger.info(&format!(
            "Direct Dependencies ({}):",
            target_node.deps.len()
        ));
        logger.separator();
        logger.newline();

        if target_node.deps.is_empty() {
            logger.info("  (none)\n");
        } else {
            let mut table = Table::new();
            table.set_header(vec!["Dependency Name"]);
            for dep in &target_node.deps {
                table.add_row(vec![Cell::new(dep)]);
            }
            logger.table(&table);
            logger.newline();
        }

        // Get dependents
        let dependents_map = graph.build_dependents_map();
        let direct_dependents = dependents_map
            .get(&target_node.name)
            .cloned()
            .unwrap_or_default();

        logger.separator();
        logger.info(&format!("Direct Dependents ({}):", direct_dependents.len()));
        logger.separator();
        logger.newline();

        if direct_dependents.is_empty() {
            logger.info("  (none)\n");
        } else {
            let mut table = Table::new();
            table.set_header(vec!["Dependent Name"]);
            for dep in &direct_dependents {
                table.add_row(vec![Cell::new(dep)]);
            }
            logger.table(&table);
            logger.newline();
        }

        // Check external usage if available
        if let Some(checker) = external_checker {
            let mut single_node_set = std::collections::HashSet::new();
            single_node_set.insert(target_node.name.clone());
            let externally_used = checker.filter_unused(&single_node_set);

            logger.separator();
            logger.info("External Usage:");
            logger.separator();
            logger.newline();

            if externally_used.is_empty() {
                logger.info("  ❌ Not used by external files\n");
            } else {
                logger.info("  ✅ Used by external files\n");
            }
        }

        // Analysis summary
        logger.separator();
        logger.info("Summary:");
        logger.separator();
        logger.newline();

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

        logger.info(&format!("  Node Type: {node_type}"));

        // Check if in dead branches
        let unrequired = graph.find_unrequired();
        if unrequired.contains(&target_node.name) {
            logger.info("  ⚠️  Part of unrequired files (not used by others)");
        } else {
            logger.info("  ✅ Required by other files");
        }

        Ok(())
    }
}
