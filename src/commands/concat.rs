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

use std::path::PathBuf;

use clap::Args;

use topcat::{
    cli::{FilterArgs, FormattingArgs, GlobalArgs, GraphInputArgs},
    config,
    exceptions::TopCatError,
    file_dag::TCGraph,
    fs, output,
};

use super::common as cmd_common;

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
        // Load settings and apply CLI overrides
        let settings =
            cmd_common::load_settings_with_overrides(self.global.config_path(), |settings| {
                self.global.apply_to_settings(settings);
                self.input.apply_to_settings(settings);
                self.filter.apply_to_settings(settings);
                self.formatting.apply_to_settings(settings);
                settings.output = Some(self.output.clone());
            })?;

        // Validate settings
        cmd_common::validate_settings(&settings, true)?;

        // Initialize logging
        let logger = cmd_common::init_logger_from_settings(&settings);

        // Create Config directly from Settings
        let fallback_layer = settings.layers.fallback.clone();

        // Create Config struct with borrowed slices from Settings
        let config = config::Config {
            input_dirs: settings.input_dirs.clone(),
            include_globs: (!settings.filters.include_globs.is_empty())
                .then_some(settings.filters.include_globs.as_slice()),
            exclude_globs: (!settings.filters.exclude_globs.is_empty())
                .then_some(settings.filters.exclude_globs.as_slice()),
            include_extensions: (!settings.filters.include_extensions.is_empty())
                .then_some(settings.filters.include_extensions.as_slice()),
            exclude_extensions: (!settings.filters.exclude_extensions.is_empty())
                .then_some(settings.filters.exclude_extensions.as_slice()),
            output: self.output.clone(),
            comment_str: settings.formatting.comment_str.clone(),
            file_separator_str: settings.formatting.file_separator_str.clone(),
            file_end_str: settings.formatting.file_end_str.clone(),
            verbose: settings.behavior.verbose,
            dry_run: settings.behavior.dry_run,
            include_node_prefixes: (!settings.node_filtering.include_prefixes.is_empty())
                .then_some(settings.node_filtering.include_prefixes.as_slice()),
            exclude_node_prefixes: (!settings.node_filtering.exclude_prefixes.is_empty())
                .then_some(settings.node_filtering.exclude_prefixes.as_slice()),
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
        filedag.build_graph()?;
        logger.info("Graph built successfully!");

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
        output::generate(filedag, config, &mut fs::RealFileSystem)?;
        logger.success("Generation Successful!");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concat_execute_returns_error_for_invalid_setup() {
        let args = ConcatArgs {
            global: GlobalArgs::default(),
            input: GraphInputArgs::default(),
            filter: FilterArgs::default(),
            formatting: FormattingArgs::default(),
            output: PathBuf::from("out.sql"),
        };

        let result = args.execute();
        assert!(matches!(result, Err(TopCatError::ConfigError(_))));
    }
}
