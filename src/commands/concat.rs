//! Concatenation command for combining files in topological order.
//!
//! This module implements the `concat` subcommand which reads files with
//! dependency metadata, builds a directed acyclic graph (DAG), performs
//! topological sorting, and concatenates the files in dependency order.
//!
//! # Examples
//!
//! ```bash
//! # Basic concatenation
//! topcat concat -i sql/ -e sql output.sql
//!
//! # With custom formatting
//! topcat concat -i sql/ -e sql -c "-- " -s "----" output.sql
//!
//! # With schema filtering
//! topcat concat -i sql/ -e sql --schema auth auth-migrations.sql
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;

use topcat::{
    cli::{FilterArgs, FormattingArgs, GlobalArgs, GraphInputArgs},
    config,
    exceptions::TopCatError,
    file_dag::TCGraph,
    fs,
    logging::{Logger, init_logging},
    output,
    settings::Settings,
};

/// Command-line arguments for the concat subcommand.
///
/// The concat command uses:
/// - GlobalArgs: config, verbose, quiet
/// - GraphInputArgs: input directories, file filtering, layers
/// - FilterArgs: schema filtering, node filtering
/// - FormattingArgs: comment string, file separator, file suffix
#[derive(Debug, Args, Clone)]
pub struct ConcatArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(flatten)]
    pub input: GraphInputArgs,

    #[command(flatten)]
    pub filter: FilterArgs,

    #[command(flatten)]
    pub formatting: FormattingArgs,

    /// Output file path (positional argument)
    #[arg(value_name = "OUTPUT")]
    output: PathBuf,
}

impl ConcatArgs {
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Load settings from config files and environment variables
        let config_path = self.global.config_path();
        let mut settings = Settings::load(config_path)
            .map_err(|e| TopCatError::ConfigError(format!("Failed to load configuration: {e}")))?;

        // Apply CLI overrides
        self.global.apply_to_settings(&mut settings);
        self.input.apply_to_settings(&mut settings);
        self.filter.apply_to_settings(&mut settings);
        self.formatting.apply_to_settings(&mut settings);

        // Set output from positional argument
        settings.output = Some(self.output.clone());

        // Validate settings
        settings.validate().map_err(TopCatError::ConfigError)?;

        // Ensure required fields are set
        if settings.input_dirs.is_empty() {
            return Err(TopCatError::ConfigError(
                "At least one input directory must be specified via -i/--input-dirs or config file"
                    .to_string(),
            ));
        }

        // Initialize logging
        let quiet = settings.behavior.quiet;
        let verbose = settings.behavior.verbose;
        init_logging(verbose, quiet);
        let logger = Logger::new(quiet, verbose);

        // Create Config directly from Settings
        let fallback_layer = settings.layers.fallback.clone();

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
            output: self.output.clone(),
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
