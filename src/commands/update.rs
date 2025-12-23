//! Update command for discovering dependencies and updating file headers.
//!
//! This module implements the `update` subcommand which analyzes SQL files,
//! discovers dependencies, and updates file headers accordingly. It can also
//! rename files based on discovered node names.
//!
//! # Smart Defaults
//!
//! When SQL-like extensions are specified (`sql`, `pg`, `psql`, `ddl`, `pgsql`),
//! the update command automatically enables:
//! - SQL discovery (`--sql-discovery`)
//! - In-place header updates (`--update-headers`)
//! - File renaming based on discovered names (`--rename-files`)
//!
//! These can be explicitly disabled with `--no-sql-discovery`,
//! `--no-update-headers`, or `--no-rename-files`.
//!
//! # Examples
//!
//! ```bash
//! # Preview changes (dry-run by default)
//! topcat update -i sql/ -e sql
//!
//! # Apply changes
//! topcat update -i sql/ -e sql --mode execute
//!
//! # Generate to separate directory instead of in-place
//! topcat update -i sql/ -e sql --generate-headers ./updated/
//!
//! # Disable specific defaults
//! topcat update -i sql/ -e sql --no-rename-files         # Keep original filenames
//! topcat update -i sql/ -e sql --no-sql-discovery        # Header-only parsing
//! ```

use clap::Args;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

use topcat::{
    cli::{ExecutionArgs, GlobalArgs, GraphInputArgs, SqlDiscoveryArgs},
    config,
    exceptions::{FileNodeError, TopCatError},
    file_node::{FileNode, NameSource},
    header_generator, io_utils,
    layer_mapper::LayerMapper,
    logging::{Logger, init_logging},
    settings::Settings,
    soft_deps_matcher::SoftDepsMapper,
    sql_config::{self, MergeStrategy},
    sql_parser::SqlAnalyzer,
};

/// Command-line arguments for the update subcommand.
///
/// The update command uses:
/// - GlobalArgs: config, verbose, quiet
/// - GraphInputArgs: input directories, file filtering, layers
/// - SqlDiscoveryArgs: SQL discovery settings, header update mode
/// - ExecutionArgs: mode (dry-run/execute)
#[derive(Debug, Args, Clone)]
pub struct UpdateArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(flatten)]
    pub input: GraphInputArgs,

    #[command(flatten)]
    pub sql_discovery: SqlDiscoveryArgs,

    #[command(flatten)]
    pub execution: ExecutionArgs,
}

impl UpdateArgs {
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Load settings from config files and environment variables
        let config_path = self.global.config_path();
        let mut settings = Settings::load(config_path)
            .map_err(|e| TopCatError::ConfigError(format!("Failed to load configuration: {e}")))?;

        // Apply CLI overrides
        self.global.apply_to_settings(&mut settings);
        self.input.apply_to_settings(&mut settings);
        self.sql_discovery.apply_to_settings(&mut settings);
        self.execution.apply_to_settings(&mut settings);

        // Smart defaults for SQL files: enable discovery and in-place updates
        let sql_extensions = ["sql", "pg", "psql", "ddl", "pgsql"];
        let has_sql_ext = settings
            .filters
            .include_extensions
            .iter()
            .any(|ext| sql_extensions.contains(&ext.to_lowercase().as_str()));

        if has_sql_ext {
            // Default: enable SQL discovery unless explicitly set via CLI or config
            if self.sql_discovery.effective_sql_discovery().is_none()
                && !settings.sql_discovery.enabled
            {
                settings.sql_discovery.enabled = true;
            }

            // Default: update headers in-place unless explicitly set otherwise
            if self.sql_discovery.effective_update_headers().is_none()
                && self.sql_discovery.generate_headers_dir.is_none()
                && settings.header_update_mode == sql_config::HeaderUpdateMode::Never
            {
                settings.header_update_mode = sql_config::HeaderUpdateMode::InPlace;
            }

            // Default: rename files based on discovered node names
            if self.sql_discovery.effective_rename_files().is_none() && !settings.rename_files {
                settings.rename_files = true;
            }
        }

        // Validate settings
        settings.validate().map_err(TopCatError::ConfigError)?;

        // Ensure required fields are set
        if settings.input_dirs.is_empty() {
            return Err(TopCatError::ConfigError(
                "At least one input directory must be specified via -i/--input-dirs or config file"
                    .to_string(),
            ));
        }

