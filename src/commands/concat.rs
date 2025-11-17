use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;
use env_logger::Builder;
use log::{LevelFilter, error, info};

use topcat::{
    config, exceptions::TopCatError, file_dag::TCGraph, fs, header_generator, output, sql_config,
};

use super::common;

/// Concatenate files in topological order based on dependencies
#[derive(Debug, Args)]
pub struct ConcatArgs {
    #[arg(
        short = 'i',
        long = "input-dirs",
        help = "Paths to directories containing files to be concatenated",
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
        help = "Only include files matching glob pattern. Must be relative to the working directory, not the input directories. eg 'src/**/*.rs'",
        value_name = "PATTERN"
    )]
    include_globs: Option<Vec<String>>,

    #[arg(
        short = 'G',
        long = "exclude-glob",
        help = "Exclude files matching given glob pattern. Must be relative to the working directory, not the input directories. eg 'src/**/*.rs'",
        value_name = "PATTERN"
    )]
    exclude_globs: Option<Vec<String>>,

    #[arg(
        short = 'o',
        long = "output-file",
        help = "Path to generate combined output file",
        value_name = "FILE"
    )]
    output: PathBuf,

    #[arg(
        short = 'c',
        long = "comment-prefix",
        help = "The string used to denote a comment. eg '--'",
        default_value = "--"
    )]
    comment_str: String,

    #[arg(
        short = 's',
        long = "file-separator",
        help = "Add this between each concatenated file in the output. eg '---'",
        default_value = "------------------------------------------------------------------------------------------------------------------------"
    )]
    file_separator_str: String,

    #[arg(
        short = 'a',
        long = "file-suffix",
        help = "Add this string to the end of files if it does not exist. eg ';'",
        default_value = ";"
    )]
    ensure_each_file_ends_with_str: String,

    #[arg(long = "include-hidden", help = "Include hidden files and directories")]
    include_hidden_files_and_directories: bool,

    #[arg(short = 'v', long = "verbose", help = "Print debug information")]
    verbose: bool,

    #[arg(
        long = "include-prefix",
        help = "Only include nodes with the given prefixes in the output",
        value_name = "PREFIXES"
    )]
    include_node_prefixes: Option<Vec<String>>,

    #[arg(
        long = "exclude-prefix",
        help = "Exclude nodes with the given prefixes from the output",
        value_name = "PREFIXES"
    )]
    exclude_node_prefixes: Option<Vec<String>>,

    #[arg(
        long = "subdir-filter",
        help = "Only include files from this subdirectory and their dependencies",
        value_name = "SUBDIR"
    )]
    subdir_filter: Option<PathBuf>,

    #[arg(
        short = 'd',
        long = "dry-run",
        help = "Only print the output, do not write to file"
    )]
    dry_run: bool,

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

    // SQL Discovery Options
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
        help = "Regex pattern for matching schema names (e.g., '(?:schema1|schema2)_\\w+')",
        value_name = "PATTERN"
    )]
    schema_pattern: Option<String>,

    #[arg(
        long = "merge-strategy",
        help = "How to merge discovered and manual dependencies [header-only|discovery-only|union|header-with-fallback|validate]",
        value_name = "STRATEGY",
        default_value = "discovery-only"
    )]
    merge_strategy: String,

    #[arg(
        long = "update-headers",
        help = "Update files in-place with discovered dependencies"
    )]
    update_headers: bool,

    #[arg(
        long = "generate-headers",
        help = "Generate files with updated headers in the specified directory",
        value_name = "DIR"
    )]
    generate_headers_dir: Option<PathBuf>,
}

impl ConcatArgs {
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Initialize logging based on verbosity
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

        // Load SQL discovery configuration early
        let sql_discovery = common::load_sql_discovery_config(
            &self.sql_config_file,
            self.enable_sql_discovery,
            &self.schema_pattern,
            &self.merge_strategy,
        )?;

        // Load layers from config file + CLI
        let (layers, fallback_layer) = common::parse_and_validate_layers(
            &self.sql_config_file,
            &self.layers,
            &self.fallback_layer,
        )?;

        // Merge file filters from config file + CLI
        let (include_globs, exclude_globs, include_exts, exclude_exts, include_hidden) =
            common::merge_file_filters(
                &self.sql_config_file,
                self.include_globs.clone(),
                self.exclude_globs.clone(),
                self.include_file_extensions.clone(),
                self.exclude_file_extensions.clone(),
                self.include_hidden_files_and_directories,
            );

        // Determine header update mode
        let header_update_mode = if self.update_headers {
            sql_config::HeaderUpdateMode::InPlace
        } else if self.generate_headers_dir.is_some() {
            sql_config::HeaderUpdateMode::Generate
        } else {
            sql_config::HeaderUpdateMode::Never
        };

        let config = config::Config {
            input_dirs: self.input_dirs.clone(),
            include_extensions: include_exts.as_deref(),
            exclude_extensions: exclude_exts.as_deref(),
            include_globs: include_globs.as_deref(),
            exclude_globs: exclude_globs.as_deref(),
            output: self.output.clone(),
            comment_str: self.comment_str.clone(),
            file_separator_str: self.file_separator_str.clone(),
            file_end_str: self.ensure_each_file_ends_with_str.clone(),
            include_hidden,
            verbose: self.verbose,
            include_node_prefixes: self.include_node_prefixes.as_deref(),
            exclude_node_prefixes: self.exclude_node_prefixes.as_deref(),
            dry_run: self.dry_run,
            subdir_filter: self.subdir_filter.clone(),
            layers,
            fallback_layer,
            sql_discovery,
            header_update_mode,
            header_output_dir: self.generate_headers_dir.clone(),
        };

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

        if config.verbose {
            for layer in &config.layers {
                println!("{} Graph: {:#?}", layer, filedag.graph_as_dot(layer)?);
            }
        }

        // Update headers if requested
        if config.header_update_mode != sql_config::HeaderUpdateMode::Never {
            info!("Updating file headers...");
            let file_nodes: Vec<_> = filedag.get_all_nodes();
            header_generator::update_headers(
                &file_nodes,
                &config.comment_str,
                config.header_update_mode,
                config.header_output_dir.as_deref(),
            )?;
        }

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
}
