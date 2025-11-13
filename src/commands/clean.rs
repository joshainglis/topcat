use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Args, Subcommand};
use comfy_table::{Cell, Color, Table};
use env_logger::Builder;
use log::LevelFilter;

use topcat::analysis::GraphAnalyzer;
use topcat::analysis::external_usage::ExternalUsageChecker;
use topcat::analysis::root_matcher::RootNodeMatcher;
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;

use super::common;

/// Remove unused files based on dependency analysis
#[derive(Debug, Args)]
pub struct CleanArgs {
    #[arg(
        short = 'i',
        long = "input-dirs",
        help = "Paths to directories containing files to clean",
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
        help = "Directory to check for external usage before deletion (can specify multiple times)",
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
        help = "Specific node names to protect from deletion (can specify multiple times)",
        value_name = "NODE"
    )]
    root_nodes: Vec<String>,

    #[arg(
        long = "root-pattern",
        help = "Glob patterns for protected files (e.g., '**/api/*.sql', can specify multiple times)",
        value_name = "PATTERN"
    )]
    root_patterns: Vec<String>,

    #[arg(
        long = "root-regex",
        help = "Regex patterns for protected node names (e.g., '^api_.*', can specify multiple times)",
        value_name = "REGEX"
    )]
    root_regex: Vec<String>,

    #[arg(
        long = "root-dir",
        help = "Directories whose files are protected (can specify multiple times)",
        value_name = "DIR"
    )]
    root_dirs: Vec<PathBuf>,

    // Schema Filtering
    #[arg(
        long = "schema",
        help = "Filter cleaning to specific schema(s) (can specify multiple times)",
        value_name = "SCHEMA"
    )]
    schema_filter: Vec<String>,

    // Clean-specific flags
    #[arg(
        long = "dry-run",
        help = "Show what would be deleted without actually deleting",
        default_value = "true"
    )]
    dry_run: bool,

    #[arg(
        long = "no-dry-run",
        help = "Actually perform deletion (overrides --dry-run)",
        conflicts_with = "dry_run"
    )]
    no_dry_run: bool,

    #[arg(
        long = "force",
        short = 'f',
        help = "Skip confirmation prompt (for automation)"
    )]
    force: bool,

    #[command(subcommand)]
    command: CleanCommand,
}

#[derive(Debug, Subcommand)]
enum CleanCommand {
    /// Remove complete dead branches (subtrees that can be trimmed together)
    DeadBranches,
    /// Remove files with no dependencies or dependents
    Orphans,
    /// Remove files not required by any other files
    Unrequired,
    /// Remove specific target files (with dependency checking)
    Targets {
        #[arg(help = "File paths or patterns to remove")]
        files: Vec<String>,
    },
}

impl CleanArgs {
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Initialize logging
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

        // Determine if we're actually deleting or just previewing
        let actually_delete = self.no_dry_run || !self.dry_run;

        if actually_delete {
            println!("⚠️  DELETION MODE: Files will be permanently removed!");
        } else {
            println!("🔍 DRY-RUN MODE: No files will be deleted");
        }

        // Build the dependency graph
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