        // For update command, we need either update-headers or generate-headers to be set
        if settings.header_update_mode == sql_config::HeaderUpdateMode::Never {
            return Err(TopCatError::ConfigError(
                "Header update mode must be specified. Use --update-headers to update files in-place, or --generate-headers DIR to write to a directory".to_string(),
            ));
        }

        // Note: rename_files validation is now handled by Settings.validate()

        // Initialize logging
        let quiet = settings.behavior.quiet;
        let verbose = settings.behavior.verbose;
        init_logging(verbose, quiet);
        let logger = Logger::new(quiet, verbose);

        // Show what we're doing
        let dry_run = self.execution.mode.is_dry_run();
        if dry_run {
            logger.info("DRY RUN MODE: No files will be modified");
        }

        if settings.rename_files {
            logger.info("File renaming enabled based on discovered node names");
        }

        // Create Config directly from Settings
        let fallback_layer = settings.layers.fallback.clone();

        // For update command, we don't need an output file, so use a dummy path
        let dummy_output = std::path::PathBuf::from("/dev/null");

        // Create Config struct with borrowed slices from Settings
        let config = config::Config {
            input_dirs: settings.input_dirs.clone(),
            include_globs: if settings.filters.include_globs.is_empty() {
                None
            } else {
                Some(&settings.filters.include_globs)
            },
            exclude_globs: if settings.filters.exclude_globs.is_empty() {
                None
            } else {
                Some(&settings.filters.exclude_globs)
            },
            include_extensions: if settings.filters.include_extensions.is_empty() {
                None
            } else {
                Some(&settings.filters.include_extensions)
            },
            exclude_extensions: if settings.filters.exclude_extensions.is_empty() {
                None
            } else {
                Some(&settings.filters.exclude_extensions)
            },
            output: dummy_output,
            comment_str: settings.formatting.comment_str.clone(),
            file_separator_str: settings.formatting.file_separator_str.clone(),
            file_end_str: settings.formatting.file_end_str.clone(),
            verbose: settings.behavior.verbose,
            dry_run,
            include_node_prefixes: if settings.node_filtering.include_prefixes.is_empty() {
                None
            } else {
                Some(&settings.node_filtering.include_prefixes)
            },
            exclude_node_prefixes: if settings.node_filtering.exclude_prefixes.is_empty() {
                None
            } else {
                Some(&settings.node_filtering.exclude_prefixes)
            },
            include_hidden: settings.filters.include_hidden,
            subdir_filter: settings.node_filtering.subdir_filter.clone(),
            layers: settings.layers.names.clone(),
            fallback_layer,
            auto_mapping: &settings.layers.auto_mapping,
            sql_discovery: settings.sql_discovery.clone(),
            header_update_mode: settings.header_update_mode,
            header_output_dir: settings.header_output_dir.clone(),
        };

        // For update command, we don't build a graph - just discover dependencies per file
        // This matches the behavior of the Python lint_nodes.py script
        logger.info("Discovering dependencies from SQL files...");

        let file_nodes = discover_files_without_graph(&config)?;

        if settings.behavior.verbose {
            logger.debug(&format!("Discovered {} files", file_nodes.len()));
        }

        // Update headers
        if dry_run {
            logger.info("Previewing header updates (dry-run mode)...");
        } else {
            logger.info("Updating file headers...");
        }

        // Count how many files will be updated
        let update_count = file_nodes
            .iter()
            .filter(|n| n.discovered_deps.is_some())
            .count();

        if update_count == 0 {
            if settings.sql_discovery.enabled {
                logger.info("No dependencies discovered in any files.");
            } else {
                logger.info(
                    "No files with discovered dependencies found. SQL discovery is disabled.",
                );
            }
            return Ok(());
        }

        logger.info(&format!(
            "Found {update_count} file(s) with discovered dependencies"
        ));

        // Determine default extension from filters config, or use "sql" as fallback
        let default_extension = settings
            .filters
            .include_extensions
            .first()
            .map(|s| s.as_str())
            .unwrap_or("sql");

