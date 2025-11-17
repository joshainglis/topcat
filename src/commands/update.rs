use clap::Args;

use topcat::{
    cli::CommonArgs,
    config,
    exceptions::TopCatError,
    file_dag::TCGraph,
    header_generator,
    logging::{Logger, init_logging},
    settings::Settings,
    sql_config,
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
        let fallback_layer = settings.layers.fallback.clone().ok_or_else(|| {
            TopCatError::ConfigError("Fallback layer must be specified".to_string())
        })?;

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
            sql_discovery: settings.sql_discovery.clone(),
            header_update_mode: settings.header_update_mode,
            header_output_dir: settings.header_output_dir.clone(),
        };

        // Build the dependency graph (this triggers SQL discovery if enabled)
        logger.info("Building dependency graph...");
        let mut filedag = TCGraph::new(&config);
        let res = filedag.build_graph();
        match res {
            Ok(_) => {
                logger.info("Graph built successfully!");
            }
            Err(e) => {
                logger.error(&format!("Error Encountered:\n{e}\n\nExiting."));
                std::process::exit(1);
            }
        }

        if settings.behavior.verbose {
            for layer in &settings.layers.names {
                logger.debug(&format!(
                    "{} Graph: {:#?}",
                    layer,
                    filedag.graph_as_dot(layer)?
                ));
            }
        }

        // Update headers
        if settings.behavior.dry_run {
            logger.info("Previewing header updates (dry-run mode)...");
        } else {
            logger.info("Updating file headers...");
        }

        let file_nodes: Vec<_> = filedag.get_all_nodes();

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

        if rename_files && file_node.needs_rename(default_extension) {
            if let Some(new_name) = file_node.suggested_filename(default_extension) {
                logger.info(&format!("  → Would rename to: {new_name}"));
            }
        }

        logger.info(&format!("\nProposed header:\n{header}"));
    }

    logger.info(&format!("\n{}", "=".repeat(60)));
    logger.info("To apply these changes, run without --dry-run");

    Ok(())
}