        // Execute the requested cleanup
        match &self.command {
            CleanCommand::DeadBranches => self.clean_dead_branches(
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
            ),
            CleanCommand::Orphans => self.clean_orphans(
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
            ),
            CleanCommand::Unrequired => self.clean_unrequired(
                &graph,
                external_checker.as_ref(),
                root_matcher.as_ref(),
                actually_delete,
            ),
            CleanCommand::Targets { files } => {
                self.clean_targets(&graph, files, root_matcher.as_ref(), actually_delete)
            }
        }
    }

    fn build_graph(&self) -> Result<TCGraph, TopCatError> {
        let sql_discovery = common::load_sql_discovery_config(
            &self.sql_config_file,
            self.enable_sql_discovery,
            &self.schema_pattern,
            &self.merge_strategy,
        )?;
        let (layers, fallback_layer) =
            common::parse_and_validate_layers(&self.layers, &self.fallback_layer)?;
        let include_node_prefixes = common::build_schema_filter_prefixes(&self.schema_filter);

        common::build_graph(
            self.input_dirs.clone(),
            self.include_file_extensions.as_deref(),
            self.exclude_file_extensions.as_deref(),
            self.include_globs.as_deref(),
            self.exclude_globs.as_deref(),
            self.include_hidden_files_and_directories,
            self.verbose,
            self.comment_str.clone(),
            layers,
            fallback_layer,
            sql_discovery,
            include_node_prefixes,
        )
    }

    fn build_root_matcher(&self) -> Result<Option<RootNodeMatcher>, TopCatError> {
        common::build_root_matcher(
            &self.sql_config_file,
            self.root_nodes.clone(),
            self.root_patterns.clone(),
            self.root_regex.clone(),
            self.root_dirs.clone(),
        )
    }

    fn clean_dead_branches(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
        root_matcher: Option<&RootNodeMatcher>,
        actually_delete: bool,
    ) -> Result<(), TopCatError> {
        println!("\n🌳 Finding dead branches...\n");

        // Find dead branches
        let mut dead_branches = graph.find_dead_branches(root_matcher);

        // Filter out externally used files
        if let Some(checker) = external_checker {
            let before_count = dead_branches.len();
            dead_branches = checker.filter_unused(&dead_branches);
            let filtered_count = before_count - dead_branches.len();
            if filtered_count > 0 {
                println!("✅ Filtered out {filtered_count} file(s) with external usage\n");
            }
        }

        if dead_branches.is_empty() {
            println!("✅ No dead branches found! Your codebase is clean.");
            return Ok(());
        }

        // Show what will be deleted
        self.show_deletion_preview(graph, &dead_branches, "Dead Branches")?;

        // Delete files if not dry-run
        if actually_delete {
            self.perform_deletion(graph, &dead_branches)?;
        } else {
            println!("\n💡 Run with --no-dry-run to actually delete these files");
        }

        Ok(())
    }

    fn clean_orphans(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
        root_matcher: Option<&RootNodeMatcher>,
        actually_delete: bool,
    ) -> Result<(), TopCatError> {
        println!("\n🌿 Finding orphan files...\n");

        // Find orphans
        let mut orphans = graph.find_orphans();

        // Filter out root nodes if specified
        if let Some(matcher) = root_matcher {
            let node_to_path = graph.build_node_to_path_map();
            let before_count = orphans.len();
            orphans.retain(|node| {
                if let Some(path) = node_to_path.get(node) {
                    !matcher.is_root(node, path)
                } else {
                    true
                }
            });
            let filtered_count = before_count - orphans.len();
            if filtered_count > 0 {
                println!("🔒 Protected {filtered_count} root node(s) from deletion");
            }
        }

        // Filter out externally used files
        if let Some(checker) = external_checker {
            let before_count = orphans.len();
            orphans = checker.filter_unused(&orphans);
            let filtered_count = before_count - orphans.len();
            if filtered_count > 0 {
                println!("✅ Filtered out {filtered_count} file(s) with external usage\n");
            }
        }

        if orphans.is_empty() {
            println!("✅ No orphan files found!");
            return Ok(());
        }

        // Show what will be deleted
        self.show_deletion_preview(graph, &orphans, "Orphan Files")?;

        // Delete files if not dry-run
        if actually_delete {
            self.perform_deletion(graph, &orphans)?;
        } else {
            println!("\n💡 Run with --no-dry-run to actually delete these files");
        }

        Ok(())
    }

    fn clean_unrequired(
        &self,
        graph: &TCGraph,
        external_checker: Option<&ExternalUsageChecker>,
        root_matcher: Option<&RootNodeMatcher>,
        actually_delete: bool,
    ) -> Result<(), TopCatError> {
        println!("\n🍃 Finding unrequired files...\n");

        // Find unrequired
        let mut unrequired = graph.find_unrequired();

        // Filter out root nodes if specified
        if let Some(matcher) = root_matcher {
            let node_to_path = graph.build_node_to_path_map();
            let before_count = unrequired.len();
            unrequired.retain(|node| {
                if let Some(path) = node_to_path.get(node) {
                    !matcher.is_root(node, path)
                } else {
                    true
                }
            });
            let filtered_count = before_count - unrequired.len();
            if filtered_count > 0 {
                println!("🔒 Protected {filtered_count} root node(s) from deletion");
            }
        }

        // Filter out externally used files
        if let Some(checker) = external_checker {
            let before_count = unrequired.len();
            unrequired = checker.filter_unused(&unrequired);
            let filtered_count = before_count - unrequired.len();
            if filtered_count > 0 {
                println!("✅ Filtered out {filtered_count} file(s) with external usage\n");
            }
        }

        if unrequired.is_empty() {
            println!("✅ No unrequired files found!");
            return Ok(());
        }

        // Show what will be deleted
        self.show_deletion_preview(graph, &unrequired, "Unrequired Files")?;

        // Delete files if not dry-run
        if actually_delete {
            self.perform_deletion(graph, &unrequired)?;
        } else {
            println!("\n💡 Run with --no-dry-run to actually delete these files");
        }

        Ok(())
    }

    fn clean_targets(
        &self,
        graph: &TCGraph,
        target_files: &[String],
        root_matcher: Option<&RootNodeMatcher>,
        actually_delete: bool,
    ) -> Result<(), TopCatError> {
        println!("\n🎯 Processing target files for deletion...\n");

        if target_files.is_empty() {
            return Err(TopCatError::ConfigError(
                "No target files specified".to_string(),
            ));
        }

        // Build a map of node names to check
        let node_to_path = graph.build_node_to_path_map();
        let mut targets_to_delete = std::collections::HashSet::new();

        // Match target patterns to actual nodes
        for pattern in target_files {
            let mut found_match = false;
            for (node_name, path) in &node_to_path {
                // Check if the pattern matches the node name or file path
                if node_name.contains(pattern) || path.to_string_lossy().contains(pattern) {
                    targets_to_delete.insert(node_name.clone());
                    found_match = true;
                }
            }
            if !found_match {
                eprintln!("⚠️  Warning: No files matched pattern '{pattern}'");
            }
        }

        if targets_to_delete.is_empty() {
            println!("❌ No files matched the specified patterns");
            return Ok(());
        }

        // Filter out root nodes if specified
        if let Some(matcher) = root_matcher {
            let before_count = targets_to_delete.len();
            targets_to_delete.retain(|node| {
                if let Some(path) = node_to_path.get(node) {
                    !matcher.is_root(node, path)
                } else {
                    true
                }
            });
            let filtered_count = before_count - targets_to_delete.len();
            if filtered_count > 0 {
                println!("🔒 Protected {filtered_count} root node(s) from deletion");
            }
        }

        // Check for dependents that would break
        let dependents_map = graph.build_dependents_map();
        let mut has_dependents = false;
        let mut dependent_warnings = Vec::new();

        for target in &targets_to_delete {
            if let Some(dependents) = dependents_map.get(target) {
                if !dependents.is_empty() {
                    has_dependents = true;
                    dependent_warnings.push((target.clone(), dependents.clone()));
                }
            }
        }

        if has_dependents {
            println!("⚠️  WARNING: Some target files have dependents!\n");
            for (target, dependents) in &dependent_warnings {
                println!("  {target} is required by:");
                for dep in dependents {
                    println!("    - {dep}");
                }
                println!();
            }
            println!("❌ Cannot delete files that are still required by others");
            println!("💡 Tip: Use 'clean dead-branches' to remove entire unused subtrees\n");
            return Ok(());
        }

        // Show what will be deleted
        self.show_deletion_preview(graph, &targets_to_delete, "Target Files")?;

        // Delete files if not dry-run
        if actually_delete {
            self.perform_deletion(graph, &targets_to_delete)?;
        } else {
            println!("\n💡 Run with --no-dry-run to actually delete these files");
        }

        Ok(())
    }

    fn show_deletion_preview(
        &self,
        graph: &TCGraph,
        nodes_to_delete: &std::collections::HashSet<String>,
        category: &str,
    ) -> Result<(), TopCatError> {
        let node_to_path = graph.build_node_to_path_map();

        let mut table = Table::new();
        table.set_header(vec!["Node Name", "File Path"]);

        let mut paths = Vec::new();
        for node in nodes_to_delete {
            if let Some(path) = node_to_path.get(node) {
                paths.push((node.clone(), path.clone()));
            }
        }

        // Sort for consistent output
        paths.sort_by(|a, b| a.1.cmp(&b.1));

        for (node, path) in &paths {
            table.add_row(vec![
                Cell::new(node).fg(Color::Red),
                Cell::new(path.display()).fg(Color::Red),
            ]);
        }

        println!(
            "📋 {category} to be deleted ({} files):\n",
            nodes_to_delete.len()
        );
        println!("{table}\n");

        Ok(())
    }

    fn perform_deletion(
        &self,
        graph: &TCGraph,
        nodes_to_delete: &std::collections::HashSet<String>,
    ) -> Result<(), TopCatError> {
        if nodes_to_delete.is_empty() {
            return Ok(());
        }

        // Ask for confirmation unless --force is set
        if !self.force {
            print!(
                "\n⚠️  Are you sure you want to delete {} files? [y/N]: ",
                nodes_to_delete.len()
            );
            io::stdout().flush().unwrap();

            let mut response = String::new();
            io::stdin().read_line(&mut response).unwrap();
            let response = response.trim().to_lowercase();

            if response != "y" && response != "yes" {
                println!("❌ Deletion cancelled");
                return Ok(());
            }
        }

        println!("\n🗑️  Deleting files...\n");

        let node_to_path = graph.build_node_to_path_map();
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut errors = Vec::new();

        for node in nodes_to_delete {
            if let Some(path) = node_to_path.get(node) {
                match std::fs::remove_file(path) {
                    Ok(_) => {
                        success_count += 1;
                        if self.verbose {
                            println!("✅ Deleted: {}", path.display());
                        }
                    }
                    Err(e) => {
                        failure_count += 1;
                        let error_msg = format!("Failed to delete {}: {e}", path.display());
                        eprintln!("❌ {error_msg}");
                        errors.push(error_msg);
                    }
                }
            }
        }

        // Show summary
        println!("\n📊 Deletion Summary:");
        println!("  ✅ Successfully deleted: {success_count} files");
        if failure_count > 0 {
            println!("  ❌ Failed to delete: {failure_count} files");
        }

        if failure_count > 0 {
            Err(TopCatError::Io(std::io::Error::other(format!(
                "Failed to delete {failure_count} file(s)"
            ))))
        } else {
            println!("\n✅ All files deleted successfully!");
            Ok(())
        }
    }
}