        // Preview or execute header updates
        if dry_run {
            preview_header_updates(
                &file_nodes,
                &settings.formatting.comment_str,
                settings.rename_files,
                default_extension,
                &logger,
            )?;
        } else {
            header_generator::update_headers(
                &file_nodes,
                &settings.formatting.comment_str,
                settings.header_update_mode,
                settings.header_output_dir.as_deref(),
                settings.rename_files,
                default_extension,
            )?;

            match settings.header_update_mode {
                sql_config::HeaderUpdateMode::InPlace => {
                    logger.success("Headers updated successfully!");
                }
                sql_config::HeaderUpdateMode::Generate => {
                    let output_dir = settings
                        .header_output_dir
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    logger.success(&format!("Updated files written to: {output_dir}"));
                }
                sql_config::HeaderUpdateMode::Never => {
                    // This shouldn't happen due to earlier validation
                    unreachable!()
                }
            }
        }

        Ok(())
    }
}

/// Collect and filter files from input directories
fn collect_and_filter_files(config: &config::Config) -> Result<HashSet<PathBuf>, TopCatError> {
    // Collect all files from input directories
    let mut all_files = HashSet::new();
    for dir in &config.input_dirs {
        let dir_files = io_utils::walk_dir(dir, config.include_hidden).map_err(|e| {
            TopCatError::config_error(format!("Failed to walk directory {}: {}", dir.display(), e))
        })?;
        all_files.extend(dir_files);
    }

    // Apply include/exclude glob filters
    let include_globs: Option<HashSet<PathBuf>> = config
        .include_globs
        .map(io_utils::glob_files)
        .transpose()
        .map_err(|e| TopCatError::config_error(format!("Failed to apply include globs: {e}")))?;

    let exclude_globs: Option<HashSet<PathBuf>> = config
        .exclude_globs
        .map(io_utils::glob_files)
        .transpose()
        .map_err(|e| TopCatError::config_error(format!("Failed to apply exclude globs: {e}")))?;

    // Filter files
    let include_extensions: Option<HashSet<String>> = config
        .include_extensions
        .map(|ext| ext.iter().cloned().collect());
    let exclude_extensions: Option<HashSet<String>> = config
        .exclude_extensions
        .map(|ext| ext.iter().cloned().collect());

    let filtered: HashSet<PathBuf> = all_files
        .into_iter()
        .filter(|file| {
            // Include glob filter
            if let Some(ref include) = include_globs
                && !include.contains(file)
            {
                return false;
            }

            // Exclude glob filter
            if let Some(ref exclude) = exclude_globs
                && exclude.contains(file)
            {
                return false;
            }

            // Extension filters
            if let Some(extension) = file.extension().and_then(|e| e.to_str()) {
                // Include extensions
                if let Some(ref include_ext) = include_extensions
                    && !include_ext.contains(extension)
                {
                    return false;
                }

                // Exclude extensions
                if let Some(ref exclude_ext) = exclude_extensions
                    && exclude_ext.contains(extension)
                {
                    return false;
                }
            } else if include_extensions.is_some() {
                // No extension but we have include filter - exclude this file
                return false;
            }

            true
        })
        .collect();

    Ok(filtered)
}

