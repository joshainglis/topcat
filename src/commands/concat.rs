use std::collections::HashMap;

use clap::Args;
use env_logger::Builder;
use log::{LevelFilter, error, info};

use topcat::{
    cli::CommonArgs, config, exceptions::TopCatError, file_dag::TCGraph, fs, header_generator,
    output, settings::Settings, sql_config,
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
        let log_level = if settings.behavior.verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        };

        Builder::new().filter(None, log_level).try_init().ok();

        // Convert Settings to ConfigBuilder (temporary bridge until TCGraph is refactored)
        let config_builder = Self::settings_to_config_builder(&settings)?;
        let config = config_builder.build()?;

        // Build the dependency graph
        let mut filedag = TCGraph::new(&config);
        let res = filedag.build_graph();
        match res {
            Ok(_) => {
                info!("Graph built successfully!");
            }
            Err(e) => {
                eprintln!("Error Encountered:\n{e}\n\nExiting.");
                std::process::exit(1);
            }
        }

        if settings.behavior.verbose {
            for layer in &settings.layers.names {
                println!("{} Graph: {:#?}", layer, filedag.graph_as_dot(layer)?);
            }
        }

        // Update headers if requested
        if settings.header_update_mode != sql_config::HeaderUpdateMode::Never {
            info!("Updating file headers...");
            let file_nodes: Vec<_> = filedag.get_all_nodes();
            header_generator::update_headers(
                &file_nodes,
                &settings.formatting.comment_str,
                settings.header_update_mode,
                settings.header_output_dir.as_deref(),
            )?;
        }

        // Generate output
        let result = output::generate(filedag, config, &mut fs::RealFileSystem);

        match result {
            Ok(()) => {
                info!("Generation Successful!");
            }
            Err(e) => {
                let mut map = HashMap::new();
                map.insert(1, e);
                error!("Initialization Failure:\n{map:#?}\n\nExiting.");
                std::process::exit(1);
            }
        }

        Ok(())
    }

    /// Convert Settings to ConfigBuilder (temporary bridge until TCGraph is refactored)
    #[allow(deprecated)]
    fn settings_to_config_builder(
        settings: &Settings,
    ) -> Result<config::ConfigBuilder, TopCatError> {
        let fallback_layer = settings.layers.fallback.clone().ok_or_else(|| {
            TopCatError::ConfigError("Fallback layer must be specified".to_string())
        })?;

        let output = settings
            .output
            .clone()
            .ok_or_else(|| TopCatError::ConfigError("Output path must be specified".to_string()))?;

        // Since Config expects borrowed slices, we need to use ConfigBuilder
        let builder = config::Config::builder()
            .input_dirs(settings.input_dirs.clone())
            .output(output)
            .comment_str(settings.formatting.comment_str.clone())
            .file_separator_str(settings.formatting.file_separator_str.clone())
            .file_end_str(settings.formatting.file_end_str.clone())
            .verbose(settings.behavior.verbose)
            .dry_run(settings.behavior.dry_run)
            .include_hidden(settings.filters.include_hidden)
            .layers(settings.layers.names.clone())
            .fallback_layer(&fallback_layer)
            .sql_discovery(settings.sql_discovery.clone())
            .header_update_mode(settings.header_update_mode);

        let builder = if !settings.filters.include_globs.is_empty() {
            builder.include_globs(settings.filters.include_globs.clone())
        } else {
            builder
        };

        let builder = if !settings.filters.exclude_globs.is_empty() {
            builder.exclude_globs(settings.filters.exclude_globs.clone())
        } else {
            builder
        };

        let builder = if !settings.filters.include_extensions.is_empty() {
            builder.include_extensions(settings.filters.include_extensions.clone())
        } else {
            builder
        };

        let builder = if !settings.filters.exclude_extensions.is_empty() {
            builder.exclude_extensions(settings.filters.exclude_extensions.clone())
        } else {
            builder
        };

        let builder = if !settings.node_filtering.include_prefixes.is_empty() {
            builder.include_node_prefixes(settings.node_filtering.include_prefixes.clone())
        } else {
            builder
        };

        let builder = if !settings.node_filtering.exclude_prefixes.is_empty() {
            builder.exclude_node_prefixes(settings.node_filtering.exclude_prefixes.clone())
        } else {
            builder
        };

        let builder = if let Some(ref subdir) = settings.node_filtering.subdir_filter {
            builder.subdir_filter(subdir.clone())
        } else {
            builder
        };

        let builder = if let Some(ref dir) = settings.header_output_dir {
            builder.header_output_dir(dir.clone())
        } else {
            builder
        };

        Ok(builder)
    }
}
