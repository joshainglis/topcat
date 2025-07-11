use std::collections::HashMap;
use std::path::PathBuf;
use std::string::ToString;

use env_logger::Builder;
use log::{error, info, LevelFilter};
use structopt::StructOpt;

use file_dag::TCGraph;

use crate::exceptions::TopCatError;

mod config;
mod exceptions;
mod file_dag;
mod file_node;
mod fs;
mod io_utils;
mod output;
mod stable_topo;

#[derive(Debug, StructOpt)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(
        short = "i",
        long = "input-dirs",
        help = "Paths to directories containing files to be concatenated",
        value_name = "DIRS"
    )]
    input_dirs: Vec<PathBuf>,

    #[structopt(
        short = "e",
        long = "include-exts",
        help = "Only include files with the given file extensions",
        value_name = "EXTENSIONS"
    )]
    include_file_extensions: Option<Vec<String>>,

    #[structopt(
        short = "E",
        long = "exclude-exts",
        help = "Exclude files with the given file extensions",
        value_name = "EXTENSIONS"
    )]
    exclude_file_extensions: Option<Vec<String>>,

    #[structopt(
        short = "g",
        long = "include-glob",
        help = "Only include files matching glob pattern. Must be relative to the working directory, not the input directories. eg 'src/**/*.rs'",
        value_name = "PATTERN"
    )]
    include_globs: Option<Vec<String>>,

    #[structopt(
        short = "G",
        long = "exclude-glob",
        help = "Exclude files matching given glob pattern. Must be relative to the working directory, not the input directories. eg 'src/**/*.rs'",
        value_name = "PATTERN"
    )]
    exclude_globs: Option<Vec<String>>,

    #[structopt(
        short = "o",
        long = "output-file",
        help = "Path to generate combined output file",
        value_name = "FILE"
    )]
    output: PathBuf,

    #[structopt(
        short = "c",
        long = "comment-prefix",
        help = "The string used to denote a comment. eg '--'",
        default_value = "--"
    )]
    comment_str: String,

    #[structopt(
        short = "s",
        long = "file-separator",
        help = "Add this between each concatenated file in the output. eg '---'",
        default_value = "------------------------------------------------------------------------------------------------------------------------"
    )]
    file_separator_str: String,

    #[structopt(
        short = "a",
        long = "file-suffix",
        help = "Add this string to the end of files if it does not exist. eg ';'",
        default_value = ";"
    )]
    ensure_each_file_ends_with_str: String,

    #[structopt(long = "include-hidden", help = "Include hidden files and directories")]
    include_hidden_files_and_directories: bool,

    #[structopt(short = "v", long = "verbose", help = "Print debug information")]
    verbose: bool,

    #[structopt(
        long = "include-prefix",
        help = "Only include nodes with the given prefixes in the output",
        value_name = "PREFIXES"
    )]
    include_node_prefixes: Option<Vec<String>>,

    #[structopt(
        long = "exclude-prefix",
        help = "Exclude nodes with the given prefixes from the output",
        value_name = "PREFIXES"
    )]
    exclude_node_prefixes: Option<Vec<String>>,

    #[structopt(
        long = "subdir-filter",
        help = "Only include files from this subdirectory and their dependencies",
        value_name = "SUBDIR"
    )]
    subdir_filter: Option<PathBuf>,

    #[structopt(
        short = "d",
        long = "dry-run",
        help = "Only print the output, do not write to file"
    )]
    dry_run: bool,

    #[structopt(
        long = "layers",
        help = "Comma-separated list of layer names in order",
        value_name = "LAYERS"
    )]
    layers: Option<String>,

    #[structopt(
        long = "fallback-layer",
        help = "Default layer for nodes without explicit layer declaration",
        value_name = "LAYER"
    )]
    fallback_layer: Option<String>,
}
fn main() -> Result<(), TopCatError> {
    let opt = Opt::from_args();
    if opt.verbose {
        Builder::new().filter(None, LevelFilter::Debug).init();
    } else {
        Builder::new().filter(None, LevelFilter::Info).init();
    }

    // Parse layers from CLI or use defaults
    let layers = if let Some(layers_str) = opt.layers {
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

    // Set fallback layer
    let fallback_layer = opt.fallback_layer.unwrap_or_else(|| "normal".to_string());

    // Validate that fallback layer exists in layers
    if !layers.contains(&fallback_layer) {
        eprintln!(
            "Error: Fallback layer '{}' is not in the layers list: {:?}",
            fallback_layer, layers
        );
        std::process::exit(1);
    }

    let config = config::Config {
        input_dirs: opt.input_dirs,
        include_extensions: opt.include_file_extensions.as_deref(),
        exclude_extensions: opt.exclude_file_extensions.as_deref(),
        include_globs: opt.include_globs.as_deref(),
        exclude_globs: opt.exclude_globs.as_deref(),
        output: opt.output,
        comment_str: opt.comment_str,
        file_separator_str: opt.file_separator_str,
        file_end_str: opt.ensure_each_file_ends_with_str,
        include_hidden: opt.include_hidden_files_and_directories,
        verbose: opt.verbose,
        include_node_prefixes: opt.include_node_prefixes.as_deref(),
        exclude_node_prefixes: opt.exclude_node_prefixes.as_deref(),
        dry_run: opt.dry_run,
        subdir_filter: opt.subdir_filter,
        layers,
        fallback_layer,
    };

    let mut filedag = TCGraph::new(&config);
    let res = filedag.build_graph();
    match res {
        Ok(_) => {
            info!("Graph built successfully!");
        }
        Err(e) => {
            eprintln!("Error Encountered:\n{}\n\nExiting.", e);
            std::process::exit(1);
        }
    }

    if config.verbose {
        for layer in &config.layers {
            println!("{} Graph: {:#?}", layer, filedag.graph_as_dot(layer)?);
        }
    }

    let result = output::generate(filedag, config, &mut fs::RealFileSystem);

    match result {
        Ok(()) => {
            info!("Generation Successful!");
        }
        Err(e) => {
            let mut map = HashMap::new();
            map.insert(1, e);
            error!("Initialization Failure:\n{:#?}\n\nExiting.", map);
            std::process::exit(1);
        }
    }

    Ok(())
}
