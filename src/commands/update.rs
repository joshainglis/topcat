use clap::Args;
use std::collections::HashSet;
use std::path::PathBuf;

use topcat::{
    cli::CommonArgs,
    config,
    exceptions::{FileNodeError, TopCatError},
    file_node::{FileNode, NameSource},
    header_generator, io_utils,
    layer_mapper::LayerMapper,
    logging::{Logger, init_logging},
    settings::Settings,
    sql_config,
    sql_parser::SqlAnalyzer,
};

/// Update file headers with discovered dependencies and optionally rename files
///
/// This command analyzes SQL files (when --enable-sql-discovery is set), discovers
/// dependencies, and updates file headers accordingly. It can also rename files based
/// on discovered node names.
///
/// By default, files are updated in-place. Use --dry-run to preview changes without
/// modifying files, or --generate-headers to write updated files to a separate directory.
#[derive(Debug, Args, Clone)]
pub struct UpdateArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    // Command-specific arguments (not in CommonArgs)
    // None for update - all args are shared
}

impl UpdateArgs {
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Load settings from config files and environment variables
        let config_path = self.common.config_path();
        let mut settings = Settings::load(config_path)
            .map_err(|e| TopCatError::ConfigError(format!("Failed to load configuration: {e}")))?;

        // Apply CLI overrides
        self.common.apply_to_settings(&mut settings);

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

        // If rename_files is true but header update is disabled, that's an error
        if settings.rename_files
            && settings.header_update_mode == sql_config::HeaderUpdateMode::Never
        {
            return Err(TopCatError::ConfigError(
                "Cannot rename files without header updates. Use --update-headers or --generate-headers".to_string(),
            ));
        }

        // Initialize logging
        let quiet = settings.behavior.quiet;
        let verbose = settings.behavior.verbose;
        init_logging(verbose, quiet);
        let logger = Logger::new(quiet, verbose);

        // Show what we're doing
        if settings.behavior.dry_run {
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
            dry_run: settings.behavior.dry_run,
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
        if settings.behavior.dry_run {
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
            logger.info("No files with discovered dependencies found. Did you enable --enable-sql-discovery?");
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
        if settings.behavior.dry_run {
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
        .map(|patterns| io_utils::glob_files(patterns))
        .transpose()
        .map_err(|e| TopCatError::config_error(format!("Failed to apply include globs: {e}")))?;

    let exclude_globs: Option<HashSet<PathBuf>> = config
        .exclude_globs
        .map(|patterns| io_utils::glob_files(patterns))
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
            if let Some(ref include) = include_globs {
                if !include.contains(file) {
                    return false;
                }
            }

            // Exclude glob filter
            if let Some(ref exclude) = exclude_globs {
                if exclude.contains(file) {
                    return false;
                }
            }

            // Extension filters
            if let Some(extension) = file.extension().and_then(|e| e.to_str()) {
                // Include extensions
                if let Some(ref include_ext) = include_extensions {
                    if !include_ext.contains(extension) {
                        return false;
                    }
                }

                // Exclude extensions
                if let Some(ref exclude_ext) = exclude_extensions {
                    if exclude_ext.contains(extension) {
                        return false;
                    }
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

            // Store discovered dependencies
            if !analysis_result.dependencies.is_empty() {
                file_node.discovered_deps = Some(analysis_result.dependencies);
            }

            // Update node name if discovered (and not manual)
            if let Some(discovered_name) = analysis_result.node_name {
                if !file_node.manual {
                    file_node.name = discovered_name.clone();
                    file_node.name_source = NameSource::Discovered;
                    // Update schema from discovered name
                    file_node.schema = FileNode::extract_schema(&discovered_name);
                }
            }

            // Set implicit flag
            file_node.implicit = analysis_result.has_implicit;

            // Merge discovered dependencies with header dependencies based on strategy
            file_node.merge_dependencies(config.sql_discovery.merge_strategy);
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
    logger.info("To apply these changes, run without --dry-run");

    Ok(())
}