/// Discover files and perform SQL analysis without building a dependency graph
///
/// This function walks through input directories, performs SQL discovery on each file,
/// and returns FileNode objects with discovered dependencies. Unlike building a full
/// graph, this does not validate dependencies, check for cycles, or enforce layer
/// constraints - it simply discovers and returns what's in each file.
fn discover_files_without_graph(config: &config::Config) -> Result<Vec<FileNode>, TopCatError> {
    let mut file_nodes = Vec::new();

    // Create SQL analyzer if discovery is enabled
    let analyzer = if config.sql_discovery.enabled {
        Some(SqlAnalyzer::new(config.sql_discovery.clone()).map_err(|e| {
            TopCatError::config_error(format!("Failed to create SQL analyzer: {e}"))
        })?)
    } else {
        None
    };

    // Create layer mapper if configured
    let layer_mapper = if !config.auto_mapping.is_empty() {
        Some(LayerMapper::new(config.auto_mapping).map_err(|e| {
            TopCatError::config_error(format!("Failed to create layer mapper: {e}"))
        })?)
    } else {
        None
    };

    // Create soft dependency mapper if patterns are configured
    let soft_deps_mapper = if !config.sql_discovery.soft_deps_mappings.is_empty() {
        let mapper =
            SoftDepsMapper::new(&config.sql_discovery.soft_deps_mappings).map_err(|e| {
                TopCatError::config_error(format!("Failed to create soft dependency mapper: {e}"))
            })?;
        Some(Rc::new(mapper))
    } else {
        None
    };

    // Collect and filter files
    let files = collect_and_filter_files(config)?;

    for file_path in files {
        // Parse file headers using FileNode::from_file
        let mut file_node = FileNode::from_file(
            &config.comment_str,
            &file_path,
            &config.layers,
            &config.fallback_layer,
            layer_mapper.as_ref(),
        )
        .map_err(|e| {
            // Convert FileNodeError to TopCatError
            match e {
                FileNodeError::FileOpen(p, err) => TopCatError::config_error(format!(
                    "Failed to open file {}: {}",
                    p.display(),
                    err
                )),
                FileNodeError::NoNameDefined(p) => {
                    TopCatError::config_error(format!("No name defined in file {}", p.display()))
                }
                FileNodeError::TooManyNames(p, names) => TopCatError::config_error(format!(
                    "File {} has multiple names: {:?}",
                    p.display(),
                    names
                )),
                FileNodeError::InvalidLayer(p, layer) => TopCatError::config_error(format!(
                    "Invalid layer '{}' in file {}",
                    layer,
                    p.display()
                )),
            }
        })?;

        // Perform SQL discovery if enabled
        if let Some(ref analyzer) = analyzer {
            let content = std::fs::read_to_string(&file_path).map_err(|e| {
                TopCatError::config_error(format!(
                    "Failed to read file {}: {}",
                    file_path.display(),
                    e
                ))
            })?;

            let analysis_result = analyzer.analyze(&content);

            // Update node name if discovered (and not manual)
            if let Some(discovered_name) = analysis_result.node_name
                && !file_node.manual
            {
                file_node.name = discovered_name.clone();
                file_node.name_source = NameSource::Discovered;
                // Update schema from discovered name
                file_node.schema = FileNode::extract_schema(&discovered_name);
            }

            // Re-apply auto-mapping when using discovery-only strategy
            // This ensures layers are updated based on discovered/current name, not stale headers
            // We do this outside the node name update block so it always runs, even if the
            // node name didn't change (e.g., header name matches discovered name)
            if config.sql_discovery.merge_strategy == MergeStrategy::DiscoveryOnly
                && !file_node.manual
                && let Some(ref mapper) = layer_mapper
                && let Some(mapped_layer) = mapper.map_node_to_layer(&file_node.name)
            {
                file_node.layer = mapped_layer.clone();
                file_node.layer_is_fallback = mapped_layer.eq(&config.fallback_layer);
            }

            // Clean up discovered dependencies: remove self-references and subobjects
            let mut discovered_deps = analysis_result.dependencies;
            discovered_deps.remove(&file_node.name);
            for subobj in &analysis_result.subobjects {
                discovered_deps.remove(subobj);
            }

            // Store discovered dependencies (after cleanup)
            // IMPORTANT: Always set this, even if empty, so merge_dependencies can replace old deps
            file_node.discovered_deps = Some(discovered_deps);

            // Set implicit flag
            file_node.implicit = analysis_result.has_implicit;

            // Merge discovered dependencies with header dependencies based on strategy
            file_node.merge_dependencies(config.sql_discovery.merge_strategy);
        }

        // Assign soft dependency mapper if configured and apply the mappings
        if let Some(ref mapper) = soft_deps_mapper {
            file_node.soft_deps_mapper = Some(Rc::clone(mapper));
            // Apply the mappings to move matching deps from requires to exists
            file_node.apply_soft_deps_mappings();
        }

        file_nodes.push(file_node);
    }

    Ok(file_nodes)
}

/// Preview what would be updated without actually modifying files
fn preview_header_updates(
    file_nodes: &[topcat::file_node::FileNode],
    comment_str: &str,
    rename_files: bool,
    default_extension: &str,
    logger: &Logger,
) -> Result<(), TopCatError> {
    for file_node in file_nodes {
        // Only preview if we have discovered dependencies
        if file_node.discovered_deps.is_none() {
            continue;
        }

        let header = header_generator::generate_header(file_node, comment_str);

        logger.info(&format!("\n{}", "=".repeat(60)));
        logger.info(&format!("File: {}", file_node.path.display()));

        if rename_files
            && file_node.needs_rename(default_extension)
            && let Some(new_name) = file_node.suggested_filename(default_extension)
        {
            logger.info(&format!("  → Would rename to: {new_name}"));
        }

        logger.info(&format!("\nProposed header:\n{header}"));
    }

    logger.info(&format!("\n{}", "=".repeat(60)));
    logger.info("To apply these changes, run with --mode execute");

    Ok(())
}
