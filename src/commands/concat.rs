use std::collections::HashMap;

use clap::Args;

use topcat::{
    cli::CommonArgs,
    config,
    exceptions::TopCatError,
    file_dag::TCGraph,
    fs,
    logging::{Logger, init_logging},
    output,
    settings::Settings,
};

/// Concatenate files in topological order based on dependencies
#[derive(Debug, Args, Clone)]
pub struct ConcatArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    // Command-specific arguments (not in CommonArgs)
    // None for concat - all args are shared
}

impl ConcatArgs {
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

        if settings.output.is_none() {
            return Err(TopCatError::ConfigError(
                "Output file must be specified via -o/--output or config file".to_string(),
            ));
        }

        // Initialize logging
        let quiet = settings.behavior.quiet;
        let verbose = settings.behavior.verbose;
        init_logging(verbose, quiet);
        let logger = Logger::new(quiet, verbose);

        // Create Config directly from Settings
        let fallback_layer = settings.layers.fallback.clone().ok_or_else(|| {
            TopCatError::ConfigError("Fallback layer must be specified".to_string())
        })?;

        let output = settings
            .output
            .clone()
            .ok_or_else(|| TopCatError::ConfigError("Output path must be specified".to_string()))?;

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
            output,
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

        // Build the dependency graph
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

        // Warn if header update flags are used with concat
        if self.common.update_headers.is_some()
            || self.common.generate_headers_dir.is_some()
            || self.common.rename_files == Some(true)
        {
            logger.error("Header update flags (--update-headers, --generate-headers, --rename-files) are no longer supported by the concat command.");
            logger.error(
                "Please use the 'topcat update' command to update file headers and rename files.",
            );
            logger.error(
                "Example: topcat update -i sql/ -e sql --enable-sql-discovery --update-headers",
            );
            return Err(TopCatError::ConfigError(
                "Use 'topcat update' command for header updates instead of 'topcat concat'"
                    .to_string(),
            ));
        }

        // Generate output
        let result = output::generate(filedag, config, &mut fs::RealFileSystem);

        match result {
            Ok(()) => {
                logger.success("Generation Successful!");
            }
            Err(e) => {
                let mut map = HashMap::new();
                map.insert(1, e);
                logger.error(&format!("Initialization Failure:\n{map:#?}\n\nExiting."));
                std::process::exit(1);
            }
        }

        Ok(())
    }
}
